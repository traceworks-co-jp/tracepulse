pub mod defaults;
pub mod settings;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PollingConfig {
    #[serde(default = "defaults::default_polling_interval")]
    pub interval_seconds: u64,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SnmpConfig {
    #[serde(default = "defaults::default_snmp_community")]
    #[serde(skip_serializing)]
    pub default_community: String,
    #[serde(default = "defaults::default_cpu_oid_override")]
    pub cpu_oid_override: String,
    #[serde(default = "defaults::default_memory_oid_override")]
    pub memory_oid_override: String,
    /// Hardware センサーの手動 OID 上書きリスト。複数登録可能。
    /// 各要素は "sensor_type|oid" 形式(sensor_type は temperature / power / fan のいずれか)。
    #[serde(default = "defaults::default_hardware_oid_overrides")]
    pub hardware_oid_overrides: Vec<String>,
}

impl std::fmt::Debug for SnmpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnmpConfig")
            .field("default_community", &"[REDACTED]")
            .field("cpu_oid_override", &self.cpu_oid_override)
            .field("memory_oid_override", &self.memory_oid_override)
            .field("hardware_oid_overrides", &self.hardware_oid_overrides)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlertConfig {
    #[serde(default)]
    pub cpu: crate::alert::rules::MetricThresholds,
    #[serde(default)]
    pub memory: crate::alert::rules::MetricThresholds,
    #[serde(default)]
    pub bandwidth: crate::alert::rules::MetricThresholds,
    #[serde(default = "defaults::default_error_rate_thresholds")]
    pub error_rate: crate::alert::rules::MetricThresholds,
    #[serde(default = "defaults::default_error_rate_threshold")]
    pub error_rate_threshold: f64,
    #[serde(default = "defaults::default_spike_threshold")]
    pub spike_threshold: u64,
    #[serde(default = "defaults::default_warning_threshold")]
    pub health_warning_threshold: u32,
    #[serde(default = "defaults::default_critical_threshold")]
    pub health_critical_threshold: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetentionConfig {
    #[serde(default = "defaults::default_history_days")]
    pub history_days: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
    #[serde(default = "defaults::default_timezone")]
    pub timezone: String,
    #[serde(default = "defaults::default_language")]
    pub language: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FlowConfig {
    #[serde(default = "defaults::default_flow_bind_addr")]
    pub bind_addr: String,
    #[serde(default = "defaults::default_netflow_port")]
    pub netflow_port: u16,
    #[serde(default = "defaults::default_ipfix_port")]
    pub ipfix_port: u16,
    #[serde(default = "defaults::default_sflow_port")]
    pub sflow_port: u16,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            bind_addr: defaults::default_flow_bind_addr(),
            netflow_port: defaults::default_netflow_port(),
            ipfix_port: defaults::default_ipfix_port(),
            sflow_port: defaults::default_sflow_port(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebConfig {
    #[serde(default = "defaults::default_web_port")]
    pub port: std::num::NonZeroU16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub polling: PollingConfig,
    #[serde(default)]
    pub snmp: SnmpConfig,
    #[serde(default)]
    pub alert: AlertConfig,
    #[serde(default)]
    pub retention: RetentionConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub flow: FlowConfig,
}

impl AppConfig {
    pub(crate) fn verify_stored_key(path: &std::path::Path) -> Result<(), crate::error::AppError> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => zeroize::Zeroizing::new(content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let root: toml::Value = toml::from_str(&content).map_err(|_| {
            crate::error::AppError::Config("invalid TOML configuration (contents redacted)".into())
        })?;
        if let Some(value) = root
            .get("snmp")
            .and_then(|snmp| snmp.get("default_community"))
            .and_then(toml::Value::as_str)
            .filter(|value| value.starts_with("tpenc:"))
        {
            let directory = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(std::path::Path::new("."));
            crate::crypto::MasterKey::load(directory, false)?
                .decrypt(value, CONFIG_SECRET_CONTEXT)?;
        }
        Ok(())
    }

    pub fn load(path: &std::path::Path) -> Result<Self, crate::error::AppError> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => zeroize::Zeroizing::new(content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.into()),
        };
        let mut root: toml::Value = toml::from_str(&content).map_err(|_| {
            crate::error::AppError::Config("invalid TOML configuration (contents redacted)".into())
        })?;
        let stored = root
            .get("snmp")
            .and_then(|snmp| snmp.get("default_community"))
            .and_then(toml::Value::as_str)
            .map(str::to_string);
        let mut migration = None;
        if let Some(value) = &stored {
            let directory = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(std::path::Path::new("."));
            let key = crate::crypto::MasterKey::load(directory, !value.starts_with("tpenc:"))?;
            if value.starts_with("tpenc:") {
                root["snmp"]["default_community"] =
                    toml::Value::String(key.decrypt_string(value, CONFIG_SECRET_CONTEXT)?);
            } else {
                migration = Some(key.encrypt(value.as_bytes(), CONFIG_SECRET_CONTEXT)?);
            }
        }
        let mut config: AppConfig = root.clone().try_into().map_err(|_| {
            crate::error::AppError::Config(
                "invalid configuration fields (contents redacted)".into(),
            )
        })?;
        if let Some(alert) = root.get("alert")
            && alert.get("error_rate").is_none()
            && alert.get("error_rate_threshold").is_some()
        {
            let warning = config.alert.error_rate_threshold * 100.0;
            let migrated = crate::alert::rules::MetricThresholds {
                warning,
                critical: (warning * 2.0).min(100.0),
            };
            if migrated.validate() {
                config.alert.error_rate = migrated;
            }
        }
        if [
            &config.alert.cpu,
            &config.alert.memory,
            &config.alert.bandwidth,
            &config.alert.error_rate,
        ]
        .iter()
        .any(|thresholds| !thresholds.validate())
        {
            return Err(crate::error::AppError::Config(
                "metric thresholds must satisfy 0 < warning < critical <= 100".into(),
            ));
        }
        if let Some(envelope) = migration {
            root["snmp"]["default_community"] = toml::Value::String(envelope);
            let serialized = toml::to_string_pretty(&root).map_err(|_| {
                crate::error::AppError::Config("configuration serialization failed".into())
            })?;
            crate::crypto::write_private_atomic(path, serialized.as_bytes())?;
        }
        Ok(config)
    }
}

pub(crate) const CONFIG_SECRET_CONTEXT: &[u8] = b"TracePulse/config/snmp/default_community";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Cli,
    Web,
}
