use crate::alert::status::AlertSeverity;
use crate::db::models::InterfaceSpike;
use crate::device::types::DeviceConfig;
use chrono::{DateTime, Utc};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// アラートの種別。外部モジュール（通知プラグイン等）が分岐に利用する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertKind {
    InterfaceSpike,
    ErrorRate,
    HealthDegraded,
    DeviceOffline,
    Custom(String),
}

impl AlertKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::InterfaceSpike => "SPIKE",
            Self::ErrorRate => "ERROR_RATE",
            Self::HealthDegraded => "HEALTH_DEGRADED",
            Self::DeviceOffline => "DEVICE_OFFLINE",
            Self::Custom(name) => name,
        }
    }
}

impl fmt::Display for AlertKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// コアエンジンが検知したアラートを表す構造化イベント。
#[derive(Debug, Clone)]
pub struct AlertEvent {
    pub id: String,
    pub device_id: Option<i64>,
    pub device_name: String,
    pub device_ip: String,
    pub interface_id: Option<i64>,
    pub interface_name: Option<String>,
    pub kind: AlertKind,
    pub severity: AlertSeverity,
    pub message: String,
    pub observed_value: Option<String>,
    pub threshold: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl AlertEvent {
    pub fn new(
        kind: AlertKind,
        severity: AlertSeverity,
        device: &DeviceConfig,
        message: impl Into<String>,
    ) -> Self {
        let occurred_at = Utc::now();
        Self {
            id: next_event_id(&kind, &occurred_at),
            device_id: device.id,
            device_name: device.name.clone(),
            device_ip: device.ip.clone(),
            interface_id: None,
            interface_name: None,
            kind,
            severity,
            message: message.into(),
            observed_value: None,
            threshold: None,
            occurred_at,
        }
    }

    pub fn with_interface(mut self, interface_name: impl Into<String>) -> Self {
        self.interface_name = Some(interface_name.into());
        self
    }

    pub fn with_interface_id(mut self, interface_id: i64) -> Self {
        self.interface_id = Some(interface_id);
        self
    }

    pub fn with_observed_value(mut self, observed_value: impl Into<String>) -> Self {
        self.observed_value = Some(observed_value.into());
        self
    }

    pub fn with_threshold(mut self, threshold: impl Into<String>) -> Self {
        self.threshold = Some(threshold.into());
        self
    }

    /// インターフェースのエラー/破棄カウンタ急増から生成する。
    pub fn from_interface_spike(
        device: &DeviceConfig,
        spike: &InterfaceSpike,
        spike_threshold: u64,
    ) -> Self {
        let message = format!(
            "SPIKE {} ({}) {} (if-{}) total_delta={} in_errors={} out_errors={} in_discards={} out_discards={}",
            device.name,
            device.ip,
            spike.if_name,
            spike.if_index,
            spike.total_delta,
            spike.in_errors_delta,
            spike.out_errors_delta,
            spike.in_discards_delta,
            spike.out_discards_delta
        );

        Self::new(
            AlertKind::InterfaceSpike,
            AlertSeverity::Warning,
            device,
            message,
        )
        .with_interface(spike.if_name.clone())
        .with_interface_id(i64::from(spike.if_index))
        .with_observed_value(spike.total_delta.to_string())
        .with_threshold(spike_threshold.to_string())
    }

    /// ポーリング失敗（応答なし）から生成する。
    pub fn device_offline(device: &DeviceConfig, reason: impl fmt::Display) -> Self {
        let message = format!("{} ({}) -> {}", device.name, device.ip, reason);
        Self::new(
            AlertKind::DeviceOffline,
            AlertSeverity::Offline,
            device,
            message,
        )
    }

    pub fn custom(
        kind: impl Into<String>,
        device: &DeviceConfig,
        interface_id: i32,
        interface_name: &str,
        observed: impl Into<String>,
        threshold: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            AlertKind::Custom(kind.into()),
            AlertSeverity::Warning,
            device,
            message,
        )
        .with_interface(interface_name)
        .with_interface_id(i64::from(interface_id))
        .with_observed_value(observed)
        .with_threshold(threshold)
    }
}

fn next_event_id(kind: &AlertKind, occurred_at: &DateTime<Utc>) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}-{}-{}",
        kind.as_str().to_ascii_lowercase(),
        occurred_at.timestamp_micros(),
        seq
    )
}
