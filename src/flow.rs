use crate::db::models::FlowRecord;
use crate::db::repository::Repository;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TEMPLATE_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TemplateFormat {
    NetFlowV9,
    Ipfix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TemplateKey {
    format: TemplateFormat,
    exporter: IpAddr,
    observation_domain_id: u32,
    template_id: u16,
}

#[derive(Debug, Clone)]
struct TemplateField {
    information_element: u16,
    length: u16,
}

#[derive(Debug, Clone)]
struct CachedTemplate {
    fields: Vec<TemplateField>,
    received_at: Instant,
}

type TemplateCache = Arc<Mutex<HashMap<TemplateKey, CachedTemplate>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        let templates: TemplateCache = Arc::new(Mutex::new(HashMap::new()));
        for protocol in [
            FlowProtocol::NetFlow,
            FlowProtocol::Ipfix,
            FlowProtocol::SFlow,
        ] {
            let repo = Arc::clone(&repository);
            let cfg = config;
            let templates = Arc::clone(&templates);
            thread::spawn(move || run_listener(protocol, repo, cfg, templates));
        }
    }
}

fn run_listener(
    protocol: FlowProtocol,
    repository: Arc<Mutex<Repository>>,
    config: FlowCollectorConfig,
    templates: TemplateCache,
) {
    let port = match protocol {
        FlowProtocol::NetFlow => config.netflow_port,
        FlowProtocol::Ipfix => config.ipfix_port,
        FlowProtocol::SFlow => config.sflow_port,
    };
    let socket = match UdpSocket::bind((Ipv4Addr::from(config.bind_addr), port)) {
        Ok(socket) => socket,
        Err(error) => {
            tracing::error!(
                "flow collector {} bind failed on {}: {}",
                protocol.label(),
                port,
                error
            );
            return;
        }
    };
    let _ = socket.set_read_timeout(Some(Duration::from_secs(1)));
    tracing::info!(
        "flow collector {} listening on UDP {}",
        protocol.label(),
        port
    );
    let mut buffer = [0u8; 65535];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((length, exporter)) => {
                let records = parse_datagram_with_templates(
                    protocol,
                    &buffer[..length],
                    exporter.ip(),
                    &templates,
                );
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
                tracing::error!(
                    "flow collector {} receive failed: {}",
                    protocol.label(),
                    error
                );
                break;
            }
        }
    }
}

#[cfg(test)]
fn parse_datagram(protocol: FlowProtocol, payload: &[u8]) -> Vec<FlowRecord> {
    let templates: TemplateCache = Arc::new(Mutex::new(HashMap::new()));
    parse_datagram_with_templates(protocol, payload, IpAddr::V4(Ipv4Addr::UNSPECIFIED), &templates)
}

fn parse_datagram_with_templates(
    protocol: FlowProtocol,
    payload: &[u8],
    exporter: IpAddr,
    templates: &TemplateCache,
) -> Vec<FlowRecord> {
    let observed_at = chrono::Utc::now().to_rfc3339();
    match protocol {
        FlowProtocol::NetFlow => match read_u16(payload, 0) {
            Some(5) => parse_netflow_v5(payload, &observed_at),
            Some(9) => parse_template_flow_message(
                TemplateFormat::NetFlowV9,
                payload,
                exporter,
                templates,
                &observed_at,
            ),
            _ => Vec::new(),
        },
        FlowProtocol::Ipfix => parse_template_flow_message(
            TemplateFormat::Ipfix,
            payload,
            exporter,
            templates,
            &observed_at,
        ),
        FlowProtocol::SFlow => {
            tracing::debug!(
                "received unsupported {} datagram ({} bytes); excluded from flow analytics",
                protocol.label(),
                payload.len()
            );
            Vec::new()
        }
    }
}

