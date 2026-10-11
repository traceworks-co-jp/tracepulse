use crate::alert::status::{AlertSeverity, DeviceStatus};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricThresholds {
    pub warning: f64,
    pub critical: f64,
}

impl Default for MetricThresholds {
    fn default() -> Self {
        Self {
            warning: 80.0,
            critical: 90.0,
        }
    }
}

impl MetricThresholds {
    pub fn validate(&self) -> bool {
        self.warning.is_finite()
            && self.critical.is_finite()
            && self.warning > 0.0
            && self.warning < self.critical
            && self.critical <= 100.0
    }

    pub fn severity(&self, value: f64) -> Option<AlertSeverity> {
        if !value.is_finite() {
            None
        } else if value >= self.critical {
            Some(AlertSeverity::Critical)
        } else if value >= self.warning {
            Some(AlertSeverity::Warning)
        } else {
            None
        }
    }
}

pub fn interface_error_rate(
    previous: &crate::db::models::InterfaceSample,
    current: &crate::db::models::InterfaceSample,
) -> Option<f64> {
    let errors = current
        .in_errors
        .checked_sub(previous.in_errors)?
        .saturating_add(current.out_errors.checked_sub(previous.out_errors)?);
    let packets = current
        .in_packets
        .checked_sub(previous.in_packets)?
        .saturating_add(current.out_packets.checked_sub(previous.out_packets)?);
    let total = packets.saturating_add(errors);
    (total > 0).then(|| (errors as f64 / total as f64) * 100.0)
}

#[derive(Default)]
pub struct MetricAlertTracker {
    active: std::collections::HashMap<(i64, Option<i64>, String), AlertSeverity>,
}

impl MetricAlertTracker {
    pub fn transitions(
        &mut self,
        device_id: i64,
        events: Vec<crate::alert::event::AlertEvent>,
    ) -> Vec<crate::alert::event::AlertEvent> {
        let mut next = std::collections::HashMap::new();
        let mut changed = Vec::new();
        for event in events {
            let key = (
                device_id,
                event.interface_id,
                event.kind.as_str().to_string(),
            );
            if self.active.get(&key) != Some(&event.severity) {
                changed.push(event.clone());
            }
            next.insert(key, event.severity);
        }
        self.active.retain(|key, _| key.0 != device_id);
        self.active.extend(next);
        changed
    }
}

impl crate::config::AlertConfig {
    pub fn metric_alert(
        &self,
        device: &crate::device::types::DeviceConfig,
        metric: &str,
        value: f64,
        thresholds: &MetricThresholds,
        interface: Option<&crate::db::models::InterfaceSample>,
    ) -> Option<crate::alert::event::AlertEvent> {
        let severity = thresholds.severity(value)?;
        let threshold = if severity == AlertSeverity::Critical {
            thresholds.critical
        } else {
            thresholds.warning
        };
        let mut event = crate::alert::event::AlertEvent::new(
            crate::alert::event::AlertKind::Custom(format!("{}_HIGH", metric.to_uppercase())),
            severity,
            device,
            format!(
                "{} ({}) {}{} {:.2}% >= {:.2}% ({})",
                device.name,
                device.ip,
                metric,
                interface
                    .map(|sample| format!(" {} (if-{})", sample.if_name, sample.if_index))
                    .unwrap_or_default(),
                value,
                threshold,
                severity
            ),
        )
        .with_observed_value(format!("{value:.2}%"))
        .with_threshold(format!("{threshold:.2}%"));
        if let Some(sample) = interface {
            event = event
                .with_interface(sample.if_name.clone())
                .with_interface_id(i64::from(sample.if_index));
        }
        Some(event)
    }

    pub fn system_alerts(
        &self,
        device: &crate::device::types::DeviceConfig,
        cpu: Option<u32>,
        memory: Option<u32>,
    ) -> Vec<crate::alert::event::AlertEvent> {
        [("cpu", cpu, &self.cpu), ("memory", memory, &self.memory)]
            .into_iter()
            .filter_map(|(metric, value, thresholds)| {
                self.metric_alert(device, metric, f64::from(value?), thresholds, None)
            })
            .collect()
    }

