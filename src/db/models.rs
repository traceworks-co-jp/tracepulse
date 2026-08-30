use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Option<i64>,
    pub name: String,
    pub ip: String,
    pub community: String,
    pub device_type: String,
    pub status: String,
    pub last_seen_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSample {
    pub id: Option<i64>,
    pub device_id: i64,
    pub if_index: i32,
    pub if_name: String,
    pub link_status: String,
    pub in_errors: u64,
    pub out_errors: u64,
    pub in_discards: u64,
    pub out_discards: u64,
    pub in_octets: u64,
    pub out_octets: u64,
    pub bandwidth_utilization: f64,
    pub sampled_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub id: Option<i64>,
    pub device_id: i64,
    pub alert_type: String,
    pub severity: String,
    pub details: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentAlert {
    pub id: Option<i64>,
    pub device_id: i64,
    pub device_name: String,
    pub device_ip: String,
    pub alert_type: String,
    pub severity: String,
    pub details: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceMetrics {
    pub id: Option<i64>,
    pub device_id: i64,
    pub cpu_usage: Option<u32>,
    pub memory_usage: Option<u32>,
    pub sampled_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSpike {
    pub if_index: i32,
    pub if_name: String,
    pub link_status: String,
    pub in_errors_delta: u64,
    pub out_errors_delta: u64,
    pub in_discards_delta: u64,
    pub out_discards_delta: u64,
    pub total_delta: u64,
    pub latest_sampled_at: String,
    pub previous_sampled_at: String,
}
