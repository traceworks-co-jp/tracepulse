#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceConfig {
    pub id: Option<i64>,
    pub name: String,
    pub ip: String,
    pub community: String,
    pub device_type: String,
    pub status: String,
    pub last_seen_at: Option<String>,
}

impl DeviceConfig {
    pub fn new(ip: impl Into<String>, community: impl Into<String>) -> Self {
        let ip = ip.into();
        let community = community.into();
        let hostname = format!("device-{}", ip.split('.').last().unwrap_or("unknown"));

        Self {
            id: None,
            name: hostname,
            ip: ip.clone(),
            community,
            device_type: "router-switch".to_string(),
            status: "unknown".to_string(),
            last_seen_at: None,
        }
    }
}
