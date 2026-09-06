use crate::db::models::InterfaceSample;

#[derive(Debug, Clone)]
pub struct InterfaceMonitor {
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
}

impl InterfaceMonitor {
    pub fn new(if_index: i32, if_name: &str) -> Self {
        Self {
            if_index,
            if_name: if_name.to_string(),
            link_status: "up".to_string(),
            in_errors: 0,
            out_errors: 0,
            in_packets: 0,
            out_packets: 0,
            in_discards: 0,
            out_discards: 0,
            late_collisions: 0,
            fcs_errors: 0,
            alignment_errors: 0,
            frame_too_longs: 0,
            internal_mac_receive_errors: 0,
            rx_optical_power_dbm: None,
            in_octets: 0,
            out_octets: 0,
            bandwidth_utilization: 0.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_snmp_snapshot(
        if_index: i32,
        if_name: &str,
        link_status: u32,
        in_errors: u64,
        out_errors: u64,
        in_packets: u64,
        out_packets: u64,
        in_discards: u64,
        out_discards: u64,
        late_collisions: u64,
        fcs_errors: u64,
        alignment_errors: u64,
        frame_too_longs: u64,
        internal_mac_receive_errors: u64,
        rx_optical_power_dbm: Option<f64>,
        in_octets: u64,
        out_octets: u64,
    ) -> Self {
        let status = match link_status {
            1 => "up",
            2 => "down",
            3 => "testing",
            4 => "unknown",
            5 => "dormant",
            6 => "notPresent",
            7 => "lowerLayerDown",
            _ => "unknown",
        };

        Self {
            if_index,
            if_name: if_name.to_string(),
            link_status: status.to_string(),
            in_errors,
            out_errors,
            in_packets,
            out_packets,
            in_discards,
            out_discards,
            late_collisions,
            fcs_errors,
            alignment_errors,
            frame_too_longs,
            internal_mac_receive_errors,
            rx_optical_power_dbm,
            in_octets,
            out_octets,
            bandwidth_utilization: 0.0,
        }
    }

    pub fn into_sample(&self, device_id: i64) -> InterfaceSample {
        InterfaceSample {
            id: None,
            device_id,
            if_index: self.if_index,
            if_name: self.if_name.clone(),
            link_status: self.link_status.clone(),
            in_errors: self.in_errors,
            out_errors: self.out_errors,
            in_packets: self.in_packets,
            out_packets: self.out_packets,
            in_discards: self.in_discards,
            out_discards: self.out_discards,
            late_collisions: self.late_collisions,
            fcs_errors: self.fcs_errors,
            alignment_errors: self.alignment_errors,
            frame_too_longs: self.frame_too_longs,
            internal_mac_receive_errors: self.internal_mac_receive_errors,
            rx_optical_power_dbm: self.rx_optical_power_dbm,
            in_octets: self.in_octets,
            out_octets: self.out_octets,
            bandwidth_utilization: self.bandwidth_utilization,
            sampled_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
