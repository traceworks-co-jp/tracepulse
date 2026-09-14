use crate::db::models::FlowRecord;
use crate::db::repository::Repository;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TEMPLATE_TTL: Duration = Duration::from_secs(30 * 60);
const ORPHAN_TTL: Duration = Duration::from_secs(10);
// One orphan entry represents one received data-set packet awaiting its template.
const ORPHAN_PACKET_LIMIT: usize = 100;
const MEMORY_FLOW_TTL: Duration = Duration::from_secs(600);
const MEMORY_FLOW_LIMIT: usize = 250_000;
const MEMORY_FLOW_BYTES_LIMIT: usize = 512 * 1024 * 1024;

static RECEIVED_DATAGRAMS: AtomicU64 = AtomicU64::new(0);
static PARSED_FLOWS: AtomicU64 = AtomicU64::new(0);
static DROPPED_FLOWS: AtomicU64 = AtomicU64::new(0);
static DB_WRITE_LATENCY_MS: AtomicU64 = AtomicU64::new(0);
static DB_WRITE_COUNT: AtomicU64 = AtomicU64::new(0);
static ORPHAN_PACKET_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn metrics() -> (u64, u64, u64, Option<f64>) {
    (
        RECEIVED_DATAGRAMS.load(Ordering::Relaxed),
        PARSED_FLOWS.load(Ordering::Relaxed),
        DROPPED_FLOWS.load(Ordering::Relaxed),
        match DB_WRITE_COUNT.load(Ordering::Relaxed) {
            0 => None,
            count => Some(DB_WRITE_LATENCY_MS.load(Ordering::Relaxed) as f64 / count as f64),
        },
    )
}

/// Flow persistence/forwarding boundary used by embedded and Enterprise collectors.
pub trait FlowRepository: Send + Sync {
    fn save_flow_record(&self, flow: &FlowRecord) -> Result<(), String>;
    fn aggregate_flow_records_1m(&self, records: &[FlowRecord]) -> Result<(), String>;
    fn protocol_shares_for_context(
        &self,
        _seconds: i64,
        _exporter: Option<&str>,
        _if_index: Option<i32>,
    ) -> Result<Vec<crate::db::models::ProtocolShare>, String> {
        Err("protocol analytics unavailable".to_string())
    }
    fn flow_summary_for_context(
        &self,
        _seconds: i64,
        _exporter: Option<&str>,
        _if_index: Option<i32>,
    ) -> Result<(crate::db::models::FlowSummary, u64), String> {
        Err("flow summary unavailable".to_string())
    }
    fn flow_applications_for_context(
        &self,
        _seconds: i64,
        _limit: usize,
        _exporter: Option<&str>,
        _if_index: Option<i32>,
    ) -> Result<Vec<crate::db::models::FlowApplicationShare>, String> {
        Err("application analytics unavailable".to_string())
    }
    fn top_talkers_for_context(
        &self,
        _seconds: i64,
        _limit: usize,
        _exporter: Option<&str>,
        _if_index: Option<i32>,
        _sort: &str,
    ) -> Result<Vec<crate::db::models::TopTalker>, String> {
        Err("top talker analytics unavailable".to_string())
    }
}

struct LegacyFlowRepository(Arc<Mutex<Repository>>);