    pub fn interface_alerts(
        &self,
        device: &crate::device::types::DeviceConfig,
        previous: Option<&crate::db::models::InterfaceSample>,
        current: &crate::db::models::InterfaceSample,
    ) -> Vec<crate::alert::event::AlertEvent> {
        let Some(previous) = previous else {
            return Vec::new();
        };
        let mut events = Vec::new();
        if let Some(event) = self.metric_alert(
            device,
            "bandwidth",
            current.bandwidth_utilization,
            &self.bandwidth,
            Some(current),
        ) {
            events.push(event);
        }
        if let Some(rate) = interface_error_rate(previous, current)
            && let Some(event) =
                self.metric_alert(device, "error_rate", rate, &self.error_rate, Some(current))
        {
            events.push(event);
        }
        events
    }
}

#[cfg(test)]
mod metric_tests {
    use super::*;

    #[test]
    fn metric_thresholds_classify_independently() {
        let thresholds = MetricThresholds::default();
        assert_eq!(thresholds.severity(100.0), Some(AlertSeverity::Critical));
        assert_eq!(thresholds.severity(90.0), Some(AlertSeverity::Critical));
        assert_eq!(thresholds.severity(80.0), Some(AlertSeverity::Warning));
        assert_eq!(thresholds.severity(79.9), None);
        assert_eq!(thresholds.severity(f64::NAN), None);
        assert!(thresholds.validate());
        assert!(
            !MetricThresholds {
                warning: 90.0,
                critical: 80.0
            }
            .validate()
        );
    }

    #[test]
    fn metric_alerts_identify_cpu_and_deduplicate_until_recovery() {
        let config = crate::config::AlertConfig::default();
        let mut device = crate::device::types::DeviceConfig::new("192.0.2.1", "public");
        device.id = Some(1);
        let mut tracker = MetricAlertTracker::default();
        let events = tracker.transitions(1, config.system_alerts(&device, Some(100), None));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind.as_str(), "CPU_HIGH");
        assert_eq!(events[0].severity, AlertSeverity::Critical);
        assert_eq!(events[0].observed_value.as_deref(), Some("100.00%"));
        assert!(
            tracker
                .transitions(1, config.system_alerts(&device, Some(100), None))
                .is_empty()
        );
        assert_eq!(
            tracker
                .transitions(1, config.system_alerts(&device, Some(80), None))
                .len(),
            1
        );
        assert!(tracker.transitions(1, Vec::new()).is_empty());
        assert_eq!(
            tracker
                .transitions(1, config.system_alerts(&device, Some(100), None))
                .len(),
            1
        );
    }

    #[test]
    fn metric_interface_rates_use_deltas_and_skip_missing_or_reset_counters() {
        let previous: crate::db::models::InterfaceSample = serde_json::from_value(serde_json::json!({
            "id": null, "device_id": 1, "if_index": 7, "if_name": "uplink", "link_status": "up",
            "in_errors": 100, "out_errors": 0, "in_packets": 1000, "out_packets": 0,
            "in_discards": 0, "out_discards": 0, "late_collisions": 0,
            "fcs_errors": 0, "alignment_errors": 0, "frame_too_longs": 0,
            "internal_mac_receive_errors": 0, "rx_optical_power_dbm": null,
            "in_octets": 0, "out_octets": 0, "bandwidth_utilization": 0.0, "sampled_at": "2026-10-06T00:00:00Z"
        })).unwrap();
        let mut current = previous.clone();
        current.in_errors += 1;
        current.in_packets += 99;
        current.bandwidth_utilization = 90.0;
        assert_eq!(interface_error_rate(&previous, &current), Some(1.0));
        let mut config = crate::config::AlertConfig::default();
        config.error_rate = MetricThresholds {
            warning: 1.0,
            critical: 2.0,
        };
        let device = crate::device::types::DeviceConfig::new("192.0.2.1", "public");
        let events = config.interface_alerts(&device, Some(&previous), &current);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].severity, AlertSeverity::Critical);
        assert_eq!(events[1].severity, AlertSeverity::Warning);
        assert_eq!(events[1].interface_id, Some(7));
        assert_eq!(events[1].observed_value.as_deref(), Some("1.00%"));
        assert!(config.interface_alerts(&device, None, &current).is_empty());
        assert_eq!(interface_error_rate(&previous, &previous), None);
        current.in_errors = 0;
        assert_eq!(interface_error_rate(&previous, &current), None);
        assert_eq!(
            config.system_alerts(&device, None, Some(100))[0]
                .kind
                .as_str(),
            "MEMORY_HIGH"
        );
    }
}

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