fn parse_template_flow_message(
    format: TemplateFormat,
    payload: &[u8],
    exporter: IpAddr,
    templates: &TemplateCache,
    observed_at: &str,
) -> Vec<FlowRecord> {
    let (header_len, observation_domain_id, template_set_id) = match format {
        TemplateFormat::NetFlowV9 if read_u16(payload, 0) == Some(9) && payload.len() >= 20 => {
            (20, read_u32(payload, 16).unwrap_or_default(), 0)
        }
        TemplateFormat::Ipfix if read_u16(payload, 0) == Some(10) && payload.len() >= 16 => {
            let message_len = read_u16(payload, 2).unwrap_or_default() as usize;
            if message_len < 16 || message_len > payload.len() {
                return Vec::new();
            }
            (16, read_u32(payload, 12).unwrap_or_default(), 2)
        }
        _ => return Vec::new(),
    };
    let message = if format == TemplateFormat::Ipfix {
        &payload[..read_u16(payload, 2).unwrap_or_default() as usize]
    } else {
        payload
    };
    let mut records = Vec::new();
    let mut offset = header_len;

    while offset + 4 <= message.len() {
        let set_id = read_u16(message, offset).unwrap_or_default();
        let set_len = read_u16(message, offset + 2).unwrap_or_default() as usize;
        if set_len < 4 || offset + set_len > message.len() {
            break;
        }
        let set = &message[offset + 4..offset + set_len];
        if set_id == template_set_id {
            cache_templates(format, exporter, observation_domain_id, set, templates);
        } else if set_id >= 256 {
            records.extend(decode_data_set(
                format,
                exporter,
                observation_domain_id,
                set_id,
                set,
                templates,
                observed_at,
            ));
        }
        offset += set_len;
    }
    records
}

fn cache_templates(
    format: TemplateFormat,
    exporter: IpAddr,
    observation_domain_id: u32,
    set: &[u8],
    templates: &TemplateCache,
) {
    let Ok(mut cache) = templates.lock() else {
        return;
    };
    cache.retain(|_, template| template.received_at.elapsed() <= TEMPLATE_TTL);

    let mut offset = 0;
    while offset + 4 <= set.len() {
        let template_id = read_u16(set, offset).unwrap_or_default();
        let field_count = read_u16(set, offset + 2).unwrap_or_default() as usize;
        offset += 4;
        let mut fields = Vec::with_capacity(field_count);
        let mut valid = template_id >= 256;
        for _ in 0..field_count {
            if offset + 4 > set.len() {
                valid = false;
                break;
            }
            let raw_element = read_u16(set, offset).unwrap_or_default();
            let length = read_u16(set, offset + 2).unwrap_or_default();
            offset += 4;
            if raw_element & 0x8000 != 0 {
                if offset + 4 > set.len() {
                    valid = false;
                    break;
                }
                offset += 4;
            }
            fields.push(TemplateField {
                information_element: raw_element & 0x7fff,
                length,
            });
        }
        if valid {
            cache.insert(
                TemplateKey {
                    format,
                    exporter,
                    observation_domain_id,
                    template_id,
                },
                CachedTemplate {
                    fields,
                    received_at: Instant::now(),
                },
            );
        }
    }
}

fn decode_data_set(
    format: TemplateFormat,
    exporter: IpAddr,
    observation_domain_id: u32,
    template_id: u16,
    set: &[u8],
    templates: &TemplateCache,
    observed_at: &str,
) -> Vec<FlowRecord> {
    let key = TemplateKey {
        format,
        exporter,
        observation_domain_id,
        template_id,
    };
    let fields = templates
        .lock()
        .ok()
        .and_then(|mut cache| {
            cache.retain(|_, template| template.received_at.elapsed() <= TEMPLATE_TTL);
            cache.get(&key).map(|template| template.fields.clone())
        });
    let Some(fields) = fields else {
        tracing::debug!("received flow data set without a cached template; excluded from analytics");
        return Vec::new();
    };

    let mut records = Vec::new();
    let mut offset = 0;
    while offset < set.len() {
        let Some((record, length)) = decode_record(&fields, &set[offset..], observed_at) else {
            break;
        };
        if length == 0 {
            break;
        }
        offset += length;
        if let Some(record) = record {
            records.push(record);
        }
    }
    records
}

