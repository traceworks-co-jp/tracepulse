use crate::alert::status::{AlertSeverity, DeviceStatus};

#[derive(Debug, Clone)]
pub struct AlertRules {
    pub error_rate_threshold: f64,
    pub spike_threshold: u64,
    pub warning_threshold: u32,
    pub critical_threshold: u32,
}

impl AlertRules {
    pub fn new(
        error_rate_threshold: f64,
        spike_threshold: u64,
        warning_threshold: u32,
        critical_threshold: u32,
    ) -> Self {
        Self {
            error_rate_threshold,
            spike_threshold,
            warning_threshold,
            critical_threshold,
        }
    }

    pub fn detect_spike(&self, previous: u64, current: u64) -> bool {
        current.saturating_sub(previous) >= self.spike_threshold
    }

    pub fn detect_error_rate(&self, error_count: u64, total_packets: u64) -> bool {
        let rate = if total_packets == 0 {
            0.0
        } else {
            (error_count as f64 / total_packets as f64) * 100.0
        };

        rate >= self.error_rate_threshold * 100.0
    }

    pub fn classify_health(&self, score: u32) -> DeviceStatus {
        if score >= self.warning_threshold {
            DeviceStatus::Healthy
        } else if score >= self.critical_threshold {
            DeviceStatus::Warning
        } else {
            DeviceStatus::Critical
        }
    }

    pub fn severity_for_status(&self, status: DeviceStatus) -> AlertSeverity {
        match status {
            DeviceStatus::Healthy => AlertSeverity::Info,
            DeviceStatus::Warning => AlertSeverity::Warning,
            DeviceStatus::Critical => AlertSeverity::Critical,
            DeviceStatus::Offline => AlertSeverity::Offline,
        }
    }
}
