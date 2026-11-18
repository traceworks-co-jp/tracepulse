use crate::notifier::{AlertNotification, NotificationSeverity};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlertKey {
    pub device_id: Option<i64>,
    pub interface_id: Option<i64>,
    pub alert_kind: String,
    pub severity: NotificationSeverity,
}

#[derive(Debug, Clone)]
pub struct AlertBatch {
    pub key: AlertKey,
    pub first_alert: AlertNotification,
    pub last_alert: AlertNotification,
    pub count: usize,
    pub first_seen: Instant,
    pub last_seen: Instant,
}

impl AlertBatch {
    pub fn suppressed_count(&self) -> usize {
        self.count.saturating_sub(1)
    }
}

#[derive(Debug, Clone)]
struct BufferedAlert {
    first_alert: AlertNotification,
    last_alert: AlertNotification,
    count: usize,
    first_seen: Instant,
    last_seen: Instant,
}

pub struct FlapGuard {
    window: Duration,
    buffered: HashMap<AlertKey, BufferedAlert>,
}

impl FlapGuard {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            buffered: HashMap::new(),
        }
    }

    pub fn ingest(&mut self, alert: AlertNotification) -> Vec<AlertBatch> {
        let now = Instant::now();
        if alert.is_immediate() {
            return vec![AlertBatch {
                key: alert.flap_key(),
                first_alert: alert.clone(),
                last_alert: alert,
                count: 1,
                first_seen: now,
                last_seen: now,
            }];
        }

        let key = alert.flap_key();
        self.buffered
            .entry(key)
            .and_modify(|buffered| {
                buffered.last_alert = alert.clone();
                buffered.last_seen = now;
                buffered.count += 1;
            })
            .or_insert_with(|| BufferedAlert {
                first_alert: alert.clone(),
                last_alert: alert,
                count: 1,
                first_seen: now,
                last_seen: now,
            });

        self.flush_due_at(now)
    }

    pub fn flush_due(&mut self) -> Vec<AlertBatch> {
        self.flush_due_at(Instant::now())
    }

    pub fn flush_all(&mut self) -> Vec<AlertBatch> {
        let keys: Vec<AlertKey> = self.buffered.keys().cloned().collect();
        self.remove_batches(keys)
    }

    fn flush_due_at(&mut self, now: Instant) -> Vec<AlertBatch> {
        let keys: Vec<AlertKey> = self
            .buffered
            .iter()
            .filter_map(|(key, buffered)| {
                if now.duration_since(buffered.first_seen) >= self.window {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();

        self.remove_batches(keys)
    }

    fn remove_batches(&mut self, keys: Vec<AlertKey>) -> Vec<AlertBatch> {
        keys.into_iter()
            .filter_map(|key| {
                self.buffered.remove(&key).map(|buffered| AlertBatch {
                    key,
                    first_alert: buffered.first_alert,
                    last_alert: buffered.last_alert,
                    count: buffered.count,
                    first_seen: buffered.first_seen,
                    last_seen: buffered.last_seen,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alert(severity: NotificationSeverity, message: &str) -> AlertNotification {
        AlertNotification {
            id: "alert-1".to_string(),
            device_id: Some(1),
            interface_id: Some(10),
            core_kind: None,
            device_name: "edge-1".to_string(),
            device_ip: "192.0.2.10".to_string(),
            interface_name: Some("ge-0/0/1".to_string()),
            alert_kind: "interface_spike".to_string(),
            severity,
            title: "Interface spike".to_string(),
            message: message.to_string(),
            observed_value: Some("1200000".to_string()),
            threshold: Some("1000000".to_string()),
            occurred_at: "2026-09-03T00:00:00Z".to_string(),
            recommended_actions: Vec::new(),
        }
    }

    #[test]
    fn critical_alert_bypasses_buffer() {
        let mut guard = FlapGuard::new(Duration::from_secs(10));

        let batches = guard.ingest(alert(NotificationSeverity::Critical, "critical"));

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].count, 1);
        assert!(guard.flush_all().is_empty());
    }

    #[test]
    fn warning_alerts_are_aggregated_until_flush() {
        let mut guard = FlapGuard::new(Duration::from_secs(10));

        assert!(
            guard
                .ingest(alert(NotificationSeverity::Warning, "first"))
                .is_empty()
        );
        assert!(
            guard
                .ingest(alert(NotificationSeverity::Warning, "second"))
                .is_empty()
        );
        let batches = guard.flush_all();

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].count, 2);
        assert_eq!(batches[0].suppressed_count(), 1);
        assert_eq!(batches[0].last_alert.message, "second");
    }

    #[test]
    fn warning_alerts_with_different_interface_ids_are_not_aggregated_together() {
        let mut guard = FlapGuard::new(Duration::from_secs(10));
        let mut first = alert(NotificationSeverity::Warning, "first");
        let mut second = alert(NotificationSeverity::Warning, "second");
        first.interface_id = Some(10);
        second.interface_id = Some(11);

        assert!(guard.ingest(first).is_empty());
        assert!(guard.ingest(second).is_empty());
        let mut batches = guard.flush_all();

        batches.sort_by_key(|batch| batch.key.interface_id);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].key.interface_id, Some(10));
        assert_eq!(batches[1].key.interface_id, Some(11));
    }
}