fn decode_record(
    fields: &[TemplateField],
    data: &[u8],
    observed_at: &str,
) -> Option<(Option<FlowRecord>, usize)> {
    let mut offset = 0;
    let mut source_ip = None;
    let mut destination_ip = None;
    let mut source_port = 0;
    let mut destination_port = 0;
    let mut protocol = None;
    let mut bytes = 0;
    let mut packets = 0;

    for field in fields {
        let length = if field.length == u16::MAX {
            let first = *data.get(offset)? as usize;
            offset += 1;
            if first < 255 {
                first
            } else {
                let length = read_u16(data, offset)? as usize;
                offset += 2;
                length
            }
        } else {
            field.length as usize
        };
        let value = data.get(offset..offset + length)?;
        offset += length;
        match field.information_element {
            8 | 27 => source_ip = decode_ip(value),
            12 | 28 => destination_ip = decode_ip(value),
            7 => source_port = decode_number(value).unwrap_or_default() as u16,
            11 => destination_port = decode_number(value).unwrap_or_default() as u16,
            4 => protocol = decode_number(value).map(|value| return_unknown_protocol(value as u8)),
            1 | 85 => bytes = decode_number(value).unwrap_or_default(),
            2 | 86 => packets = decode_number(value).unwrap_or_default(),
            _ => {}
        }
    }

    let record = match (source_ip, destination_ip, protocol) {
        (Some(source_ip), Some(destination_ip), Some(protocol)) => Some(FlowRecord {
            source_ip,
            destination_ip,
            source_port,
            destination_port,
            protocol: protocol.to_string(),
            bytes,
            packets,
            observed_at: observed_at.to_string(),
        }),
        _ => None,
    };
    Some((record, offset))
}

fn decode_ip(value: &[u8]) -> Option<String> {
    match value {
        [a, b, c, d] => Some(IpAddr::V4(Ipv4Addr::new(*a, *b, *c, *d)).to_string()),
        value if value.len() == 16 => {
            let octets: [u8; 16] = value.try_into().ok()?;
            Some(std::net::Ipv6Addr::from(octets).to_string())
        }
        _ => None,
    }
}