impl FlowRepository for Mutex<Repository> {
    fn save_flow_record(&self, flow: &FlowRecord) -> Result<(), String> {
        self.lock()
            .map_err(|_| "SQLite repository lock failed".to_string())?
            .save_flow_record(flow)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    fn aggregate_flow_records_1m(&self, records: &[FlowRecord]) -> Result<(), String> {
        self.lock()
            .map_err(|_| "SQLite repository lock failed".to_string())?
            .aggregate_flow_records_1m(records)
            .map_err(|e| e.to_string())
    }
    fn protocol_shares_for_context(
        &self,
        s: i64,
        e: Option<&str>,
        i: Option<i32>,
    ) -> Result<Vec<crate::db::models::ProtocolShare>, String> {
        self.lock()
            .map_err(|_| "SQLite repository lock failed".to_string())?
            .protocol_shares_for_context(s, e, i)
            .map_err(|x| x.to_string())
    }
    fn flow_summary_for_context(
        &self,
        s: i64,
        e: Option<&str>,
        i: Option<i32>,
    ) -> Result<(crate::db::models::FlowSummary, u64), String> {
        self.lock()
            .map_err(|_| "SQLite repository lock failed".to_string())?
            .flow_summary_for_context(s, e, i)
            .map_err(|x| x.to_string())
    }
    fn flow_applications_for_context(
        &self,
        s: i64,
        l: usize,
        e: Option<&str>,
        i: Option<i32>,
    ) -> Result<Vec<crate::db::models::FlowApplicationShare>, String> {
        self.lock()
            .map_err(|_| "SQLite repository lock failed".to_string())?
            .flow_applications_for_context(s, l, e, i)
            .map_err(|x| x.to_string())
    }
    fn top_talkers_for_context(
        &self,
        s: i64,
        l: usize,
        e: Option<&str>,
        i: Option<i32>,
        k: &str,
    ) -> Result<Vec<crate::db::models::TopTalker>, String> {
        self.lock()
            .map_err(|_| "SQLite repository lock failed".to_string())?
            .top_talkers_for_context(s, l, e, i, k)
            .map_err(|x| x.to_string())
    }
}

impl FlowRepository for LegacyFlowRepository {
    fn save_flow_record(&self, flow: &FlowRecord) -> Result<(), String> {
        self.0.save_flow_record(flow)
    }
    fn aggregate_flow_records_1m(&self, records: &[FlowRecord]) -> Result<(), String> {
        self.0.aggregate_flow_records_1m(records)
    }
    fn protocol_shares_for_context(
        &self,
        s: i64,
        e: Option<&str>,
        i: Option<i32>,
    ) -> Result<Vec<crate::db::models::ProtocolShare>, String> {
        self.0.protocol_shares_for_context(s, e, i)
    }
    fn flow_summary_for_context(
        &self,
        s: i64,
        e: Option<&str>,
        i: Option<i32>,
    ) -> Result<(crate::db::models::FlowSummary, u64), String> {
        self.0.flow_summary_for_context(s, e, i)
    }
    fn flow_applications_for_context(
        &self,
        s: i64,
        l: usize,
        e: Option<&str>,
        i: Option<i32>,
    ) -> Result<Vec<crate::db::models::FlowApplicationShare>, String> {
        self.0.flow_applications_for_context(s, l, e, i)
    }
    fn top_talkers_for_context(
        &self,
        s: i64,
        l: usize,
        e: Option<&str>,
        i: Option<i32>,
        k: &str,
    ) -> Result<Vec<crate::db::models::TopTalker>, String> {
        self.0.top_talkers_for_context(s, l, e, i, k)
    }
}

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

#[derive(Debug, Clone)]
struct OrphanPacket {
    packet_id: u64,
    format: TemplateFormat,
    exporter: IpAddr,
    observation_domain_id: u32,
    data_sets: Vec<(u16, Vec<u8>)>,
    received_at: Instant,
}

type TemplateCache = Arc<Mutex<HashMap<TemplateKey, CachedTemplate>>>;
type OrphanCache = Arc<Mutex<Vec<OrphanPacket>>>;
type FlowRingBuffer = Arc<Mutex<VecDeque<(Instant, FlowRecord, usize)>>>;
static LIVE_FLOW_RING: OnceLock<FlowRingBuffer> = OnceLock::new();

pub fn live_records(window_seconds: i64) -> Vec<FlowRecord> {
    let Some(ring) = LIVE_FLOW_RING.get() else {
        return Vec::new();
    };
    let Ok(buffer) = ring.lock() else {
        return Vec::new();
    };
    let now = Instant::now();
    let window = Duration::from_secs(window_seconds.max(1) as u64);
    buffer
        .iter()
        .filter(|(received_at, _, _)| now.duration_since(*received_at) <= window)
        .map(|(_, record, _)| record.clone())
        .collect()
}

pub fn live_memory_bytes() -> usize {
    LIVE_FLOW_RING
        .get()
        .and_then(|ring| ring.lock().ok())
        .map(|buffer| buffer.iter().map(|(_, _, size)| *size).sum())
        .unwrap_or(0)
}

pub fn process_memory_bytes() -> Option<u64> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let result =
            unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
        if result != 0 {
            Some(counters.WorkingSetSize as u64)
        } else {
            None
        }
    }
    #[cfg(target_family = "unix")]
    {
        let page_size = 4096_u64;
        let pages = std::fs::read_to_string("/proc/self/statm")
            .ok()?
            .split_whitespace()
            .nth(1)?
            .parse::<u64>()
            .ok()?;
        Some(pages.saturating_mul(page_size))
    }
    #[cfg(not(any(target_os = "windows", target_family = "unix")))]
    {
        None
    }
}

fn estimated_flow_bytes(flow: &FlowRecord) -> usize {
    std::mem::size_of::<FlowRecord>()
        + flow.source_ip.len()
        + flow.destination_ip.len()
        + flow.protocol.len()
        + flow.observed_at.len()
        + 64
}

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

