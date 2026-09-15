use super::{
    AlertConfig, AppConfig, DisplayConfig, FlowConfig, PollingConfig, RetentionConfig, SnmpConfig,
};

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            interval_seconds: super::defaults::default_polling_interval(),
        }
    }
}

impl Default for SnmpConfig {
    fn default() -> Self {
        Self {
            default_community: super::defaults::default_snmp_community(),
            cpu_oid_override: super::defaults::default_cpu_oid_override(),
            memory_oid_override: super::defaults::default_memory_oid_override(),
            hardware_oid_overrides: super::defaults::default_hardware_oid_overrides(),
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            error_rate_threshold: super::defaults::default_error_rate_threshold(),
            spike_threshold: super::defaults::default_spike_threshold(),
            health_warning_threshold: super::defaults::default_warning_threshold(),
            health_critical_threshold: super::defaults::default_critical_threshold(),
        }
    }
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            history_days: super::defaults::default_history_days(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            timezone: super::defaults::default_timezone(),
            language: super::defaults::default_language(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            polling: PollingConfig::default(),
            snmp: SnmpConfig::default(),
            alert: AlertConfig::default(),
            retention: RetentionConfig::default(),
            display: DisplayConfig::default(),
            flow: FlowConfig::default(),
        }
    }
}
