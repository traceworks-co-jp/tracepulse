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
    /// Hardware センサーの手動 OID 上書きリスト。複数登録可能。
    /// 各要素は "sensor_type|oid" 形式(sensor_type は temperature / power / fan のいずれか)。
    #[serde(default = "defaults::default_hardware_oid_overrides")]
    pub hardware_oid_overrides: Vec<String>,
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
    #[serde(default)]
    pub flow: FlowConfig,
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
