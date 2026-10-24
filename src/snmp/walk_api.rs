use crate::snmp::SnmpVarBind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const DEFAULT_RELIABLE_WALK_MAX_VARBINDS: usize = 10_000;
pub const MIN_WALK_INTERVAL: Duration = Duration::from_millis(20);
const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkOptions {
    pub timeout: Duration,
    pub max_varbinds: usize,
    pub request_interval: Duration,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_varbinds: DEFAULT_RELIABLE_WALK_MAX_VARBINDS,
            request_interval: MIN_WALK_INTERVAL,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliableWalkStatus {
    Complete,
    Unsupported,
    Partial,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkReason {
    EndOfMib,
    OutsidePrefix,
    NonIncreasingOid,
    UnsupportedResponse(String),
    VarBindLimitReached,
    DeadlineExceeded,
    Cancelled,
    RequestTimeout,
    Transport(String),
    AgentError { status: u32, index: u32 },
    InvalidRequest(String),
    EmptyResponse,
    InvalidOid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReliableWalkOutcome {
    pub status: ReliableWalkStatus,
    pub varbinds: Vec<SnmpVarBind>,
    pub reason: Option<WalkReason>,
    pub count: usize,
    pub cursor: Option<Vec<u32>>,
}

#[derive(Debug)]
pub(crate) enum WalkStep {
    VarBind(SnmpVarBind),
    EndOfMib,
    Unsupported(String),
    Error(WalkReason),
}

pub(crate) fn walk_with<F>(
    prefix: &[u32],
    options: &WalkOptions,
    cancellation: &AtomicBool,
    mut get_next: F,
) -> ReliableWalkOutcome
where
    F: FnMut(&[u32], Duration) -> WalkStep,
{
    let started = Instant::now();
    let deadline = started.checked_add(options.timeout);
    let interval = options.request_interval.max(MIN_WALK_INTERVAL);
    let mut current = prefix.to_vec();
    let mut varbinds = Vec::new();
    let mut first_request = true;

    let finish = |status, reason, varbinds: Vec<SnmpVarBind>| ReliableWalkOutcome {
        count: varbinds.len(),
        cursor: varbinds.last().map(|value| value.oid.clone()),
        status,
        reason: Some(reason),
        varbinds,
    };

    if prefix.is_empty() {
        return finish(
            ReliableWalkStatus::Failed,
            WalkReason::InvalidRequest("SNMP walk prefix is empty".to_string()),
            varbinds,
        );
    }
    if options.max_varbinds == 0 {
        return finish(
            ReliableWalkStatus::Failed,
            WalkReason::InvalidRequest("SNMP walk maximum must be greater than zero".to_string()),
            varbinds,
        );
    }

    loop {
        if cancellation.load(Ordering::Acquire) {
            return finish(
                ReliableWalkStatus::Cancelled,
                WalkReason::Cancelled,
                varbinds,
            );
        }
        let now = Instant::now();
        let Some(deadline) = deadline else {
            return finish(
                if varbinds.is_empty() {
                    ReliableWalkStatus::Failed
                } else {
                    ReliableWalkStatus::Partial
                },
                WalkReason::DeadlineExceeded,
                varbinds,
            );
        };
        let remaining = deadline.saturating_duration_since(now);
        if remaining.is_zero() {
            return finish(
                if varbinds.is_empty() {
                    ReliableWalkStatus::Failed
                } else {
                    ReliableWalkStatus::Partial
                },
                WalkReason::DeadlineExceeded,
                varbinds,
            );
        }

        if !first_request {
            let wait_until = Instant::now() + interval.min(remaining);
            while Instant::now() < wait_until {
                if cancellation.load(Ordering::Acquire) {
                    return finish(
                        ReliableWalkStatus::Cancelled,
                        WalkReason::Cancelled,
                        varbinds,
                    );
                }
                if Instant::now() >= deadline {
                    return finish(
                        if varbinds.is_empty() {
                            ReliableWalkStatus::Failed
                        } else {
                            ReliableWalkStatus::Partial
                        },
                        WalkReason::DeadlineExceeded,
                        varbinds,
                    );
                }
                std::thread::sleep(
                    CANCELLATION_POLL_INTERVAL
                        .min(wait_until.saturating_duration_since(Instant::now())),
                );
            }
        }
        first_request = false;

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return finish(
                if varbinds.is_empty() {
                    ReliableWalkStatus::Failed
                } else {
                    ReliableWalkStatus::Partial
                },
                WalkReason::DeadlineExceeded,
                varbinds,
            );
        }

        match get_next(&current, remaining) {
            WalkStep::EndOfMib => {
                return finish(ReliableWalkStatus::Complete, WalkReason::EndOfMib, varbinds);
            }
            WalkStep::Unsupported(reason) => {
                return finish(
                    ReliableWalkStatus::Unsupported,
                    WalkReason::UnsupportedResponse(reason),
                    varbinds,
                );
            }
            WalkStep::Error(reason) => {
                let status = match reason {
                    WalkReason::RequestTimeout | WalkReason::DeadlineExceeded => {
                        if varbinds.is_empty() {
                            ReliableWalkStatus::Failed
                        } else {
                            ReliableWalkStatus::Partial
                        }
                    }
                    WalkReason::Cancelled => ReliableWalkStatus::Cancelled,
                    _ if varbinds.is_empty() => ReliableWalkStatus::Failed,
                    _ => ReliableWalkStatus::Partial,
                };
                return finish(status, reason, varbinds);
            }
            WalkStep::VarBind(varbind) => {
                if !varbind.oid.starts_with(prefix) {
                    return finish(
                        ReliableWalkStatus::Complete,
                        WalkReason::OutsidePrefix,
                        varbinds,
                    );
                }
                if varbind.oid <= current {
                    return finish(
                        if varbinds.is_empty() {
                            ReliableWalkStatus::Failed
                        } else {
                            ReliableWalkStatus::Partial
                        },
                        WalkReason::NonIncreasingOid,
                        varbinds,
                    );
                }
                current = varbind.oid.clone();
                varbinds.push(varbind);
                if varbinds.len() == options.max_varbinds {
                    return finish(
                        ReliableWalkStatus::Partial,
                        WalkReason::VarBindLimitReached,
                        varbinds,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snmp::SnmpValue;
    use std::sync::atomic::AtomicBool;

    fn options(timeout: Duration, max_varbinds: usize) -> WalkOptions {
        WalkOptions {
            timeout,
            max_varbinds,
            request_interval: MIN_WALK_INTERVAL,
        }
    }

    fn varbind(oid: &[u32]) -> SnmpVarBind {
        SnmpVarBind {
            oid: oid.to_vec(),
            value: SnmpValue::Integer(1),
        }
    }

    #[test]
    fn normal_end_is_complete() {
        let result = walk_with(
            &[1, 3, 6],
            &options(Duration::from_secs(1), 10),
            &AtomicBool::new(false),
            |current, _| {
                if current == [1, 3, 6] {
                    WalkStep::VarBind(varbind(&[1, 3, 6, 1]))
                } else {
                    WalkStep::EndOfMib
                }
            },
        );

        assert_eq!(result.status, ReliableWalkStatus::Complete);
        assert_eq!(result.reason, Some(WalkReason::EndOfMib));
        assert_eq!(result.count, 1);
        assert_eq!(result.cursor, Some(vec![1, 3, 6, 1]));
    }

    #[test]
    fn timeout_after_rows_is_partial() {
        let result = walk_with(
            &[1],
            &options(Duration::from_secs(1), 10),
            &AtomicBool::new(false),
            |current, _| {
                if current == [1] {
                    WalkStep::VarBind(varbind(&[1, 1]))
                } else {
                    WalkStep::Error(WalkReason::RequestTimeout)
                }
            },
        );

        assert_eq!(result.status, ReliableWalkStatus::Partial);
        assert_eq!(result.reason, Some(WalkReason::RequestTimeout));
        assert_eq!(result.count, 1);
    }

    #[test]
    fn outside_prefix_completes_even_when_oid_does_not_increase() {
        for after_row in [false, true] {
            let result = walk_with(
                &[1, 3, 6],
                &options(Duration::from_secs(1), 10),
                &AtomicBool::new(false),
                |current, _| {
                    if after_row && current == [1, 3, 6] {
                        WalkStep::VarBind(varbind(&[1, 3, 6, 1]))
                    } else {
                        WalkStep::VarBind(varbind(&[1, 3, 5]))
                    }
                },
            );

            assert_eq!(result.status, ReliableWalkStatus::Complete);
            assert_eq!(result.reason, Some(WalkReason::OutsidePrefix));
            assert_eq!(result.count, usize::from(after_row));
            assert_eq!(result.cursor, after_row.then(|| vec![1, 3, 6, 1]));
        }
    }

    #[test]
    fn repeated_oid_inside_prefix_is_not_complete() {
        for after_row in [false, true] {
            let mut calls = 0;
            let result = walk_with(
                &[1, 3, 6],
                &options(Duration::from_secs(1), 10),
                &AtomicBool::new(false),
                |current, _| {
                    calls += 1;
                    if after_row && calls == 1 {
                        WalkStep::VarBind(varbind(&[1, 3, 6, 1]))
                    } else {
                        WalkStep::VarBind(varbind(current))
                    }
                },
            );

            assert_eq!(
                result.status,
                if after_row {
                    ReliableWalkStatus::Partial
                } else {
                    ReliableWalkStatus::Failed
                }
            );
            assert_eq!(result.reason, Some(WalkReason::NonIncreasingOid));
            assert_eq!(result.count, usize::from(after_row));
        }
    }

    #[test]
    fn row_cap_is_partial() {
        let result = walk_with(
            &[1],
            &options(Duration::from_secs(1), 1),
            &AtomicBool::new(false),
            |_, _| WalkStep::VarBind(varbind(&[1, 1])),
        );

        assert_eq!(result.status, ReliableWalkStatus::Partial);
        assert_eq!(result.reason, Some(WalkReason::VarBindLimitReached));
        assert_eq!(result.count, 1);
    }

    #[test]
    fn cancellation_interrupts_inter_request_wait() {
        let cancellation = AtomicBool::new(false);
        let mut calls = 0;
        let result = walk_with(
            &[1],
            &options(Duration::from_secs(1), 10),
            &cancellation,
            |_, _| {
                calls += 1;
                cancellation.store(true, Ordering::Release);
                WalkStep::VarBind(varbind(&[1, 1]))
            },
        );

        assert_eq!(result.status, ReliableWalkStatus::Cancelled);
        assert_eq!(calls, 1);
        assert_eq!(result.count, 1);
    }

    #[test]
    fn deadline_expires_during_wait_as_partial() {
        let mut walk_options = options(Duration::from_millis(40), 10);
        walk_options.request_interval = Duration::from_millis(100);
        let result = walk_with(&[1], &walk_options, &AtomicBool::new(false), |_, _| {
            WalkStep::VarBind(varbind(&[1, 1]))
        });

        assert_eq!(result.status, ReliableWalkStatus::Partial);
        assert_eq!(result.reason, Some(WalkReason::DeadlineExceeded));
        assert_eq!(result.count, 1);
    }
}