impl FlowCollectorConfig {
    pub fn from_app_config(config: &crate::config::AppConfig) -> Self {
        let bind_addr = config
            .flow
            .bind_addr
            .parse::<Ipv4Addr>()
            .unwrap_or(Ipv4Addr::UNSPECIFIED);
        Self {
            bind_addr: bind_addr.octets(),
            netflow_port: config.flow.netflow_port,
            ipfix_port: config.flow.ipfix_port,
            sflow_port: config.flow.sflow_port,
        }
    }
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
        Self::start_with_repository(Arc::new(LegacyFlowRepository(repository)), config);
    }

    pub fn start_with_repository(repository: Arc<dyn FlowRepository>, config: FlowCollectorConfig) {
        let templates: TemplateCache = Arc::new(Mutex::new(HashMap::new()));
        let orphans: OrphanCache = Arc::new(Mutex::new(Vec::new()));
        let ring: FlowRingBuffer = Arc::new(Mutex::new(VecDeque::new()));
        let _ = LIVE_FLOW_RING.set(Arc::clone(&ring));
        let rollup_repository = Arc::clone(&repository);
        let rollup_ring = Arc::clone(&ring);
        thread::spawn(move || run_rollup_loop(rollup_repository, rollup_ring));
        for protocol in [
            FlowProtocol::NetFlow,
            FlowProtocol::Ipfix,
            FlowProtocol::SFlow,
        ] {
            let repo = Arc::clone(&repository);
            let cfg = config;
            let templates = Arc::clone(&templates);
            let orphans = Arc::clone(&orphans);
            let ring = Arc::clone(&ring);
            thread::spawn(move || run_listener(protocol, repo, cfg, templates, orphans, ring));
        }
    }
}

fn run_rollup_loop(repository: Arc<dyn FlowRepository>, ring: FlowRingBuffer) {
    let mut watermark = Instant::now();
    loop {
        thread::sleep(Duration::from_secs(60));
        rollup_ring_once(&repository, &ring, &mut watermark);
    }
}

