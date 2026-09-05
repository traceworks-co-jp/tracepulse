#[derive(Debug, Clone)]
pub struct SystemMonitor {
    pub cpu_usage: Option<u32>,
    pub memory_usage: Option<u32>,
    pub memory_used_bytes: Option<u64>,
    pub sys_name: String,
    pub sys_descr: String,
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            cpu_usage: None,
            memory_usage: None,
            memory_used_bytes: None,
            sys_name: "unknown".to_string(),
            sys_descr: "not polled yet".to_string(),
        }
    }

    pub fn from_snmp_device_info(info: &crate::snmp::SnmpDeviceInfo) -> Self {
        Self {
            cpu_usage: info.cpu_usage,
            memory_usage: info.memory_usage,
            memory_used_bytes: info.memory_used_bytes,
            sys_name: info.sys_name.clone(),
            sys_descr: info.sys_descr.clone(),
        }
    }
}
