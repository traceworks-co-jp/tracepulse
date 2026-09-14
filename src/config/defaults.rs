pub fn default_polling_interval() -> u64 {
    30
}

pub fn default_snmp_community() -> String {
    "public".to_string()
}

pub fn default_cpu_oid_override() -> String {
    String::new()
}

pub fn default_memory_oid_override() -> String {
    String::new()
}

pub fn default_hardware_oid_overrides() -> Vec<String> {
    Vec::new()
}

pub fn default_error_rate_threshold() -> f64 {
    0.05
}

pub fn default_spike_threshold() -> u64 {
    10
}

pub fn default_warning_threshold() -> u32 {
    80
}

pub fn default_critical_threshold() -> u32 {
    60
}

pub fn default_history_days() -> u32 {
    7
}

pub fn default_timezone() -> String {
    "utc".to_string()
}

pub fn default_language() -> String {
    "en".to_string()
}

pub fn default_flow_bind_addr() -> String {
    "0.0.0.0".to_string()
}
pub fn default_netflow_port() -> u16 {
    2055
}
pub fn default_ipfix_port() -> u16 {
    4739
}
pub fn default_sflow_port() -> u16 {
    6343
}
