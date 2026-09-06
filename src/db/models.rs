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
    pub in_packets: u64,
    pub out_packets: u64,
    pub in_discards: u64,
    pub out_discards: u64,
    pub late_collisions: u64,
    // EtherLike-MIB (RFC 3635) 破損パケット内訳カウンタ
    pub fcs_errors: u64,
    pub alignment_errors: u64,
    pub frame_too_longs: u64,
    pub internal_mac_receive_errors: u64,
    pub rx_optical_power_dbm: Option<f64>,
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
    pub memory_used_bytes: Option<u64>,
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
    pub late_collisions_delta: u64,
    pub total_delta: u64,
    pub latest_sampled_at: String,
    pub previous_sampled_at: String,
}

/// L1/L2 障害検知用のポート単位カウンタ差分（Counter32 ラップアラウンド考慮済み）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfacePortDelta {
    pub in_errors_delta: u64,
    pub out_errors_delta: u64,
    pub in_discards_delta: u64,
    pub out_discards_delta: u64,
    pub late_collisions_delta: u64,
    // EtherLike-MIB 破損パケット内訳（Error Breakdown カード用）
    pub fcs_errors_delta: u64,
    pub alignment_errors_delta: u64,
    pub frame_too_longs_delta: u64,
    pub internal_mac_receive_errors_delta: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowRecord {
    pub source_ip: String,
    pub destination_ip: String,
    pub source_port: u16,
    pub destination_port: u16,
    pub protocol: String,
    pub bytes: u64,
    pub packets: u64,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolShare {
    pub protocol: String,
    pub bytes: u64,
    pub percentage: f64,
    pub bps: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopTalker {
    pub source_ip: String,
    pub destination_ip: String,
    pub source_port: u16,
    pub destination_port: u16,
    pub protocol: String,
    pub bytes: u64,
    pub bps: u64,
}
