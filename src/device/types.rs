#[derive(Clone, PartialEq, Eq)]
pub struct DeviceConfig {
    pub id: Option<i64>,
    pub name: String,
    pub ip: String,
    pub community: String,
    pub device_type: String,
    pub status: String,
    pub last_seen_at: Option<String>,
}

impl std::fmt::Debug for DeviceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceConfig")
            .field("id", &self.id)
            .field("ip", &self.ip)
            .field("community", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl DeviceConfig {
    pub fn new(ip: impl Into<String>, community: impl Into<String>) -> Self {
        let ip = ip.into();
        let community = community.into();
        let hostname = format!("device-{}", ip.split('.').next_back().unwrap_or("unknown"));

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
