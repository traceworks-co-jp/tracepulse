pub mod defaults;
pub mod settings;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PollingConfig {
    #[serde(default = "defaults::default_polling_interval")]
    pub interval_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnmpConfig {
    #[serde(default = "defaults::default_snmp_community")]
    pub default_community: String,
    #[serde(default = "defaults::default_cpu_oid_override")]
    pub cpu_oid_override: String,
    #[serde(default = "defaults::default_memory_oid_override")]
    pub memory_oid_override: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlertConfig {
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
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
}

impl AppConfig {
    pub fn load(path: &std::path::Path) -> Result<Self, crate::error::AppError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Cli,
    Web,
}
