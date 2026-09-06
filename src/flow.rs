use crate::db::models::FlowRecord;
use crate::db::repository::Repository;
use std::net::{Ipv4Addr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowProtocol {
    NetFlow,
    Ipfix,
    SFlow,
}

impl FlowProtocol {
    pub fn port(self) -> u16 {
        match self {
            Self::NetFlow => 2055,
            Self::Ipfix => 4739,
            Self::SFlow => 6343,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NetFlow => "NetFlow",
            Self::Ipfix => "IPFIX",
            Self::SFlow => "sFlow",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlowCollectorConfig {
    pub bind_addr: [u8; 4],
    pub netflow_port: u16,
    pub ipfix_port: u16,
    pub sflow_port: u16,
}

impl Default for FlowCollectorConfig {
    fn default() -> Self {
        Self {
            bind_addr: [0, 0, 0, 0],
            netflow_port: FlowProtocol::NetFlow.port(),
            ipfix_port: FlowProtocol::Ipfix.port(),
            sflow_port: FlowProtocol::SFlow.port(),
        }
    }
}

pub struct FlowCollector;

impl FlowCollector {
    pub fn start(repository: Arc<Mutex<Repository>>, config: FlowCollectorConfig) {
        for protocol in [
            FlowProtocol::NetFlow,
            FlowProtocol::Ipfix,
            FlowProtocol::SFlow,
        ] {
            let repo = Arc::clone(&repository);
            let cfg = config;
            thread::spawn(move || run_listener(protocol, repo, cfg));
        }
    }
}

fn run_listener(
    protocol: FlowProtocol,
    repository: Arc<Mutex<Repository>>,
    config: FlowCollectorConfig,
) {
    let port = match protocol {
        FlowProtocol::NetFlow => config.netflow_port,
        FlowProtocol::Ipfix => config.ipfix_port,
        FlowProtocol::SFlow => config.sflow_port,
    };
    let socket = match UdpSocket::bind((Ipv4Addr::from(config.bind_addr), port)) {
        Ok(socket) => socket,
        Err(error) => {
            eprintln!(
                "flow collector {} bind failed on {}: {}",
                protocol.label(),
                port,
                error
            );
            return;
        }
    };
    let _ = socket.set_read_timeout(Some(Duration::from_secs(1)));
    eprintln!(
        "flow collector {} listening on UDP {}",
        protocol.label(),
        port
    );
    let mut buffer = [0u8; 65535];
    loop {
        match socket.recv(&mut buffer) {
            Ok(length) => {
                let records = parse_datagram(protocol, &buffer[..length]);
                if let Ok(repository) = repository.lock() {
                    for record in records {
                        let _ = repository.save_flow_record(&record);
                    }
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(error) => {
                eprintln!(
                    "flow collector {} receive failed: {}",
                    protocol.label(),
                    error
                );
                break;
            }
        }
    }
}

fn parse_datagram(protocol: FlowProtocol, payload: &[u8]) -> Vec<FlowRecord> {
    let observed_at = chrono::Utc::now().to_rfc3339();
    match protocol {
        FlowProtocol::NetFlow => parse_netflow_v5(payload, &observed_at),
        FlowProtocol::Ipfix | FlowProtocol::SFlow => vec![FlowRecord {
            source_ip: "unknown".to_string(),
            destination_ip: "unknown".to_string(),
            source_port: 0,
            destination_port: 0,
            protocol: protocol.label().to_string(),
            bytes: payload.len() as u64,
            packets: 1,
            observed_at,
        }],
    }
}

fn parse_netflow_v5(payload: &[u8], observed_at: &str) -> Vec<FlowRecord> {
    if payload.len() < 24 || u16::from_be_bytes([payload[0], payload[1]]) != 5 {
        return Vec::new();
    }
    let count = u16::from_be_bytes([payload[2], payload[3]]) as usize;
    let mut records = Vec::new();
    for index in 0..count {
        let offset = 24 + index * 48;
        if offset + 48 > payload.len() {
            break;
        }
        let source_ip = ipv4(&payload[offset..offset + 4]);
        let destination_ip = ipv4(&payload[offset + 4..offset + 8]);
        let packets =
            u32::from_be_bytes(payload[offset + 16..offset + 20].try_into().unwrap()) as u64;
        let bytes =
            u32::from_be_bytes(payload[offset + 20..offset + 24].try_into().unwrap()) as u64;
        let protocol = match payload[offset + 38] {
            6 => "TCP",
            17 => "UDP",
            1 => "ICMP",
            value => return_unknown_protocol(value),
        };
        let source_port = u16::from_be_bytes(payload[offset + 40..offset + 42].try_into().unwrap());
        let destination_port =
            u16::from_be_bytes(payload[offset + 42..offset + 44].try_into().unwrap());
        records.push(FlowRecord {
            source_ip,
            destination_ip,
            source_port,
            destination_port,
            protocol: protocol.to_string(),
            bytes,
            packets,
            observed_at: observed_at.to_string(),
        });
    }
    records
}

fn return_unknown_protocol(value: u8) -> &'static str {
    match value {
        2 => "IGMP",
        47 => "GRE",
        50 => "ESP",
        _ => "Other",
    }
}

fn ipv4(value: &[u8]) -> String {
    format!("{}.{}.{}.{}", value[0], value[1], value[2], value[3])
}

#[allow(dead_code)]
fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_netflow_v5_record() {
        let mut packet = vec![0u8; 72];
        packet[0..2].copy_from_slice(&5u16.to_be_bytes());
        packet[2..4].copy_from_slice(&1u16.to_be_bytes());
        packet[24..28].copy_from_slice(&[192, 0, 2, 1]);
        packet[28..32].copy_from_slice(&[198, 51, 100, 2]);
        packet[40..44].copy_from_slice(&10u32.to_be_bytes());
        packet[44..48].copy_from_slice(&1000u32.to_be_bytes());
        packet[62] = 6;
        packet[64..66].copy_from_slice(&1234u16.to_be_bytes());
        packet[66..68].copy_from_slice(&443u16.to_be_bytes());
        let records = parse_netflow_v5(&packet, "now");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].protocol, "TCP");
        assert_eq!(records[0].bytes, 1000);
    }
}