fn rollup_ring_once<R: FlowRepository + ?Sized>(
    repository: &Arc<R>,
    ring: &FlowRingBuffer,
    watermark: &mut Instant,
) {
    let now = Instant::now();
    let records = ring
        .lock()
        .map(|buffer| {
            buffer
                .iter()
                .filter(|(received_at, _, _)| *received_at > *watermark && *received_at <= now)
                .map(|(_, record, _)| record.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    *watermark = now;
    if records.is_empty() {
        return;
    }
    if let Err(error) = repository.aggregate_flow_records_1m(&records) {
        tracing::error!("flow 1m rollup failed: {}", error);
    }
}

fn run_listener(
    protocol: FlowProtocol,
    repository: Arc<dyn FlowRepository>,
    config: FlowCollectorConfig,
    templates: TemplateCache,
    orphans: OrphanCache,
    ring: FlowRingBuffer,
) {
    let port = match protocol {
        FlowProtocol::NetFlow => config.netflow_port,
        FlowProtocol::Ipfix => config.ipfix_port,
        FlowProtocol::SFlow => config.sflow_port,
    };
    let socket = match bind_flow_socket(config.bind_addr, port) {
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
                RECEIVED_DATAGRAMS.fetch_add(1, Ordering::Relaxed);
                let records = parse_datagram_with_templates(
                    protocol,
                    &buffer[..length],
                    exporter.ip(),
                    &templates,
                    &orphans,
                );
                PARSED_FLOWS.fetch_add(records.len() as u64, Ordering::Relaxed);
                for record in records {
                    if let Ok(mut buffer) = ring.lock() {
                        let now = Instant::now();
                        buffer.retain(|(received_at, _, _)| {
                            now.duration_since(*received_at) <= MEMORY_FLOW_TTL
                        });
                        let estimated = estimated_flow_bytes(&record);
                        buffer.push_back((now, record.clone(), estimated));
                        let mut total_bytes: usize = buffer.iter().map(|(_, _, size)| *size).sum();
                        while buffer.len() > MEMORY_FLOW_LIMIT
                            || total_bytes > MEMORY_FLOW_BYTES_LIMIT
                        {
                            if let Some((_, _, size)) = buffer.pop_front() {
                                total_bytes = total_bytes.saturating_sub(size);
                            } else {
                                break;
                            }
                        }
                    }
                    let started = Instant::now();
                    let save_result = repository.save_flow_record(&record);
                    DB_WRITE_LATENCY_MS
                        .fetch_add(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                    DB_WRITE_COUNT.fetch_add(1, Ordering::Relaxed);
                    if save_result.is_err() {
                        DROPPED_FLOWS.fetch_add(1, Ordering::Relaxed);
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

fn bind_flow_socket(bind_addr: [u8; 4], port: u16) -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    if let Err(error) = socket.set_recv_buffer_size(8 * 1024 * 1024) {
        tracing::warn!(
            port,
            "could not set flow UDP receive buffer to 8 MiB: {}",
            error
        );
    }
    socket.set_reuse_address(true)?;
    socket.bind(&std::net::SocketAddr::from((Ipv4Addr::from(bind_addr), port)).into())?;
    Ok(socket.into())
}

#[cfg(test)]
fn parse_datagram(protocol: FlowProtocol, payload: &[u8]) -> Vec<FlowRecord> {
    let templates: TemplateCache = Arc::new(Mutex::new(HashMap::new()));
    let orphans: OrphanCache = Arc::new(Mutex::new(Vec::new()));
    parse_datagram_with_templates(
        protocol,
        payload,
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        &templates,
        &orphans,
    )
}

fn parse_datagram_with_templates(
    protocol: FlowProtocol,
    payload: &[u8],
    exporter: IpAddr,
    templates: &TemplateCache,
    orphans: &OrphanCache,
) -> Vec<FlowRecord> {
    let observed_at = chrono::Utc::now().to_rfc3339();
    match protocol {
        FlowProtocol::NetFlow => match read_u16(payload, 0) {
            Some(5) => parse_netflow_v5(payload, exporter, &observed_at),
            Some(9) => parse_template_flow_message(
                TemplateFormat::NetFlowV9,
                payload,
                exporter,
                templates,
                &observed_at,
                orphans,
            ),
            _ => Vec::new(),
        },
        FlowProtocol::Ipfix => parse_template_flow_message(
            TemplateFormat::Ipfix,
            payload,
            exporter,
            templates,
            &observed_at,
            orphans,
        ),
        FlowProtocol::SFlow => parse_sflow_v5(payload, exporter, &observed_at),
    }
}

fn parse_sflow_v5(payload: &[u8], exporter: IpAddr, observed_at: &str) -> Vec<FlowRecord> {
    if read_u32(payload, 0) != Some(5) || payload.len() < 8 {
        return Vec::new();
    }
    let address_type = read_u32(payload, 4).unwrap_or_default();
    let address_len = match address_type {
        1 => 4,
        2 => 16,
        _ => return Vec::new(),
    };
    let mut offset = 8 + address_len;
    if offset + 16 > payload.len() {
        return Vec::new();
    }
    offset += 12;
    let sample_count = read_u32(payload, offset).unwrap_or_default() as usize;
    offset += 4;
    let mut records = Vec::new();
    for _ in 0..sample_count {
        if offset + 8 > payload.len() {
            break;
        }
        let sample_type = read_u32(payload, offset).unwrap_or_default();
        let sample_len = read_u32(payload, offset + 4).unwrap_or_default() as usize;
        offset += 8;
        if offset + sample_len > payload.len() {
            break;
        }
        let sample = &payload[offset..offset + sample_len];
        offset += sample_len;
        let format = sample_type & 0x0fff;
        let enterprise = sample_type >> 12;
        if enterprise != 0 || (format != 1 && format != 3) {
            continue;
        }
        records.extend(parse_sflow_flow_sample(
            sample,
            format,
            exporter,
            observed_at,
        ));
    }
    records
}

fn parse_sflow_flow_sample(
    sample: &[u8],
    format: u32,
    exporter: IpAddr,
    observed_at: &str,
) -> Vec<FlowRecord> {
    let (mut offset, sampling_rate, input, output, record_count) = if format == 1 {
        if sample.len() < 32 {
            return Vec::new();
        }
        let rate = read_u32(sample, 8).unwrap_or(1).max(1);
        let input = read_u32(sample, 20).unwrap_or_default();
        let output = read_u32(sample, 24).unwrap_or_default();
        (
            32,
            rate,
            input,
            output,
            read_u32(sample, 28).unwrap_or_default() as usize,
        )
    } else {
        if sample.len() < 44 {
            return Vec::new();
        }
        let rate = read_u32(sample, 12).unwrap_or(1).max(1);
        let input = read_u32(sample, 28).unwrap_or_default();
        let output = read_u32(sample, 36).unwrap_or_default();
        (
            44,
            rate,
            input,
            output,
            read_u32(sample, 40).unwrap_or_default() as usize,
        )
    };
    let mut records = Vec::new();
    for _ in 0..record_count {
        if offset + 8 > sample.len() {
            break;
        }
        let record_type = read_u32(sample, offset).unwrap_or_default();
        let record_len = read_u32(sample, offset + 4).unwrap_or_default() as usize;
        offset += 8;
        if offset + record_len > sample.len() {
            break;
        }
        if record_type & 0x0fff == 1 && record_type >> 12 == 0 {
            if let Some(record) = decode_sflow_raw_packet(
                &sample[offset..offset + record_len],
                sampling_rate,
                input,
                output,
                exporter,
                observed_at,
            ) {
                records.push(record);
            }
        }
        offset += record_len;
    }
    records
}

fn decode_sflow_raw_packet(
    data: &[u8],
    sampling_rate: u32,
    input: u32,
    output: u32,
    exporter: IpAddr,
    observed_at: &str,
) -> Option<FlowRecord> {
    if data.len() < 16 {
        return None;
    }
    let protocol = read_u32(data, 0)?;
    let frame_length = read_u32(data, 4)? as u64;
    let header_length = read_u32(data, 12)? as usize;
    let header = data.get(16..16 + header_length.min(data.len().saturating_sub(16)))?;
    let (source_ip, destination_ip, source_port, destination_port, ip_protocol, tcp_flags) =
        decode_sflow_header(protocol, header)?;
    Some(FlowRecord {
        exporter_ip: Some(exporter.to_string()),
        source_ip,
        destination_ip,
        source_port,
        destination_port,
        protocol: return_unknown_protocol(ip_protocol).to_string(),
        bytes: frame_length.saturating_mul(sampling_rate as u64),
        packets: sampling_rate as u64,
        ingress_if_index: Some(input),
        egress_if_index: Some(output),
        tcp_flags,
        sampling_rate,
        dscp: 0,
        bgp_next_hop: None,
        observed_at: observed_at.to_string(),
    })
}

fn decode_sflow_header(protocol: u32, header: &[u8]) -> Option<(String, String, u16, u16, u8, u8)> {
    if protocol != 1 || header.len() < 14 {
        return None;
    }
    let mut network_offset = 14;
    let mut ethertype = u16::from_be_bytes([header[12], header[13]]);
    if (ethertype == 0x8100 || ethertype == 0x88a8) && header.len() >= 18 {
        ethertype = u16::from_be_bytes([header[16], header[17]]);
        network_offset = 18;
    }
    if ethertype == 0x0800 {
        let ip = header.get(network_offset..)?;
        if ip.len() < 20 {
            return None;
        }
        let ihl = ((ip[0] & 0x0f) as usize).saturating_mul(4);
        if ihl < 20 || ip.len() < ihl {
            return None;
        }
        let proto = ip[9];
        let source = format!("{}.{}.{}.{}", ip[12], ip[13], ip[14], ip[15]);
        let destination = format!("{}.{}.{}.{}", ip[16], ip[17], ip[18], ip[19]);
        return decode_sflow_transport(&ip[ihl..], proto)
            .map(|(src, dst, flags)| (source, destination, src, dst, proto, flags));
    }
    if ethertype == 0x86dd {
        let ip = header.get(network_offset..)?;
        if ip.len() < 40 {
            return None;
        }
        let proto = ip[6];
        let source = std::net::Ipv6Addr::from(<[u8; 16]>::try_from(&ip[8..24]).ok()?).to_string();
        let destination =
            std::net::Ipv6Addr::from(<[u8; 16]>::try_from(&ip[24..40]).ok()?).to_string();
        return decode_sflow_transport(&ip[40..], proto)
            .map(|(src, dst, flags)| (source, destination, src, dst, proto, flags));
    }
    None
}

fn decode_sflow_transport(data: &[u8], protocol: u8) -> Option<(u16, u16, u8)> {
    if protocol == 6 && data.len() >= 14 {
        return Some((
            u16::from_be_bytes([data[0], data[1]]),
            u16::from_be_bytes([data[2], data[3]]),
            data[13],
        ));
    }
    if protocol == 17 && data.len() >= 4 {
        return Some((
            u16::from_be_bytes([data[0], data[1]]),
            u16::from_be_bytes([data[2], data[3]]),
            0,
        ));
    }
    Some((0, 0, 0))
}

fn parse_template_flow_message(
    format: TemplateFormat,
    payload: &[u8],
    exporter: IpAddr,
    templates: &TemplateCache,
    observed_at: &str,
    orphans: &OrphanCache,
) -> Vec<FlowRecord> {
    let packet_id = ORPHAN_PACKET_SEQUENCE.fetch_add(1, Ordering::Relaxed);
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
            records.extend(replay_orphans(
                format,
                exporter,
                observation_domain_id,
                templates,
                orphans,
                observed_at,
            ));
        } else if set_id >= 256 {
            records.extend(decode_data_set(
                format,
                exporter,
                observation_domain_id,
                set_id,
                set,
                templates,
                observed_at,
                orphans,
                packet_id,
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
            let mut scoped: Vec<(TemplateKey, Instant)> = cache
                .iter()
                .filter(|(key, _)| {
                    key.format == format
                        && key.exporter == exporter
                        && key.observation_domain_id == observation_domain_id
                })
                .map(|(key, template)| (*key, template.received_at))
                .collect();
            if scoped.len() > 256 {
                scoped.sort_by_key(|(_, received_at)| *received_at);
                let excess = scoped.len() - 256;
                for (key, _) in scoped.into_iter().take(excess) {
                    cache.remove(&key);
                }
            }
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
    orphans: &OrphanCache,
    packet_id: u64,
) -> Vec<FlowRecord> {
    let key = TemplateKey {
        format,
        exporter,
        observation_domain_id,
        template_id,
    };
    let fields = templates.lock().ok().and_then(|mut cache| {
        cache.retain(|_, template| template.received_at.elapsed() <= TEMPLATE_TTL);
        cache.get(&key).map(|template| template.fields.clone())
    });
    let Some(fields) = fields else {
        if let Ok(mut cache) = orphans.lock() {
            cache.retain(|item| item.received_at.elapsed() <= ORPHAN_TTL);
            if let Some(packet) = cache.iter_mut().find(|item| item.packet_id == packet_id) {
                packet.data_sets.push((template_id, set.to_vec()));
            } else {
                cache.push(OrphanPacket {
                    packet_id,
                    format,
                    exporter,
                    observation_domain_id,
                    data_sets: vec![(template_id, set.to_vec())],
                    received_at: Instant::now(),
                });
            }
            while cache
                .iter()
                .filter(|item| {
                    item.format == format
                        && item.exporter == exporter
                        && item.observation_domain_id == observation_domain_id
                })
                .count()
                > ORPHAN_PACKET_LIMIT
            {
                if let Some(oldest_packet_id) = cache
                    .iter()
                    .filter(|item| {
                        item.format == format
                            && item.exporter == exporter
                            && item.observation_domain_id == observation_domain_id
                    })
                    .map(|item| item.packet_id)
                    .min()
                {
                    cache.retain(|item| item.packet_id != oldest_packet_id);
                } else {
                    break;
                }
            }
        }
        return Vec::new();
    };

    let mut records = Vec::new();
    let mut offset = 0;
    while offset < set.len() {
        let Some((record, length)) = decode_record(&fields, &set[offset..], exporter, observed_at)
        else {
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

fn replay_orphans(
    format: TemplateFormat,
    exporter: IpAddr,
    observation_domain_id: u32,
    templates: &TemplateCache,
    orphans: &OrphanCache,
    observed_at: &str,
) -> Vec<FlowRecord> {
    let mut records = Vec::new();
    let pending = if let Ok(mut cache) = orphans.lock() {
        let mut matching = Vec::new();
        cache.retain(|item| {
            if item.received_at.elapsed() > ORPHAN_TTL {
                return false;
            }
            if item.format == format
                && item.exporter == exporter
                && item.observation_domain_id == observation_domain_id
            {
                matching.push(item.clone());
                false
            } else {
                true
            }
        });
        matching
    } else {
        Vec::new()
    };
    for item in pending {
        for (template_id, data) in item.data_sets {
            records.extend(decode_data_set(
                format,
                exporter,
                observation_domain_id,
                template_id,
                &data,
                templates,
                observed_at,
                orphans,
                item.packet_id,
            ));
        }
    }
    records
}

fn decode_record(
    fields: &[TemplateField],
    data: &[u8],
    exporter: IpAddr,
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
    let mut ingress_if_index = None;
    let mut egress_if_index = None;
    let mut tcp_flags = 0;
    let mut sampling_rate = 1_u32;
    let mut dscp = 0_u8;
    let mut bgp_next_hop = None;

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
            6 => tcp_flags = decode_number(value).unwrap_or_default().min(u8::MAX as u64) as u8,
            18 => bgp_next_hop = decode_ip_addr(value),
            195 => dscp = decode_number(value).unwrap_or_default().min(u8::MAX as u64) as u8,
            10 => ingress_if_index = Some(decode_number(value).unwrap_or_default() as u32),
            14 => egress_if_index = Some(decode_number(value).unwrap_or_default() as u32),
            34 | 305 => {
                sampling_rate = decode_number(value)
                    .unwrap_or(1)
                    .max(1)
                    .min(u32::MAX as u64) as u32
            }
            _ => {}
        }
    }

    let record = match (source_ip, destination_ip, protocol) {
        (Some(source_ip), Some(destination_ip), Some(protocol)) => Some(FlowRecord {
            exporter_ip: Some(exporter.to_string()),
            source_ip,
            destination_ip,
            source_port,
            destination_port,
            protocol: protocol.to_string(),
            bytes: bytes.saturating_mul(sampling_rate as u64),
            packets: packets.saturating_mul(sampling_rate as u64),
            ingress_if_index,
            egress_if_index,
            tcp_flags,
            sampling_rate,
            dscp,
            bgp_next_hop,
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

fn decode_ip_addr(value: &[u8]) -> Option<IpAddr> {
    match value.len() {
        4 => Some(IpAddr::V4(Ipv4Addr::new(
            value[0], value[1], value[2], value[3],
        ))),
        16 => {
            let mut octets = [0_u8; 16];
            octets.copy_from_slice(value);
            Some(IpAddr::V6(std::net::Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}

fn decode_number(value: &[u8]) -> Option<u64> {
    if value.is_empty() || value.len() > 8 {
        return None;
    }
    Some(
        value
            .iter()
            .fold(0_u64, |number, byte| (number << 8) | u64::from(*byte)),
    )
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        data.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn parse_netflow_v5(payload: &[u8], exporter: IpAddr, observed_at: &str) -> Vec<FlowRecord> {
    if payload.len() < 24 || u16::from_be_bytes([payload[0], payload[1]]) != 5 {
        return Vec::new();
    }
    let count = u16::from_be_bytes([payload[2], payload[3]]) as usize;
    let sampling_rate = if payload.len() >= 24 {
        (u16::from_be_bytes([payload[22], payload[23]]) & 0x3fff).max(1) as u32
    } else {
        1
    };
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
            exporter_ip: Some(exporter.to_string()),
            source_ip,
            destination_ip,
            source_port,
            destination_port,
            protocol: protocol.to_string(),
            bytes: bytes.saturating_mul(sampling_rate as u64),
            packets: packets.saturating_mul(sampling_rate as u64),
            ingress_if_index: Some(
                u16::from_be_bytes([payload[offset + 12], payload[offset + 13]]) as u32,
            ),
            egress_if_index: Some(
                u16::from_be_bytes([payload[offset + 14], payload[offset + 15]]) as u32,
            ),
            tcp_flags: payload[offset + 37],
            sampling_rate,
            dscp: 0,
            bgp_next_hop: None,
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
    use crate::db::repository::Repository;
    use crate::db::sqlite::initialize_database;

    const BASIC_FIELDS: &[(u16, u16)] = &[(8, 4), (12, 4), (7, 2), (11, 2), (4, 1), (1, 4), (2, 4)];

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
        let records = parse_netflow_v5(&packet, IpAddr::V4(Ipv4Addr::UNSPECIFIED), "now");
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
        let orphans: OrphanCache = Arc::new(Mutex::new(Vec::new()));
        let mut template_set = Vec::new();
        append_basic_template(&mut template_set);
        let mut template_message = vec![0; 20];
        template_message[0..2].copy_from_slice(&9_u16.to_be_bytes());
        template_message[16..20].copy_from_slice(&42_u32.to_be_bytes());
        append_set(&mut template_message, 0, &template_set);

        let mut data_set = Vec::new();
        append_basic_record(&mut data_set);
        let mut data_message = vec![0; 20];
        data_message[0..2].copy_from_slice(&9_u16.to_be_bytes());
        data_message[16..20].copy_from_slice(&42_u32.to_be_bytes());
        append_set(&mut data_message, 256, &data_set);

        assert!(
            parse_datagram_with_templates(
                FlowProtocol::NetFlow,
                &data_message,
                exporter,
                &templates,
                &orphans,
            )
            .is_empty()
        );

        let replayed = parse_datagram_with_templates(
            FlowProtocol::NetFlow,
            &template_message,
            exporter,
            &templates,
            &orphans,
        );

        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0].source_ip, "192.0.2.1");
        assert_eq!(replayed[0].destination_ip, "198.51.100.2");
        assert_eq!(replayed[0].source_port, 1234);
        assert_eq!(replayed[0].destination_port, 443);
        assert_eq!(replayed[0].protocol, "TCP");
        assert_eq!(replayed[0].bytes, 1_000);
        assert_eq!(replayed[0].packets, 10);
    }

    #[test]
    fn decodes_ipfix_records_after_receiving_a_template() {
        let exporter = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10));
        let templates: TemplateCache = Arc::new(Mutex::new(HashMap::new()));
        let orphans: OrphanCache = Arc::new(Mutex::new(Vec::new()));
        let mut template_set = Vec::new();
        append_basic_template(&mut template_set);
        let mut template_message = vec![0; 16];
        template_message[0..2].copy_from_slice(&10_u16.to_be_bytes());
        template_message[12..16].copy_from_slice(&7_u32.to_be_bytes());
        append_set(&mut template_message, 2, &template_set);
        let template_message_len = template_message.len() as u16;
        template_message[2..4].copy_from_slice(&template_message_len.to_be_bytes());

        assert!(
            parse_datagram_with_templates(
                FlowProtocol::Ipfix,
                &template_message,
                exporter,
                &templates,
                &orphans,
            )
            .is_empty()
        );

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
            &orphans,
        );

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].protocol, "TCP");
        assert_eq!(records[0].bytes, 1_000);
        assert_eq!(records[0].packets, 10);
    }

    #[test]
    fn decodes_sflow_ipv4_tcp_header() {
        let mut ethernet = vec![0u8; 14 + 20 + 20];
        ethernet[12..14].copy_from_slice(&0x0800_u16.to_be_bytes());
        let ip = &mut ethernet[14..34];
        ip[0] = 0x45;
        ip[9] = 6;
        ip[12..16].copy_from_slice(&[192, 0, 2, 1]);
        ip[16..20].copy_from_slice(&[198, 51, 100, 2]);
        let tcp = &mut ethernet[34..54];
        tcp[0..2].copy_from_slice(&1234_u16.to_be_bytes());
        tcp[2..4].copy_from_slice(&443_u16.to_be_bytes());
        tcp[12] = 0x50;
        tcp[13] = 0x12;
        let mut raw = vec![0u8; 16];
        raw[0..4].copy_from_slice(&1_u32.to_be_bytes());
        raw[4..8].copy_from_slice(&(ethernet.len() as u32).to_be_bytes());
        raw[12..16].copy_from_slice(&(ethernet.len() as u32).to_be_bytes());
        raw.extend_from_slice(&ethernet);
        let record =
            decode_sflow_raw_packet(&raw, 2, 7, 8, IpAddr::V4(Ipv4Addr::UNSPECIFIED), "now")
                .expect("sFlow record");
        assert_eq!(record.source_ip, "192.0.2.1");
        assert_eq!(record.destination_ip, "198.51.100.2");
        assert_eq!(record.source_port, 1234);
        assert_eq!(record.destination_port, 443);
        assert_eq!(record.tcp_flags, 0x12);
        assert_eq!(record.sampling_rate, 2);
        assert_eq!(record.bytes, (ethernet.len() as u64) * 2);
        assert_eq!(record.ingress_if_index, Some(7));
        assert_eq!(record.egress_if_index, Some(8));
    }

    #[test]
    fn applies_template_sampling_and_interface_fields() {
        let fields = vec![
            TemplateField {
                information_element: 8,
                length: 4,
            },
            TemplateField {
                information_element: 12,
                length: 4,
            },
            TemplateField {
                information_element: 4,
                length: 1,
            },
            TemplateField {
                information_element: 1,
                length: 4,
            },
            TemplateField {
                information_element: 2,
                length: 4,
            },
            TemplateField {
                information_element: 6,
                length: 1,
            },
            TemplateField {
                information_element: 10,
                length: 2,
            },
            TemplateField {
                information_element: 14,
                length: 2,
            },
            TemplateField {
                information_element: 34,
                length: 2,
            },
            TemplateField {
                information_element: 18,
                length: 4,
            },
            TemplateField {
                information_element: 195,
                length: 1,
            },
        ];
        let mut data = Vec::new();
        data.extend_from_slice(&[192, 0, 2, 1]);
        data.extend_from_slice(&[198, 51, 100, 2]);
        data.push(6);
        data.extend_from_slice(&100_u32.to_be_bytes());
        data.extend_from_slice(&10_u32.to_be_bytes());
        data.push(0x12);
        data.extend_from_slice(&7_u16.to_be_bytes());
        data.extend_from_slice(&8_u16.to_be_bytes());
        data.extend_from_slice(&2_u16.to_be_bytes());
        data.extend_from_slice(&[203, 0, 113, 1]);
        data.push(46);
        let (record, consumed) =
            decode_record(&fields, &data, IpAddr::V4(Ipv4Addr::UNSPECIFIED), "now")
                .expect("record decode");
        let record = record.expect("flow record");
        assert_eq!(consumed, data.len());
        assert_eq!(record.bytes, 200);
        assert_eq!(record.packets, 20);
        assert_eq!(record.tcp_flags, 0x12);
        assert_eq!(record.ingress_if_index, Some(7));
        assert_eq!(record.egress_if_index, Some(8));
        assert_eq!(record.sampling_rate, 2);
        assert_eq!(record.dscp, 46);
        assert_eq!(
            record.bgp_next_hop,
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)))
        );
    }

    #[test]
    fn rolls_ring_records_into_protocol_minute_aggregate() {
        let path =
            std::env::temp_dir().join(format!("tracepulse-flow-rollup-{}.db", std::process::id()));
        let repository = Arc::new(Mutex::new(Repository::new(
            initialize_database(&path).expect("database should initialize"),
        )));
        let observed_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let record = FlowRecord {
            exporter_ip: None,
            source_ip: "192.0.2.1".to_string(),
            destination_ip: "198.51.100.2".to_string(),
            source_port: 1234,
            destination_port: 443,
            protocol: "TCP".to_string(),
            bytes: 1_000,
            packets: 10,
            ingress_if_index: None,
            egress_if_index: None,
            tcp_flags: 0,
            sampling_rate: 1,
            dscp: 0,
            bgp_next_hop: None,
            observed_at,
        };
        let ring: FlowRingBuffer =
            Arc::new(Mutex::new(VecDeque::from([(Instant::now(), record, 128)])));
        let mut watermark = Instant::now() - Duration::from_secs(1);
        rollup_ring_once(&repository, &ring, &mut watermark);

        let shares = repository
            .lock()
            .expect("repository lock")
            .protocol_shares(3600)
            .expect("protocol shares");
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].protocol, "TCP");
        assert_eq!(shares[0].bytes, 1_000);
        assert_eq!(shares[0].pps, 10.0 / 3600.0);
        let _ = std::fs::remove_file(path);
    }
}