fn decode_number(value: &[u8]) -> Option<u64> {
    if value.is_empty() || value.len() > 8 {
        return None;
    }
    Some(value.iter().fold(0_u64, |number, byte| (number << 8) | u64::from(*byte)))
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(offset..offset + 2)?.try_into().ok()?))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(offset..offset + 4)?.try_into().ok()?))
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
        1 => "ICMP",
        2 => "IGMP",
        6 => "TCP",
        17 => "UDP",
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

    const BASIC_FIELDS: &[(u16, u16)] = &[
        (8, 4),
        (12, 4),
        (7, 2),
        (11, 2),
        (4, 1),
        (1, 4),
        (2, 4),
    ];

    fn append_basic_template(set: &mut Vec<u8>) {
        set.extend_from_slice(&256_u16.to_be_bytes());
        set.extend_from_slice(&(BASIC_FIELDS.len() as u16).to_be_bytes());
        for (element, length) in BASIC_FIELDS {
            set.extend_from_slice(&element.to_be_bytes());
            set.extend_from_slice(&length.to_be_bytes());
        }
    }

    fn append_basic_record(set: &mut Vec<u8>) {
        set.extend_from_slice(&[192, 0, 2, 1]);
        set.extend_from_slice(&[198, 51, 100, 2]);
        set.extend_from_slice(&1234_u16.to_be_bytes());
        set.extend_from_slice(&443_u16.to_be_bytes());
        set.push(6);
        set.extend_from_slice(&1_000_u32.to_be_bytes());
        set.extend_from_slice(&10_u32.to_be_bytes());
    }

    fn append_set(message: &mut Vec<u8>, set_id: u16, set: &[u8]) {
        message.extend_from_slice(&set_id.to_be_bytes());
        message.extend_from_slice(&((set.len() + 4) as u16).to_be_bytes());
        message.extend_from_slice(set);
    }

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

    #[test]
    fn excludes_unsupported_flow_formats_from_analytics() {
        let payload = [0_u8; 32];

        assert!(parse_datagram(FlowProtocol::Ipfix, &payload).is_empty());
        assert!(parse_datagram(FlowProtocol::SFlow, &payload).is_empty());
    }

    #[test]
    fn decodes_netflow_v9_records_after_receiving_a_template() {
        let exporter = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10));
        let templates: TemplateCache = Arc::new(Mutex::new(HashMap::new()));
        let mut template_set = Vec::new();
        append_basic_template(&mut template_set);
        let mut template_message = vec![0; 20];
        template_message[0..2].copy_from_slice(&9_u16.to_be_bytes());
        template_message[16..20].copy_from_slice(&42_u32.to_be_bytes());
        append_set(&mut template_message, 0, &template_set);

        assert!(parse_datagram_with_templates(
            FlowProtocol::NetFlow,
            &template_message,
            exporter,
            &templates,
        )
        .is_empty());

        let mut data_set = Vec::new();
        append_basic_record(&mut data_set);
        let mut data_message = vec![0; 20];
        data_message[0..2].copy_from_slice(&9_u16.to_be_bytes());
        data_message[16..20].copy_from_slice(&42_u32.to_be_bytes());
        append_set(&mut data_message, 256, &data_set);
        let records = parse_datagram_with_templates(
            FlowProtocol::NetFlow,
            &data_message,
            exporter,
            &templates,
        );

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source_ip, "192.0.2.1");
        assert_eq!(records[0].destination_ip, "198.51.100.2");
        assert_eq!(records[0].source_port, 1234);
        assert_eq!(records[0].destination_port, 443);
        assert_eq!(records[0].protocol, "TCP");
        assert_eq!(records[0].bytes, 1_000);
        assert_eq!(records[0].packets, 10);
    }

    #[test]
    fn decodes_ipfix_records_after_receiving_a_template() {
        let exporter = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10));
        let templates: TemplateCache = Arc::new(Mutex::new(HashMap::new()));
        let mut template_set = Vec::new();
        append_basic_template(&mut template_set);
        let mut template_message = vec![0; 16];
        template_message[0..2].copy_from_slice(&10_u16.to_be_bytes());
        template_message[12..16].copy_from_slice(&7_u32.to_be_bytes());
        append_set(&mut template_message, 2, &template_set);
        let template_message_len = template_message.len() as u16;
        template_message[2..4].copy_from_slice(&template_message_len.to_be_bytes());

        assert!(parse_datagram_with_templates(
            FlowProtocol::Ipfix,
            &template_message,
            exporter,
            &templates,
        )
        .is_empty());

        let mut data_set = Vec::new();
        append_basic_record(&mut data_set);
        let mut data_message = vec![0; 16];
        data_message[0..2].copy_from_slice(&10_u16.to_be_bytes());
        data_message[12..16].copy_from_slice(&7_u32.to_be_bytes());
        append_set(&mut data_message, 256, &data_set);
        let data_message_len = data_message.len() as u16;
        data_message[2..4].copy_from_slice(&data_message_len.to_be_bytes());
        let records = parse_datagram_with_templates(
            FlowProtocol::Ipfix,
            &data_message,
            exporter,
            &templates,
        );

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].protocol, "TCP");
        assert_eq!(records[0].bytes, 1_000);
        assert_eq!(records[0].packets, 10);
    }
}
