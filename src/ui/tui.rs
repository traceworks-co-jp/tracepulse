use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, TableState, Tabs, Wrap},
};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::alert::event::AlertEvent;
use crate::app::AppRunner;
use crate::db::models::{Device, DeviceMetrics, InterfacePortDelta, RecentAlert, TopTalker};
use crate::db::repository::Repository;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::monitor::{PollingEngine, calculate_bandwidth_utilization_from_delta};
use crate::notifications::{NotificationSettingsProvider, NotificationStatus};
use crate::snmp::client::SnmpClient;

pub struct TuiRenderer {
    runner: AppRunner,
}

/// フォーカス中のペイン（Tab キーで切替）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Devices,
    Interfaces,
    Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiTab {
    Dashboard,
    Devices,
    Interfaces,
    Traffic,
    Alerts,
}

impl TuiTab {
    const LABELS: [&'static str; 5] = ["Dashboard", "Devices", "Interfaces", "Traffic", "Alerts"];
    const ALL: [Self; 5] = [
        Self::Dashboard,
        Self::Devices,
        Self::Interfaces,
        Self::Traffic,
        Self::Alerts,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0)
    }

    fn from_key(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Char('1') => Some(Self::Dashboard),
            KeyCode::Char('2') => Some(Self::Devices),
            KeyCode::Char('3') => Some(Self::Interfaces),
            KeyCode::Char('4') => Some(Self::Traffic),
            KeyCode::Char('5') => Some(Self::Alerts),
            _ => None,
        }
    }

    fn from_mouse(column: u16, row: u16, area: Rect) -> Option<Self> {
        if area.height < 3 || row != area.y.saturating_add(1) {
            return None;
        }
        let right = area.right().saturating_sub(1);
        let mut start = area.x.saturating_add(1);
        for (index, label) in Self::LABELS.iter().enumerate() {
            let end = start.saturating_add(label.len() as u16 + 2).min(right);
            if column >= start && column < end {
                return Some(Self::ALL[index]);
            }
            start = end.saturating_add(1);
        }
        None
    }

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProtocolSort {
    Bps,
    Pps,
    Bytes,
}

impl ProtocolSort {
    fn next(self) -> Self {
        match self {
            Self::Bps => Self::Pps,
            Self::Pps => Self::Bytes,
            Self::Bytes => Self::Bps,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Bps => "Bps",
            Self::Pps => "PPS",
            Self::Bytes => "Bytes",
        }
    }

    fn query_key(self) -> &'static str {
        match self {
            Self::Bps => "bps",
            Self::Pps => "pps",
            Self::Bytes => "bytes",
        }
    }
}

/// [Enter] キーで開く破損パケット内訳（EtherLike-MIB Error Breakdown）ポップアップの状態
#[derive(Debug, Clone, Default)]
pub struct PortErrorBreakdownState {
    pub if_name: String,
    pub if_index: i32,
    pub fcs_errors_delta: u64,
    pub alignment_errors_delta: u64,
    pub frame_too_longs_delta: u64,
    pub internal_mac_receive_errors_delta: u64,
}

impl PortErrorBreakdownState {
    fn from_delta(if_name: String, if_index: i32, delta: InterfacePortDelta) -> Self {
        Self {
            if_name,
            if_index,
            fcs_errors_delta: delta.fcs_errors_delta,
            alignment_errors_delta: delta.alignment_errors_delta,
            frame_too_longs_delta: delta.frame_too_longs_delta,
            internal_mac_receive_errors_delta: delta.internal_mac_receive_errors_delta,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveryModalState {
    pub active_field: usize, // 0: CIDR, 1: Community, 2: Version
    pub cidr: String,
    pub community: String,
    pub version: String,
    pub cursor_position: usize,
    pub error_msg: Option<String>,
}

impl DiscoveryModalState {
    pub fn current_field_mut(&mut self) -> &mut String {
        match self.active_field {
            0 => &mut self.cidr,
            1 => &mut self.community,
            2 => &mut self.version,
            _ => &mut self.cidr,
        }
    }

    pub fn current_field_ref(&self) -> &str {
        match self.active_field {
            0 => &self.cidr,
            1 => &self.community,
            2 => &self.version,
            _ => &self.cidr,
        }
    }

    pub fn toggle_version(&mut self, next: bool) {
        let versions = ["v2c", "v1", "v3"];
        let current_idx = versions
            .iter()
            .position(|&v| v == self.version)
            .unwrap_or(0);
        let new_idx = if next {
            (current_idx + 1) % versions.len()
        } else {
            (current_idx + versions.len() - 1) % versions.len()
        };
        self.version = versions[new_idx].to_string();
    }

    pub fn move_cursor_left(&mut self) {
        if self.active_field == 2 {
            self.toggle_version(false);
        } else if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.active_field == 2 {
            self.toggle_version(true);
        } else {
            let len = self.current_field_ref().chars().count();
            if self.cursor_position < len {
                self.cursor_position += 1;
            }
        }
    }

    pub fn insert_char(&mut self, c: char) {
        if self.active_field == 2 {
            match c {
                '1' => self.version = "v1".to_string(),
                '2' => self.version = "v2c".to_string(),
                '3' => self.version = "v3".to_string(),
                ' ' => self.toggle_version(true),
                _ => {}
            }
        } else {
            let pos = self.cursor_position;
            let s = self.current_field_mut();
            let byte_pos = s.char_indices().nth(pos).map(|(i, _)| i).unwrap_or(s.len());
            s.insert(byte_pos, c);
            self.cursor_position += 1;
        }
    }

    pub fn delete_backspace(&mut self) {
        if self.active_field != 2 && self.cursor_position > 0 {
            let pos = self.cursor_position - 1;
            let s = self.current_field_mut();
            if let Some((byte_pos, _)) = s.char_indices().nth(pos) {
                s.remove(byte_pos);
                self.cursor_position -= 1;
            }
        }
    }

    pub fn delete_char(&mut self) {
        if self.active_field != 2 {
            let pos = self.cursor_position;
            let s = self.current_field_mut();
            if let Some((byte_pos, _)) = s.char_indices().nth(pos) {
                s.remove(byte_pos);
            }
        }
    }

    pub fn set_field(&mut self, new_field: usize) {
        self.active_field = new_field % 3;
        self.cursor_position = self.current_field_ref().chars().count();
    }
}

#[derive(Clone)]
pub struct ScanTask {
    pub total: usize,
    pub scanned: Arc<AtomicUsize>,
    pub found: Arc<Mutex<Vec<DeviceConfig>>>,
    pub finished: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct PollTask {
    pub total: usize,
    pub polled: Arc<AtomicUsize>,
    pub finished: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct NotificationTestTask {
    pub channel: String,
    pub finished: Arc<AtomicBool>,
    pub result: Arc<Mutex<Option<Result<String, String>>>>,
}

/// [n] キーで開く通知設定の確認・テスト送信ポップアップの状態。編集は Web GUI / config.toml 側で行う。
pub struct NotificationStatusState {
    pub status: NotificationStatus,
    pub selected: usize,
    pub test: Option<NotificationTestTask>,
    pub message: Option<String>,
    pub message_is_error: bool,
}

pub enum TuiMode {
    Normal,
    FilterInput(String),
    FlowInspector(TopTalker),
    AlertDetails(RecentAlert),
    DiscoveryModal(DiscoveryModalState),
    Scanning(ScanTask),
    Polling(PollTask),
    PortErrorBreakdown(PortErrorBreakdownState),
    Notifications(NotificationStatusState),
}

pub struct DeviceSummary {
    pub device: Device,
    pub cpu_usage: Option<u32>,
    pub memory_usage: Option<u32>,
    pub active_ports: usize,
    pub total_ports: usize,
    pub alert_count: usize,
}

pub struct InterfaceSummary {
    pub if_index: i32,
    pub if_name: String,
    pub link_status: String,
    pub bandwidth_utilization: f64,
    pub in_errors: u64,
    pub out_errors: u64,
    pub in_discards: u64,
    pub out_discards: u64,
    pub late_collisions: u64,
    pub neighbor: String,
}

pub fn effective_interface_link_status(device_status: &str, sample_link_status: &str) -> String {
    if device_status.eq_ignore_ascii_case("offline") {
        "down".to_string()
    } else {
        sample_link_status.to_string()
    }
}

fn format_tui_timestamp(value: &str, timezone: &str) -> String {
    let utc = DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .map(|timestamp| Utc.from_utc_datetime(&timestamp))
        });

    match utc {
        Ok(timestamp) if timezone.eq_ignore_ascii_case("jst") => FixedOffset::east_opt(9 * 3600)
            .map(|offset| {
                timestamp
                    .with_timezone(&offset)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| value.to_string()),
        Ok(timestamp) => timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
        Err(_) => value.to_string(),
    }
}

fn format_tui_rate(value: f64) -> String {
    if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format!("{:.0}b", value)
    }
}

fn format_tui_count(value: f64) -> String {
    if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

fn format_tui_bytes(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        format!("{}B", value)
    }
}

fn tui_progress_bar(percentage: f64, width: usize) -> String {
    let filled = ((percentage.clamp(0.0, 100.0) / 100.0) * width as f64).round() as usize;
    format!(
        "{}{}",
        "#".repeat(filled),
        ".".repeat(width.saturating_sub(filled))
    )
}

fn format_tui_flags(flags: u8) -> String {
    let mut names = Vec::new();
    if flags & 0x02 != 0 {
        names.push("S");
    }
    if flags & 0x10 != 0 {
        names.push("A");
    }
    if flags & 0x01 != 0 {
        names.push("F");
    }
    if flags & 0x04 != 0 {
        names.push("R");
    }
    if flags & 0x08 != 0 {
        names.push("P");
    }
    if flags & 0x20 != 0 {
        names.push("U");
    }
    if names.is_empty() {
        "-".to_string()
    } else {
        names.join(",")
    }
}

fn truncate_tui(value: &str, width: usize) -> String {
    let mut result: String = value.chars().take(width).collect();
    if value.chars().count() > width && width >= 2 {
        result.truncate(width - 1);
        result.push('~');
    }
    format!("{result:<width$}")
}

fn flow_header_line() -> String {
    format!(
        "{:<21} {:<21} {:<6} {:>8} {:>8} {:>6} {:<5} {:<12}",
        "SOURCE IP:PORT", "DEST IP:PORT", "PROTO", "BYTES", "BPS", "PPS", "FLAGS", "IN/OUT IF"
    )
}

fn flow_row_line(talker: &TopTalker) -> String {
    let source = truncate_tui(&format!("{}:{}", talker.source_ip, talker.source_port), 21);
    let destination = truncate_tui(
        &format!("{}:{}", talker.destination_ip, talker.destination_port),
        21,
    )
    .trim_end()
    .to_string();
    let ingress = talker
        .ingress_if_name
        .clone()
        .or_else(|| talker.ingress_if_index.map(|value| format!("if-{value}")))
        .unwrap_or_else(|| "-".to_string());
    let egress = talker
        .egress_if_name
        .clone()
        .or_else(|| talker.egress_if_index.map(|value| format!("if-{value}")))
        .unwrap_or_else(|| "-".to_string());
    format!(
        "{:<21} {:<21} {:<6} {:>8} {:>8} {:>6} {:<5} {:<12}",
        source,
        destination,
        talker.protocol.chars().take(6).collect::<String>(),
        format_tui_bytes(talker.bytes),
        format_tui_rate(talker.bps as f64),
        truncate_tui(&talker.pps.to_string(), 6).trim_end(),
        truncate_tui(&format_tui_flags(talker.tcp_flags), 5).trim_end(),
        format!("{}/{}", ingress, egress)
            .chars()
            .take(12)
            .collect::<String>(),
    )
}

fn tui_filter_matches(filter: &str, values: &[&str]) -> bool {
    let needle = filter.trim().to_lowercase();
    needle.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(&needle))
}

fn move_visible_selection(visible: &[usize], selected: usize, forward: bool) -> Option<usize> {
    let position = visible.iter().position(|index| *index == selected);
    let next = match (position, forward) {
        (Some(position), true) => (position + 1).min(visible.len().saturating_sub(1)),
        (Some(position), false) => position.saturating_sub(1),
        (None, _) => 0,
    };
    visible.get(next).copied()
}

fn change_tui_tab(
    active_tab: &mut TuiTab,
    next: TuiTab,
    filter_query: &mut String,
    filters: &mut [String; 5],
) {
    filters[active_tab.index()] = std::mem::take(filter_query);
    *active_tab = next;
    filter_query.clone_from(&filters[next.index()]);
}

fn tui_text(language: &str, key: &str) -> &'static str {
    if language.eq_ignore_ascii_case("ja") {
        match key {
            "device_list" => "デバイス一覧",
            "interface_topology" => "インターフェース / トポロジー",
            "protocol_traffic" => "プロトコルとトラフィック",
            "alert_stream" => "アラートストリーム",
            "top_talkers" => "上位通信フロー",
            "no_flows" => "フローデータをまだ受信していません。",
            "corrected_note" => "インジェスト時のサンプリング率補正済みです。",
            _ => "TracePulse",
        }
    } else {
        match key {
            "device_list" => "Device List",
            "interface_topology" => "Interface & Topology",
            "protocol_traffic" => "Protocol & Traffic View",
            "alert_stream" => "Alert Stream",
            "top_talkers" => "Top Talkers",
            "no_flows" => "No flow records received yet.",
            "corrected_note" => "Values corrected by ingest sampling rate.",
            _ => "TracePulse",
        }
    }
}

fn tui_help_text(notifications_available: bool) -> String {
    let notify = if notifications_available {
        " | [n] Notify"
    } else {
        ""
    };
    format!(
        "Ready. [1-5 / ←/→ / [/]] Tabs | [↑/↓/j/k] Move | [Enter] Details\n[PgUp/PgDn] Device on Interfaces | [p] Traffic | [w] Window | [s] Sort\n[/] Filter | [c] Clear | [r] Poll | [d] Discovery{notify} | [Space] Pause | [q] Quit"
    )
}

fn guess_local_cidr() -> String {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0")
        && socket.connect("8.8.8.8:53").is_ok()
        && let Ok(addr) = socket.local_addr()
        && let std::net::IpAddr::V4(ipv4) = addr.ip()
    {
        let octets = ipv4.octets();
        return format!("{}.{}.{}.0/24", octets[0], octets[1], octets[2]);
    }
    "192.168.1.0/24".to_string()
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn start_scan(cidr: &str, community: &str) -> Result<ScanTask, AppError> {
    crate::device::discovery::validate_community_scan_range(cidr)?;
    let hosts = crate::device::discovery::enumerate_cidr_hosts(cidr)?;
    let total = hosts.len();

    let scanned = Arc::new(AtomicUsize::new(0));
    let found = Arc::new(Mutex::new(Vec::new()));
    let finished = Arc::new(AtomicBool::new(false));

    let task = ScanTask {
        total,
        scanned: Arc::clone(&scanned),
        found: Arc::clone(&found),
        finished: Arc::clone(&finished),
    };

    let community = community.to_string();
    std::thread::spawn(move || {
        const CONCURRENCY: usize = 64;
        let chunk_size = (total + CONCURRENCY - 1).max(1) / CONCURRENCY.min(total.max(1));
        let chunks: Vec<Vec<_>> = hosts.chunks(chunk_size).map(|c| c.to_vec()).collect();

        let mut handles = Vec::new();
        for chunk in chunks {
            let comm = community.clone();
            let scanned_counter = Arc::clone(&scanned);
            let found_list = Arc::clone(&found);

            let handle = std::thread::spawn(move || {
                let client = SnmpClient::new(&comm);
                for ip in chunk {
                    let mut device = DeviceConfig::new(ip.to_string(), &comm);
                    if let Ok(sys_name) = client.probe_device(&device) {
                        device.name = sys_name;
                        device.status = "online".to_string();
                        if let Ok(mut list) = found_list.lock() {
                            list.push(device);
                        }
                    }
                    scanned_counter.fetch_add(1, Ordering::Relaxed);
                }
            });
            handles.push(handle);
        }

        for h in handles {
            let _ = h.join();
        }
        finished.store(true, Ordering::SeqCst);
    });

    Ok(task)
}

fn start_notification_test(
    provider: Arc<dyn NotificationSettingsProvider>,
    channel: String,
) -> NotificationTestTask {
    let finished = Arc::new(AtomicBool::new(false));
    let result = Arc::new(Mutex::new(None));

    let task = NotificationTestTask {
        channel: channel.clone(),
        finished: Arc::clone(&finished),
        result: Arc::clone(&result),
    };

    std::thread::spawn(move || {
        let outcome = provider.send_test(&channel);
        if let Ok(mut slot) = result.lock() {
            *slot = Some(outcome);
        }
        finished.store(true, Ordering::SeqCst);
    });

    task
}

fn start_poll_task(
    runner_config: crate::config::AppConfig,
    broadcaster: crate::alert::broadcaster::AlertBroadcaster,
    database_path: &std::path::Path,
) -> Result<PollTask, AppError> {
    let repository = Repository::new(
        rusqlite::Connection::open(database_path).map_err(|e| AppError::Database(e.to_string()))?,
    );
    let db_devices = repository.list_devices()?;
    let total = db_devices.len();

    let polled = Arc::new(AtomicUsize::new(0));
    let finished = Arc::new(AtomicBool::new(false));

    let task = PollTask {
        total,
        polled: Arc::clone(&polled),
        finished: Arc::clone(&finished),
    };

    std::thread::spawn(move || {
        if total > 0 {
            let engine = PollingEngine::with_broadcaster(runner_config, repository, broadcaster);
            let reachability_client = SnmpClient::new(engine.config.snmp.default_community.clone());

            for dev in db_devices {
                let cfg = DeviceConfig {
                    id: dev.id,
                    name: dev.name.clone(),
                    ip: dev.ip.clone(),
                    community: if dev.community.is_empty() {
                        engine.config.snmp.default_community.clone()
                    } else {
                        dev.community.clone()
                    },
                    device_type: dev.device_type.clone(),
                    status: dev.status.clone(),
                    last_seen_at: dev.last_seen_at.clone(),
                };

                if reachability_client
                    .probe_device_once(&cfg, Duration::from_millis(600))
                    .is_err()
                {
                    let _ = engine.repository.update_device_status(&cfg.ip, "offline");
                    if !dev.status.eq_ignore_ascii_case("offline") {
                        engine.publish_alert(AlertEvent::device_offline(&cfg, "SNMP timeout"));
                    }
                    polled.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                if let Ok((system, interfaces)) = engine.poll_device(&cfg) {
                    let _ = engine.repository.update_device_status(&cfg.ip, "online");
                    if let Some(device_id) = dev.id {
                        let now = chrono::Utc::now().to_rfc3339();
                        let metrics = DeviceMetrics {
                            id: None,
                            device_id,
                            cpu_usage: system.cpu_usage,
                            memory_usage: system.memory_usage,
                            memory_used_bytes: system.memory_used_bytes,
                            sampled_at: now.clone(),
                        };
                        let _ = engine.repository.save_device_metrics(&metrics);

                        for iface in &interfaces {
                            let mut sample = iface.clone();
                            sample.device_id = device_id;
                            sample.sampled_at = now.clone();
                            if let Ok(Some(prev)) = engine
                                .repository
                                .get_latest_interface_sample(device_id, sample.if_index)
                            {
                                sample.bandwidth_utilization =
                                    calculate_bandwidth_utilization_from_delta(
                                        prev.in_octets.saturating_add(prev.out_octets),
                                        sample.in_octets.saturating_add(sample.out_octets),
                                        engine.config.polling.interval_seconds,
                                        1_000_000_000,
                                    );
                            }
                            let _ = engine.repository.save_sample(&sample);
                        }

                        let spike_threshold = engine.config.alert.spike_threshold;
                        if let Ok(spikes) = engine
                            .repository
                            .check_interface_spikes(device_id, spike_threshold)
                        {
                            for spike in spikes {
                                let event =
                                    AlertEvent::from_interface_spike(&cfg, &spike, spike_threshold);
                                engine.publish_alert(event);
                            }
                        }
                    }
                }
                polled.fetch_add(1, Ordering::Relaxed);
            }
        }
        finished.store(true, Ordering::SeqCst);
    });

    Ok(task)
}

impl TuiRenderer {
    pub fn new(runner: AppRunner) -> Self {
        Self { runner }
    }

    pub fn run(self) -> Result<(), AppError> {
        // ターミナルの Raw モード有効化と Alternate Screen への切り替え
        enable_raw_mode().map_err(|e| AppError::Io(e.to_string()))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| AppError::Io(e.to_string()))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|e| AppError::Io(e.to_string()))?;
        // レイアウトの margin により端の行・列は描画されないため、切替前の画面の残骸を消す。
        terminal.clear().map_err(|e| AppError::Io(e.to_string()))?;

        let res = self.run_app(&mut terminal);

        // クリーンアップ処理
        let _ = disable_raw_mode();
        let _ = execute!(
            terminal.backend_mut(),
            DisableMouseCapture,
            LeaveAlternateScreen
        );
        let _ = terminal.show_cursor();

        res
    }

    fn run_app<B: ratatui::backend::Backend>(
        &self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), AppError> {
        if self.runner.flow_repository().is_none() {
            let repository = Repository::new(
                rusqlite::Connection::open(self.runner.database_path())
                    .expect("SQLite flow repository should open"),
            );
            let repository = Arc::new(Mutex::new(repository));
            crate::flow::FlowCollector::start(
                Arc::clone(&repository),
                crate::flow::FlowCollectorConfig::from_app_config(&self.runner.config),
            );
        }
        let repository = Repository::new(
            rusqlite::Connection::open(self.runner.database_path())
                .map_err(|e| AppError::Database(e.to_string()))?,
        );

        // TUI モードでも定期監視とアラート配信を行う（Web モードと同じポーリングループ）。
        {
            let polling_config = Arc::new(Mutex::new(self.runner.config.clone()));
            let polling_repository = Arc::new(Mutex::new(Repository::new(
                rusqlite::Connection::open(self.runner.database_path())
                    .map_err(|e| AppError::Database(e.to_string()))?,
            )));
            let polling_alerts = self.runner.alert_broadcaster();
            std::thread::spawn(move || {
                crate::web::server::run_polling_loop(
                    polling_config,
                    polling_repository,
                    polling_alerts,
                    false,
                );
            });
        }

        let mut alert_rx = self.runner.subscribe_alerts();
        let mut devices = self.load_device_summaries(&repository)?;
        let mut alerts = repository.get_recent_alerts(50).unwrap_or_default();
        let mut alert_idx: usize = 0;
        let mut alert_table_state = TableState::default();

        let mut selected_device_idx: usize = 0;
        let mut table_state = TableState::default();
        if !devices.is_empty() {
            table_state.select(Some(0));
        }

        let mut current_interfaces = if !devices.is_empty() {
            self.load_interface_summaries(&devices[0].device, &repository)
        } else {
            Vec::new()
        };

        let mut if_table_state = TableState::default();
        if !current_interfaces.is_empty() {
            if_table_state.select(Some(0));
        }

        let mut mode = TuiMode::Normal;
        let mut active_tab = TuiTab::Dashboard;
        let mut tab_area = Rect::default();
        let mut protocol_filter: Option<i32> = None;
        let mut protocol_window = 60_i64;
        let mut protocol_sort = ProtocolSort::Bps;
        let mut filter_query = String::new();
        let mut tab_filters = std::array::from_fn(|_| String::new());
        let mut protocol_talker_idx = 0_usize;
        let tui_language = self.runner.config.display.language.as_str();
        let mut tui_paused = false;
        let mut interface_idx: usize = 0;
        let mut status_msg = tui_help_text(self.runner.notification_provider().is_some());
        let flow_repository = self.runner.flow_repository();

        loop {
            // スキャン完了チェック
            if let TuiMode::Scanning(ref task) = mode
                && task.finished.load(Ordering::SeqCst)
            {
                let found_devices = task.found.lock().map(|l| l.clone()).unwrap_or_default();

                for dev in found_devices {
                    let _ = repository.save_device_config(&dev);
                }

                // 登録直後にプログレスバー付きで初回ポーリングタスクを開始
                match start_poll_task(
                    self.runner.config.clone(),
                    self.runner.alert_broadcaster(),
                    self.runner.database_path(),
                ) {
                    Ok(poll_task) => {
                        mode = TuiMode::Polling(poll_task);
                    }
                    Err(_) => {
                        devices = self.load_device_summaries(&repository)?;
                        if let Some(dev) = devices.get(selected_device_idx) {
                            current_interfaces =
                                self.load_interface_summaries(&dev.device, &repository);
                        }
                        interface_idx = 0;
                        if !current_interfaces.is_empty() {
                            if_table_state.select(Some(0));
                        } else {
                            if_table_state.select(None);
                        }
                        mode = TuiMode::Normal;
                    }
                }
            }

            // ポーリング完了チェック
            if let TuiMode::Polling(ref task) = mode
                && task.finished.load(Ordering::SeqCst)
            {
                devices = self.load_device_summaries(&repository)?;
                if let Some(dev) = devices.get(selected_device_idx) {
                    current_interfaces = self.load_interface_summaries(&dev.device, &repository);
                }
                if interface_idx >= current_interfaces.len() {
                    interface_idx = current_interfaces.len().saturating_sub(1);
                }
                if !current_interfaces.is_empty() {
                    if_table_state.select(Some(interface_idx));
                } else {
                    if_table_state.select(None);
                }
                alerts = repository.get_recent_alerts(50).unwrap_or_default();

                status_msg = String::from(
                    "Manual polling completed.\n[↑/↓/j/k] Select | [r] Poll | [d] Discovery | [q] Quit",
                );
                mode = TuiMode::Normal;
            }
            // テスト通知の完了チェック
            if let TuiMode::Notifications(ref mut state) = mode {
                let finished = state
                    .test
                    .as_ref()
                    .map(|task| task.finished.load(Ordering::SeqCst))
                    .unwrap_or(false);
                if finished {
                    let outcome = state
                        .test
                        .as_ref()
                        .and_then(|task| task.result.lock().ok().and_then(|mut slot| slot.take()));
                    state.test = None;
                    match outcome {
                        Some(Ok(message)) => {
                            state.message = Some(message);
                            state.message_is_error = false;
                        }
                        Some(Err(err)) => {
                            state.message = Some(err);
                            state.message_is_error = true;
                        }
                        None => {}
                    }
                }
            }

            // アラートのリアルタイム受信（非ブロッキング）
            while let Ok(evt) = alert_rx.try_recv() {
                let had_alerts = !alerts.is_empty();
                alerts.insert(
                    0,
                    RecentAlert {
                        id: None,
                        device_id: evt.device_id.unwrap_or(0),
                        device_name: evt.device_name.clone(),
                        device_ip: evt.device_ip.clone(),
                        alert_type: evt.kind.to_string(),
                        severity: evt.severity.to_string(),
                        details: evt.message.clone(),
                        created_at: Some(evt.occurred_at.to_rfc3339()),
                    },
                );
                if alerts.len() > 100 {
                    alerts.pop();
                }
                if had_alerts {
                    alert_idx = alert_idx
                        .saturating_add(1)
                        .min(alerts.len().saturating_sub(1));
                }
            }

            if !tui_paused {
                terminal
                .draw(|f| {
                    let footer_height = 3;
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [
                                Constraint::Length(3), // トップタブ
                                Constraint::Percentage(32), // 上ペイン: Device List
                                Constraint::Percentage(42), // 中ペイン: Interface & Topology
                                Constraint::Percentage(20), // 下ペイン: Alert Stream
                                Constraint::Length(footer_height), // フッター / フィルター入力
                            ]
                            .as_ref(),
                        )
                        .split(f.area());

                    tab_area = chunks[0];
                    f.render_widget(
                        Tabs::new(TuiTab::LABELS)
                            .select(active_tab.index())
                            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                            .block(Block::default().borders(Borders::ALL).title(" TracePulse ")),
                        tab_area,
                    );
                    let body_area = Rect::new(
                        chunks[1].x,
                        chunks[1].y,
                        chunks[1].width,
                        chunks[3].bottom().saturating_sub(chunks[1].y),
                    );
                    let show_device_table = active_tab == TuiTab::Devices;
                    let show_middle_panel = matches!(active_tab, TuiTab::Interfaces | TuiTab::Traffic);

                    if active_tab == TuiTab::Dashboard {
                        let online = devices.iter().filter(|item| item.device.status == "online").count();
                        let warning = devices.iter().filter(|item| item.device.status == "warning").count();
                        let offline = devices.iter().filter(|item| item.device.status == "offline").count();
                        let overview = Paragraph::new(format!(
                            "Devices: {}    Online: {online}    Warning: {warning}    Offline: {offline}\nRecent alerts: {}",
                            devices.len(), alerts.len(),
                        ))
                        .block(Block::default().borders(Borders::ALL).title(" Overview "));
                        f.render_widget(overview, chunks[1]);

                        let attention = devices.iter()
                            .filter(|item| item.device.status != "online" || item.alert_count > 0)
                            .take(chunks[2].height.saturating_sub(2) as usize)
                            .map(|item| format!(
                                "{} ({})  {}  alerts: {}",
                                item.device.name, item.device.ip, item.device.status, item.alert_count
                            ))
                            .collect::<Vec<_>>();
                        let attention = if attention.is_empty() {
                            vec!["No devices need attention".to_string()]
                        } else {
                            attention
                        };
                        f.render_widget(
                            Paragraph::new(attention.join("\n"))
                                .block(Block::default().borders(Borders::ALL).title(" Needs Attention "))
                                .wrap(Wrap { trim: true }),
                            chunks[2],
                        );
                    }

                    // 1. 上ペイン: Device List
                    let header_cells = [
                        "IP Address",
                        "Hostname",
                        "Status",
                        "CPU Usage",
                        "Mem Usage",
                        "Active Ports",
                        "Alerts",
                    ]
                    .iter()
                    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                    let header = Row::new(header_cells)
                        .style(Style::default().bg(Color::DarkGray))
                        .height(1);

                    let visible_devices = devices.iter().enumerate().filter(|(_, item)| {
                        tui_filter_matches(&filter_query, &[&item.device.ip, &item.device.name, &item.device.status])
                    }).map(|(index, _)| index).collect::<Vec<_>>();
                    let rows = visible_devices.iter().map(|&index| {
                        let item = &devices[index];
                        let status_str = item.device.status.to_lowercase();
                        let status_style = match status_str.as_str() {
                            "online" | "active" | "healthy" => Style::default().fg(Color::Green),
                            "warning" | "degraded" => Style::default().fg(Color::Yellow),
                            _ => Style::default().fg(Color::Red),
                        };

                        let cpu_str = item
                            .cpu_usage
                            .map(|c| format!("{}%", c))
                            .unwrap_or_else(|| "N/A".to_string());

                        let mem_str = item
                            .memory_usage
                            .map(|m| format!("{}%", m))
                            .unwrap_or_else(|| "N/A".to_string());

                        let ports_str = format!("{}/{}", item.active_ports, item.total_ports);
                        let alert_str = item.alert_count.to_string();

                        let cells = vec![
                            Cell::from(item.device.ip.clone()),
                            Cell::from(item.device.name.clone()),
                            Cell::from(item.device.status.clone()).style(status_style),
                            Cell::from(cpu_str),
                            Cell::from(mem_str),
                            Cell::from(ports_str),
                            Cell::from(alert_str).style(if item.alert_count > 0 { Style::default().fg(Color::LightRed) } else { Style::default() }),
                        ];
                        Row::new(cells).height(1)
                    });

                    let device_table = Table::new(
                        rows,
                        [
                            Constraint::Percentage(18),
                            Constraint::Percentage(24),
                            Constraint::Percentage(14),
                            Constraint::Percentage(11),
                            Constraint::Percentage(11),
                            Constraint::Percentage(11),
                            Constraint::Percentage(11),
                        ],
                    )
                    .header(header)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!(" 1. {} (Registered Devices) ", tui_text(tui_language, "device_list"))),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(Color::Blue)
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");

                    if show_device_table {
                        let area = if active_tab == TuiTab::Devices { body_area } else { chunks[1] };
                        table_state.select(visible_devices.iter().position(|index| *index == selected_device_idx));
                        f.render_stateful_widget(device_table, area, &mut table_state);
                    }

                    // 2. 中ペイン: Interface & Topology / Protocol View
                    let selected_device_name = devices
                        .get(selected_device_idx)
                        .map(|d| format!("{} ({})", d.device.name, d.device.ip))
                        .unwrap_or_else(|| "No Device Selected".to_string());

                    if show_middle_panel {
                        if active_tab == TuiTab::Traffic {
                            let exporter_ip = devices.get(selected_device_idx).map(|item| item.device.ip.as_str());
                            let filter_label = protocol_filter
                                .and_then(|if_index| current_interfaces.iter().find(|iface| iface.if_index == if_index))
                                .map(|iface| format!("Filter: {}", iface.if_name))
                                .unwrap_or_else(|| "Filter: All Interfaces".to_string());
                            let shares = flow_repository.as_ref()
                                .and_then(|repo| repo.protocol_shares_for_context(protocol_window, exporter_ip, protocol_filter).ok())
                                .unwrap_or_else(|| repository.protocol_shares_for_context(protocol_window, exporter_ip, protocol_filter).unwrap_or_default());
                            let (summary, active_exporters) = flow_repository.as_ref()
                                .and_then(|repo| repo.flow_summary_for_context(protocol_window, exporter_ip, protocol_filter).ok())
                                .unwrap_or_else(|| repository.flow_summary_for_context(protocol_window, exporter_ip, protocol_filter).unwrap_or((crate::db::models::FlowSummary { total_bps: 0.0, total_pps: 0.0, active_flows: 0, top_protocol: "-".to_string() }, 0)));
                            let applications = flow_repository.as_ref()
                                .and_then(|repo| repo.flow_applications_for_context(protocol_window, 3, exporter_ip, protocol_filter).ok())
                                .unwrap_or_else(|| repository.flow_applications_for_context(protocol_window, 3, exporter_ip, protocol_filter).unwrap_or_default());
                            let mut talkers = flow_repository.as_ref()
                                .and_then(|repo| repo.top_talkers_for_context(protocol_window, 5, exporter_ip, protocol_filter, protocol_sort.query_key()).ok())
                                .unwrap_or_else(|| repository.top_talkers_for_context(protocol_window, 5, exporter_ip, protocol_filter, protocol_sort.query_key()).unwrap_or_default());
                            match protocol_sort {
                                ProtocolSort::Bps => talkers.sort_by_key(|a| std::cmp::Reverse(a.bps)),
                                ProtocolSort::Pps => talkers.sort_by_key(|a| std::cmp::Reverse(a.pps)),
                                ProtocolSort::Bytes => talkers.sort_by_key(|a| std::cmp::Reverse(a.bytes)),
                            }
                            let visible_talkers: Vec<&TopTalker> = talkers.iter().filter(|talker| {
                                tui_filter_matches(&filter_query, &[
                                    &talker.source_ip, &talker.destination_ip, &talker.protocol,
                                    &talker.app_name,
                                ])
                            }).collect();
                            if protocol_talker_idx >= visible_talkers.len() {
                                protocol_talker_idx = visible_talkers.len().saturating_sub(1);
                            }
                            let mix_height = (if body_area.width < 76 { 12 } else { 8 })
                                .min(body_area.height.saturating_sub(9));
                            let talker_height = (visible_talkers.len() as u16 + 4).max(7)
                                .min(body_area.height.saturating_sub(4 + mix_height));
                            let sections = Layout::default().direction(Direction::Vertical)
                                .constraints([
                                    Constraint::Length(4),
                                    Constraint::Length(mix_height),
                                    Constraint::Length(talker_height),
                                    Constraint::Min(0),
                                ]).split(body_area);
                            let summary_line = if body_area.width >= 90 {
                                format!("BPS  {:>10}    PPS  {:>10}    FLOWS  {:>6}    EXPORTERS  {:>4}",
                                    format_tui_rate(summary.total_bps), format_tui_count(summary.total_pps), summary.active_flows, active_exporters)
                            } else {
                                format!("BPS {}  PPS {}\nFLOWS {}  EXPORTERS {}",
                                    format_tui_rate(summary.total_bps), format_tui_count(summary.total_pps), summary.active_flows, active_exporters)
                            };
                            f.render_widget(
                                Paragraph::new(summary_line)
                                    .style(Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD))
                                    .block(Block::default().borders(Borders::ALL)
                                        .title(format!(" Traffic  |  {}  |  {}s  |  {} ",
                                            selected_device_name, protocol_window, filter_label))),
                                sections[0],
                            );

                            let mix = Layout::default().direction(if body_area.width < 76 { Direction::Vertical } else { Direction::Horizontal })
                                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                                .split(sections[1]);
                            let protocol_lines = shares.iter().take(5).map(|share| {
                                let color = match share.protocol.to_ascii_uppercase().as_str() {
                                    "TCP" => Color::Cyan, "UDP" => Color::Yellow,
                                    "ICMP" => Color::Magenta, _ => Color::Green,
                                };
                                Line::from(vec![
                                    Span::styled(format!("{:<6} ", truncate_tui(&share.protocol, 6).trim_end()), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                                    Span::styled(tui_progress_bar(share.percentage, 12), Style::default().fg(color)),
                                    Span::raw(format!(" {:>5.1}%", share.percentage)),
                                ])
                            }).collect::<Vec<_>>();
                            let app_lines = applications.iter().take(5).map(|app| {
                                Line::from(vec![
                                    Span::styled(format!("{:<16} ", truncate_tui(&app.app_name, 16).trim_end()), Style::default().fg(Color::LightGreen)),
                                    Span::styled(tui_progress_bar(app.percentage, 10), Style::default().fg(Color::Green)),
                                    Span::raw(format!(" {:>5.1}%", app.percentage)),
                                ])
                            }).collect::<Vec<_>>();
                            f.render_widget(
                                Paragraph::new(if protocol_lines.is_empty() { vec![Line::from("No protocol samples")] } else { protocol_lines })
                                    .block(Block::default().borders(Borders::ALL).title(" Protocol mix ")),
                                mix[0],
                            );
                            f.render_widget(
                                Paragraph::new(if app_lines.is_empty() { vec![Line::from("No application samples")] } else { app_lines })
                                    .block(Block::default().borders(Borders::ALL).title(" Top applications ")),
                                mix[1],
                            );

                            let talker_area = sections[2];
                            if visible_talkers.is_empty() {
                                f.render_widget(
                                    Paragraph::new(if talkers.is_empty() { tui_text(tui_language, "no_flows") } else { "No flows match the filter" })
                                        .style(Style::default().fg(Color::DarkGray))
                                        .block(Block::default().borders(Borders::ALL).title(format!(" {} ", tui_text(tui_language, "top_talkers")))),
                                    talker_area,
                                );
                            } else if (96..110).contains(&talker_area.width) {
                                let mut lines = vec![Line::from(flow_header_line()).style(Style::default().fg(Color::Yellow))];
                                lines.extend(visible_talkers.iter().enumerate().map(|(index, talker)| {
                                    let line = Line::from(flow_row_line(talker));
                                    if index == protocol_talker_idx {
                                        line.style(Style::default().bg(Color::Cyan).fg(Color::Black))
                                    } else { line }
                                }));
                                f.render_widget(
                                    Paragraph::new(Text::from(lines))
                                        .block(Block::default().borders(Borders::ALL).title(format!(" {}  |  {} ", tui_text(tui_language, "top_talkers"), protocol_sort.label()))),
                                    talker_area,
                                );
                            } else {
                                let wide = talker_area.width >= 110;
                                let header = if wide {
                                    vec!["SOURCE", "DESTINATION", "PROTO", "BYTES", "BPS", "PPS", "FLAGS", "IN / OUT IF"]
                                } else {
                                    vec!["SOURCE", "DESTINATION", "PROTO", "BPS"]
                                };
                                let widths = if wide {
                                    vec![Constraint::Percentage(18), Constraint::Percentage(18), Constraint::Percentage(8), Constraint::Percentage(8), Constraint::Percentage(8), Constraint::Percentage(7), Constraint::Percentage(7), Constraint::Percentage(26)]
                                } else {
                                    vec![Constraint::Percentage(36), Constraint::Percentage(36), Constraint::Percentage(10), Constraint::Percentage(18)]
                                };
                                let rows = visible_talkers.iter().map(|talker| {
                                    let source = format!("{}:{}", talker.source_ip, talker.source_port);
                                    let destination = format!("{}:{}", talker.destination_ip, talker.destination_port);
                                    let mut cells = vec![Cell::from(source), Cell::from(destination),
                                        Cell::from(talker.protocol.clone()).style(Style::default().fg(Color::Cyan))];
                                    if wide {
                                        let ingress = talker.ingress_if_name.clone().or_else(|| talker.ingress_if_index.map(|index| format!("if-{index}"))).unwrap_or_else(|| "-".to_string());
                                        let egress = talker.egress_if_name.clone().or_else(|| talker.egress_if_index.map(|index| format!("if-{index}"))).unwrap_or_else(|| "-".to_string());
                                        cells.push(Cell::from(format_tui_bytes(talker.bytes)));
                                        cells.push(Cell::from(format_tui_rate(talker.bps as f64)));
                                        cells.push(Cell::from(talker.pps.to_string()));
                                        cells.push(Cell::from(format_tui_flags(talker.tcp_flags)));
                                        cells.push(Cell::from(format!("{ingress} / {egress}")));
                                    } else {
                                        cells.push(Cell::from(format_tui_rate(talker.bps as f64)));
                                    }
                                    Row::new(cells)
                                });
                                let table = Table::new(rows, widths)
                                    .header(Row::new(header).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
                                    .block(Block::default().borders(Borders::ALL).title(format!(" {}  |  {} ", tui_text(tui_language, "top_talkers"), protocol_sort.label())))
                                    .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
                                    .highlight_symbol("> ");
                                let mut flow_state = TableState::default();
                                flow_state.select(Some(protocol_talker_idx));
                                f.render_stateful_widget(table, talker_area, &mut flow_state);
                            }
                        } else {
                            let if_header_cells = [
                        "Port Name",
                        "Status",
                        "Rate / Bandwidth",
                        "Err / Disc (In/Out)",
                        "Late Coll",
                        "LLDP / CDP Neighbor (Topology)",
                    ]
                    .iter()
                    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
                    let if_header = Row::new(if_header_cells)
                        .style(Style::default().bg(Color::DarkGray))
                        .height(1);

                    let visible_interfaces = current_interfaces.iter().enumerate().filter(|(_, iface)| {
                        tui_filter_matches(&filter_query, &[&iface.if_name, &iface.if_index.to_string(), &iface.link_status, &iface.neighbor])
                    }).map(|(index, _)| index).collect::<Vec<_>>();
                    let if_rows = visible_interfaces.iter().map(|&index| {
                        let iface = &current_interfaces[index];
                        let is_up = iface.link_status.to_lowercase() == "up" || iface.link_status == "1";
                        let status_style = if is_up {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };

                        let status_text = if is_up { "UP" } else { "DOWN" };
                        let rate_text = if iface.bandwidth_utilization > 0.0 {
                            format!("{:.2}%", iface.bandwidth_utilization)
                        } else {
                            "0.0%".to_string()
                        };

                        let err_disc = format!(
                            "{}/{} ({}/{})",
                            iface.in_errors, iface.out_errors, iface.in_discards, iface.out_discards
                        );

                        let err_style = if iface.in_errors + iface.out_errors > 0 {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        };

                        let cells = vec![
                            Cell::from(iface.if_name.clone()),
                            Cell::from(status_text).style(status_style),
                            Cell::from(rate_text),
                            Cell::from(err_disc).style(err_style),
                            Cell::from(iface.late_collisions.to_string()),
                            Cell::from(iface.neighbor.clone()).style(if iface.neighbor != "N/A" {
                                Style::default().fg(Color::LightCyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            }),
                        ];
                        Row::new(cells).height(1)
                    });

                    let if_table = Table::new(
                        if_rows,
                        [
                            Constraint::Percentage(18),
                            Constraint::Percentage(10),
                            Constraint::Percentage(14),
                            Constraint::Percentage(20),
                            Constraint::Percentage(10),
                            Constraint::Percentage(28),
                        ],
                    )
                    .header(if_header)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!(
                                " 2. {} (Selected: {}){} ",
                                tui_text(tui_language, "interface_topology"),
                                selected_device_name,
                                " [Enter=Error Breakdown]"
                            )),
                    )
                    .highlight_style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
                    .highlight_symbol(">> ");

                    let area = if active_tab == TuiTab::Interfaces { body_area } else { chunks[2] };
                    if_table_state.select(visible_interfaces.iter().position(|index| *index == interface_idx));
                    f.render_stateful_widget(if_table, area, &mut if_table_state);
                        }
                    }

                    // 3. 下ペイン: Alert Stream
                    let alert_header_cells = ["Time", "Device Name (IP)", "Type", "Severity", "Details"]
                        .iter()
                        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)));
                    let alert_header = Row::new(alert_header_cells)
                        .style(Style::default().bg(Color::DarkGray))
                        .height(1);

                    let visible_alerts = alerts.iter().enumerate()
                        .filter(|(_, alert)| active_tab != TuiTab::Alerts || tui_filter_matches(
                            &filter_query, &[&alert.device_name, &alert.device_ip, &alert.alert_type, &alert.severity, &alert.details]
                        ))
                        .map(|(index, _)| index).collect::<Vec<_>>();
                    let alert_rows = visible_alerts.iter().map(|&index| {
                        let alert = &alerts[index];
                        let sev_style = match alert.severity.to_uppercase().as_str() {
                            "CRITICAL" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            "WARNING" => Style::default().fg(Color::Yellow),
                            _ => Style::default().fg(Color::Green),
                        };

                        let time_str = alert
                            .created_at
                            .as_deref()
                            .map(|timestamp| {
                                format_tui_timestamp(timestamp, &self.runner.config.display.timezone)
                            })
                            .unwrap_or_else(|| "-".to_string());

                        let dev_str = format!("{} ({})", alert.device_name, alert.device_ip);

                        let cells = vec![
                            Cell::from(time_str),
                            Cell::from(dev_str),
                            Cell::from(alert.alert_type.clone()),
                            Cell::from(alert.severity.clone()).style(sev_style),
                            Cell::from(alert.details.clone()),
                        ];
                        Row::new(cells).height(1)
                    });

                    let alert_table = Table::new(
                        alert_rows,
                        [
                            Constraint::Percentage(18),
                            Constraint::Percentage(22),
                            Constraint::Percentage(20),
                            Constraint::Percentage(12),
                            Constraint::Percentage(28),
                        ],
                    )
                    .header(alert_header)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!(" 3. {} (Recent L1/L2 & Bandwidth Errors) ", tui_text(tui_language, "alert_stream"))),
                    )
                    .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
                    .highlight_symbol(">> ");

                    if matches!(active_tab, TuiTab::Dashboard | TuiTab::Alerts) {
                        let area = if active_tab == TuiTab::Alerts { body_area } else { chunks[3] };
                        if active_tab == TuiTab::Alerts {
                            alert_table_state.select(visible_alerts.iter().position(|index| *index == alert_idx));
                            f.render_stateful_widget(alert_table, area, &mut alert_table_state);
                        } else {
                            f.render_widget(alert_table, area);
                        }
                    }

                    // 4. フッター / ステータスバー
                    let footer = Paragraph::new(status_msg.clone())
                        .wrap(Wrap { trim: true })
                        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
                    f.render_widget(footer, chunks[4]);

                    // モーダル・ダイアログ オーバーレイ描画
                    match mode {
                        TuiMode::AlertDetails(ref alert) => {
                            let popup_area = centered_rect(78, 55, f.area());
                            f.render_widget(Clear, popup_area);
                            f.render_widget(
                                Paragraph::new(format!(
                                    "{}  {}\n{} ({})\n{}\n\n{}",
                                    alert.severity,
                                    alert.created_at.as_deref().unwrap_or("-"),
                                    alert.device_name,
                                    alert.device_ip,
                                    alert.alert_type,
                                    alert.details,
                                ))
                                .block(Block::default().borders(Borders::ALL).title(" Alert Details "))
                                .wrap(Wrap { trim: true }),
                                popup_area,
                            );
                        }
                        TuiMode::FlowInspector(ref talker) => {
                            let popup_area = centered_rect(78, 65, f.area());
                            f.render_widget(Clear, popup_area);
                            let ingress = talker.ingress_if_name.clone()
                                .or_else(|| talker.ingress_if_index.map(|value| format!("if-{value}")))
                                .unwrap_or_else(|| "-".to_string());
                            let egress = talker.egress_if_name.clone()
                                .or_else(|| talker.egress_if_index.map(|value| format!("if-{value}")))
                                .unwrap_or_else(|| "-".to_string());
                            let inspector_text = format!(
                                "Source             : {}:{}\nDestination        : {}:{}\nProtocol / App     : {} / {}\n\nRaw Bytes          : {}\nPackets            : {}\nBps / PPS          : {} / {}\nTCP Flags          : {} (S=SYN A=ACK R=RST F=FIN)\nIn IF              : {}\nOut IF             : {}\n\nActive Timeout     : not retained in rollup\nInactive Timeout   : not retained in rollup\nSampling Rate      : corrected at ingest; rate not retained here\n\nPress [Esc] or [Enter] to close",
                                talker.source_ip, talker.source_port,
                                talker.destination_ip, talker.destination_port,
                                talker.protocol, talker.app_name,
                                talker.bytes, talker.packets,
                                format_tui_rate(talker.bps as f64), talker.pps,
                                format_tui_flags(talker.tcp_flags), ingress, egress,
                            );
                            f.render_widget(
                                Paragraph::new(inspector_text)
                                    .block(Block::default().borders(Borders::ALL).title(" Flow Inspector "))
                                    .style(Style::default().fg(Color::White))
                                    .wrap(Wrap { trim: false }),
                                popup_area,
                            );
                        }
                        TuiMode::FilterInput(ref query) => {
                            f.render_widget(Clear, chunks[4]);
                            let prompt = Paragraph::new(format!(
                                "Filter: {}\n[Enter] Apply  [Esc] Cancel  [c] Clear",
                                query
                            ))
                                .block(Block::default().borders(Borders::ALL).title(" Incremental Filter "))
                                .style(Style::default().bg(Color::DarkGray).fg(Color::White));
                            f.render_widget(prompt, chunks[4]);
                            f.set_cursor_position((chunks[4].x + 1 + 8 + query.chars().count() as u16, chunks[4].y + 1));
                        }
                        TuiMode::DiscoveryModal(ref modal) => {
                            let popup_area = centered_rect(60, 55, f.area());
                            f.render_widget(Clear, popup_area);

                            let block = Block::default()
                                .borders(Borders::ALL)
                                .title(" Network Discovery Modal ")
                                .style(Style::default().bg(Color::Reset));
                            f.render_widget(block, popup_area);

                            let inner_layout = Layout::default()
                                .direction(Direction::Vertical)
                                .margin(2)
                                .constraints([
                                    Constraint::Length(3), // CIDR
                                    Constraint::Length(3), // Community
                                    Constraint::Length(3), // Version
                                    Constraint::Length(2), // Error or Message
                                    Constraint::Min(1),    // Instruction
                                ])
                                .split(popup_area);

                            let cidr_style = if modal.active_field == 0 {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Gray)
                            };
                            let cidr_input = Paragraph::new(modal.cidr.as_str())
                                .block(Block::default().borders(Borders::ALL).title(" Target CIDR Range ").style(cidr_style));
                            f.render_widget(cidr_input, inner_layout[0]);

                            let comm_style = if modal.active_field == 1 {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Gray)
                            };
                            let comm_input = Paragraph::new(modal.community.as_str())
                                .block(Block::default().borders(Borders::ALL).title(" SNMP Community ").style(comm_style));
                            f.render_widget(comm_input, inner_layout[1]);

                            let ver_style = if modal.active_field == 2 {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Gray)
                            };
                            let ver_display = if modal.active_field == 2 {
                                format!("◄  {}  ►  (Press ←/→/Space/1/2/3 to change)", modal.version)
                            } else {
                                modal.version.clone()
                            };
                            let ver_input = Paragraph::new(ver_display.as_str())
                                .block(Block::default().borders(Borders::ALL).title(" SNMP Version (Select) ").style(ver_style));
                            f.render_widget(ver_input, inner_layout[2]);

                            if let Some(err) = &modal.error_msg {
                                let err_p = Paragraph::new(err.as_str()).style(Style::default().fg(Color::LightRed));
                                f.render_widget(err_p, inner_layout[3]);
                            }

                            let hint_str = if modal.active_field == 2 {
                                "Press [Tab/↑/↓] Switch Field | [←/→/Space] Change Version | [Enter] Start Scan | [Esc] Cancel"
                            } else {
                                "Press [Tab/↑/↓] Switch Field | [←/→] Move Cursor | [Enter] Start Scan | [Esc] Cancel"
                            };
                            let hint_p = Paragraph::new(hint_str).style(Style::default().fg(Color::DarkGray));
                            f.render_widget(hint_p, inner_layout[4]);

                            // アクティブな入力枠がテキスト入力(0, 1)の時のみ物理カーソルを表示する
                            if modal.active_field != 2 {
                                let active_rect = inner_layout[modal.active_field];
                                f.set_cursor_position((
                                    active_rect.x + 1 + modal.cursor_position as u16,
                                    active_rect.y + 1,
                                ));
                            }
                        }
                        TuiMode::Scanning(ref task) => {
                            let popup_area = centered_rect(60, 35, f.area());
                            f.render_widget(Clear, popup_area);

                            let block = Block::default()
                                .borders(Borders::ALL)
                                .title(" Scanning Network ")
                                .style(Style::default().bg(Color::Reset));
                            f.render_widget(block, popup_area);

                            let inner_layout = Layout::default()
                                .direction(Direction::Vertical)
                                .margin(2)
                                .constraints([
                                    Constraint::Length(3), // Progress Gauge
                                    Constraint::Length(2), // Summary text
                                ])
                                .split(popup_area);

                            let scanned = task.scanned.load(Ordering::Relaxed);
                            let found_count = task.found.lock().map(|l| l.len()).unwrap_or(0);
                            let ratio = if task.total > 0 {
                                (scanned as f64 / task.total as f64).min(1.0)
                            } else {
                                1.0
                            };

                            let gauge = Gauge::default()
                                .block(Block::default().borders(Borders::ALL).title(" Progress "))
                                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
                                .ratio(ratio)
                                .label(format!("{:.0}%", ratio * 100.0));
                            f.render_widget(gauge, inner_layout[0]);

                            let status_p = Paragraph::new(format!(
                                "Scanned: {} / {} hosts  |  Discovered Devices: {}",
                                scanned, task.total, found_count
                            ))
                            .style(Style::default().fg(Color::Yellow));
                            f.render_widget(status_p, inner_layout[1]);
                        }
                        TuiMode::Polling(ref task) => {
                            let popup_area = centered_rect(60, 35, f.area());
                            f.render_widget(Clear, popup_area);

                            let block = Block::default()
                                .borders(Borders::ALL)
                                .title(" Manual Polling SNMP Devices ")
                                .style(Style::default().bg(Color::Reset));
                            f.render_widget(block, popup_area);

                            let inner_layout = Layout::default()
                                .direction(Direction::Vertical)
                                .margin(2)
                                .constraints([
                                    Constraint::Length(3), // Progress Gauge
                                    Constraint::Length(2), // Summary text
                                ])
                                .split(popup_area);

                            let polled = task.polled.load(Ordering::Relaxed);
                            let ratio = if task.total > 0 {
                                (polled as f64 / task.total as f64).min(1.0)
                            } else {
                                1.0
                            };

                            let gauge = Gauge::default()
                                .block(Block::default().borders(Borders::ALL).title(" Progress "))
                                .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black))
                                .ratio(ratio)
                                .label(format!("{:.0}% ({}/{})", ratio * 100.0, polled, task.total));
                            f.render_widget(gauge, inner_layout[0]);

                            let status_p = Paragraph::new(format!(
                                "Polled: {} / {} devices",
                                polled, task.total
                            ))
                            .style(Style::default().fg(Color::Yellow));
                            f.render_widget(status_p, inner_layout[1]);
                        }
                        TuiMode::PortErrorBreakdown(ref state) => {
                            let popup_area = centered_rect(60, 45, f.area());
                            f.render_widget(Clear, popup_area);

                            let block = Block::default()
                                .borders(Borders::ALL)
                                .title(format!(
                                    " Port Error Breakdown: {} (if-{}) ",
                                    state.if_name, state.if_index
                                ))
                                .style(Style::default().bg(Color::Reset));
                            f.render_widget(block, popup_area);

                            let inner_layout = Layout::default()
                                .direction(Direction::Vertical)
                                .margin(2)
                                .constraints([
                                    Constraint::Length(3), // FCS/CRC
                                    Constraint::Length(3), // Alignment
                                    Constraint::Length(3), // Frame-too-long
                                    Constraint::Length(3), // MAC receive
                                    Constraint::Min(1),    // Instruction
                                ])
                                .split(popup_area);

                            let max_val = [
                                state.fcs_errors_delta,
                                state.alignment_errors_delta,
                                state.frame_too_longs_delta,
                                state.internal_mac_receive_errors_delta,
                            ]
                            .into_iter()
                            .max()
                            .unwrap_or(0)
                            .max(1);

                            let ratio_of = |v: u64| (v as f64 / max_val as f64).min(1.0);

                            let fcs_gauge = Gauge::default()
                                .block(Block::default().borders(Borders::ALL).title(" FCS/CRC Errors "))
                                .gauge_style(Style::default().fg(Color::Red).bg(Color::Black))
                                .ratio(ratio_of(state.fcs_errors_delta))
                                .label(format!("+{}", state.fcs_errors_delta));
                            f.render_widget(fcs_gauge, inner_layout[0]);

                            let align_gauge = Gauge::default()
                                .block(Block::default().borders(Borders::ALL).title(" Alignment Errors "))
                                .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black))
                                .ratio(ratio_of(state.alignment_errors_delta))
                                .label(format!("+{}", state.alignment_errors_delta));
                            f.render_widget(align_gauge, inner_layout[1]);

                            let toolong_gauge = Gauge::default()
                                .block(Block::default().borders(Borders::ALL).title(" Frame-Too-Long (Giant) "))
                                .gauge_style(Style::default().fg(Color::Magenta).bg(Color::Black))
                                .ratio(ratio_of(state.frame_too_longs_delta))
                                .label(format!("+{}", state.frame_too_longs_delta));
                            f.render_widget(toolong_gauge, inner_layout[2]);

                            let macrx_gauge = Gauge::default()
                                .block(Block::default().borders(Borders::ALL).title(" Internal MAC Receive Errors "))
                                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
                                .ratio(ratio_of(state.internal_mac_receive_errors_delta))
                                .label(format!("+{}", state.internal_mac_receive_errors_delta));
                            f.render_widget(macrx_gauge, inner_layout[3]);

                            let hint_p = Paragraph::new("Press [Esc] to close")
                                .style(Style::default().fg(Color::DarkGray));
                            f.render_widget(hint_p, inner_layout[4]);
                        }
                        TuiMode::Notifications(ref state) => {
                            let popup_area = centered_rect(72, 55, f.area());
                            f.render_widget(Clear, popup_area);

                            let block = Block::default()
                                .borders(Borders::ALL)
                                .title(" Alert Notifications (view only - edit in Web GUI or config.toml) ")
                                .style(Style::default().bg(Color::Reset));
                            f.render_widget(block, popup_area);

                            let inner_layout = Layout::default()
                                .direction(Direction::Vertical)
                                .margin(2)
                                .constraints([
                                    Constraint::Length(5), // Channel table
                                    Constraint::Length(3), // Delivery settings
                                    Constraint::Length(3), // Test result
                                    Constraint::Min(1),    // Instruction
                                ])
                                .split(popup_area);

                            let channel_header = Row::new(
                                ["Channel", "Enabled", "Webhook (masked)", "Source"]
                                    .iter()
                                    .map(|h| {
                                        Cell::from(*h).style(
                                            Style::default()
                                                .fg(Color::Cyan)
                                                .add_modifier(Modifier::BOLD),
                                        )
                                    }),
                            )
                            .style(Style::default().bg(Color::DarkGray))
                            .height(1);

                            let channel_rows = state.status.channels.iter().map(|channel| {
                                let (enabled_text, enabled_style) = if channel.enabled {
                                    ("yes", Style::default().fg(Color::Green))
                                } else {
                                    ("no", Style::default().fg(Color::DarkGray))
                                };
                                Row::new(vec![
                                    Cell::from(channel.name.clone()),
                                    Cell::from(enabled_text).style(enabled_style),
                                    Cell::from(channel.webhook.clone()),
                                    Cell::from(channel.source.clone()),
                                ])
                                .height(1)
                            });

                            let channel_table = Table::new(
                                channel_rows,
                                [
                                    Constraint::Percentage(14),
                                    Constraint::Percentage(11),
                                    Constraint::Percentage(43),
                                    Constraint::Percentage(32),
                                ],
                            )
                            .header(channel_header)
                            .block(Block::default().borders(Borders::ALL).title(" Channels "))
                            .highlight_style(
                                Style::default()
                                    .bg(Color::Blue)
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .highlight_symbol("> ");

                            let mut channel_state = TableState::default();
                            if !state.status.channels.is_empty() {
                                channel_state.select(Some(
                                    state.selected.min(state.status.channels.len() - 1),
                                ));
                            }
                            f.render_stateful_widget(
                                channel_table,
                                inner_layout[0],
                                &mut channel_state,
                            );

                            let delivery_p = Paragraph::new(format!(
                                "Flap guard window: {} s    Retry attempts: {}",
                                state.status.flap_window_seconds, state.status.retry_max_attempts
                            ))
                            .block(Block::default().borders(Borders::ALL).title(" Delivery "))
                            .style(Style::default().fg(Color::White));
                            f.render_widget(delivery_p, inner_layout[1]);

                            let (result_text, result_style) = if let Some(task) = &state.test {
                                (
                                    format!("Sending test notification to {}...", task.channel),
                                    Style::default().fg(Color::Yellow),
                                )
                            } else if let Some(message) = &state.message {
                                let style = if state.message_is_error {
                                    Style::default().fg(Color::LightRed)
                                } else {
                                    Style::default().fg(Color::Green)
                                };
                                (message.clone(), style)
                            } else {
                                (
                                    "No test sent yet.".to_string(),
                                    Style::default().fg(Color::DarkGray),
                                )
                            };
                            let result_p = Paragraph::new(result_text)
                                .block(Block::default().borders(Borders::ALL).title(" Test Result "))
                                .style(result_style);
                            f.render_widget(result_p, inner_layout[2]);

                            let hint_p = Paragraph::new(
                                "[↑/↓/j/k] Select Channel | [t/Enter] Send Test\n[a] Test All Enabled | [Esc/n] Close",
                            )
                            .wrap(Wrap { trim: true })
                            .style(Style::default().fg(Color::DarkGray));
                            f.render_widget(hint_p, inner_layout[3]);
                        }
                        TuiMode::Normal => {}
                    }
                })
                .map_err(|e| AppError::Io(e.to_string()))?;
            }

            // キー入力待ち (50ms タイムアウト)
            if event::poll(Duration::from_millis(50)).map_err(|e| AppError::Io(e.to_string()))? {
                match event::read().map_err(|e| AppError::Io(e.to_string()))? {
                    Event::Mouse(mouse)
                        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                            && matches!(mode, TuiMode::Normal)
                            && mouse.row >= tab_area.y
                            && mouse.row < tab_area.y.saturating_add(tab_area.height)
                            && mouse.column >= tab_area.x
                            && mouse.column < tab_area.x.saturating_add(tab_area.width) =>
                    {
                        if let Some(tab) = TuiTab::from_mouse(mouse.column, mouse.row, tab_area) {
                            change_tui_tab(
                                &mut active_tab,
                                tab,
                                &mut filter_query,
                                &mut tab_filters,
                            );
                        }
                    }
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if key.code == KeyCode::Char(' ')
                            && !matches!(mode, TuiMode::FilterInput(_))
                        {
                            tui_paused = !tui_paused;
                            continue;
                        }
                        if key.code == KeyCode::Char('/')
                            && !matches!(mode, TuiMode::FilterInput(_))
                        {
                            tui_paused = false;
                            mode = TuiMode::FilterInput(filter_query.clone());
                            continue;
                        }
                        match mode {
                            TuiMode::Normal => match key.code {
                                KeyCode::Char('q') => break,
                                code if TuiTab::from_key(code).is_some() => {
                                    let next = TuiTab::from_key(code).unwrap_or(active_tab);
                                    change_tui_tab(
                                        &mut active_tab,
                                        next,
                                        &mut filter_query,
                                        &mut tab_filters,
                                    );
                                }
                                KeyCode::Left | KeyCode::Char('[') => {
                                    let next = active_tab.previous();
                                    change_tui_tab(
                                        &mut active_tab,
                                        next,
                                        &mut filter_query,
                                        &mut tab_filters,
                                    );
                                }
                                KeyCode::Right | KeyCode::Char(']') => {
                                    let next = active_tab.next();
                                    change_tui_tab(
                                        &mut active_tab,
                                        next,
                                        &mut filter_query,
                                        &mut tab_filters,
                                    );
                                }
                                KeyCode::Char('p')
                                    if matches!(
                                        active_tab,
                                        TuiTab::Interfaces | TuiTab::Traffic
                                    ) =>
                                {
                                    if active_tab == TuiTab::Traffic {
                                        protocol_filter = None;
                                        change_tui_tab(
                                            &mut active_tab,
                                            TuiTab::Interfaces,
                                            &mut filter_query,
                                            &mut tab_filters,
                                        );
                                    } else {
                                        protocol_filter = current_interfaces
                                            .get(interface_idx)
                                            .filter(|iface| {
                                                tui_filter_matches(
                                                    &filter_query,
                                                    &[
                                                        &iface.if_name,
                                                        &iface.if_index.to_string(),
                                                        &iface.link_status,
                                                        &iface.neighbor,
                                                    ],
                                                )
                                            })
                                            .map(|iface| iface.if_index);
                                        change_tui_tab(
                                            &mut active_tab,
                                            TuiTab::Traffic,
                                            &mut filter_query,
                                            &mut tab_filters,
                                        );
                                    }
                                }
                                KeyCode::PageUp | KeyCode::PageDown
                                    if active_tab == TuiTab::Interfaces =>
                                {
                                    let next = if key.code == KeyCode::PageUp {
                                        selected_device_idx.saturating_sub(1)
                                    } else {
                                        (selected_device_idx + 1)
                                            .min(devices.len().saturating_sub(1))
                                    };
                                    if next != selected_device_idx {
                                        selected_device_idx = next;
                                        current_interfaces = self.load_interface_summaries(
                                            &devices[next].device,
                                            &repository,
                                        );
                                        interface_idx = 0;
                                        protocol_filter = None;
                                    }
                                }
                                KeyCode::Char('c') if active_tab == TuiTab::Traffic => {
                                    protocol_filter = None;
                                    filter_query.clear();
                                }
                                KeyCode::Char('c') => {
                                    filter_query.clear();
                                }
                                KeyCode::Char('w') if active_tab == TuiTab::Traffic => {
                                    protocol_window = match protocol_window {
                                        60 => 300,
                                        300 => 3600,
                                        _ => 60,
                                    };
                                }
                                KeyCode::Char('s') if active_tab == TuiTab::Traffic => {
                                    protocol_sort = protocol_sort.next();
                                }
                                KeyCode::Enter if active_tab == TuiTab::Devices => {
                                    if devices.get(selected_device_idx).is_some_and(|item| {
                                        tui_filter_matches(
                                            &filter_query,
                                            &[
                                                &item.device.ip,
                                                &item.device.name,
                                                &item.device.status,
                                            ],
                                        )
                                    }) {
                                        change_tui_tab(
                                            &mut active_tab,
                                            TuiTab::Interfaces,
                                            &mut filter_query,
                                            &mut tab_filters,
                                        );
                                    }
                                }
                                KeyCode::Char('d') => {
                                    let default_comm =
                                        if self.runner.config.snmp.default_community.is_empty() {
                                            "public".to_string()
                                        } else {
                                            self.runner.config.snmp.default_community.clone()
                                        };
                                    let cidr = guess_local_cidr();
                                    let cursor_pos = cidr.chars().count();
                                    mode = TuiMode::DiscoveryModal(DiscoveryModalState {
                                        active_field: 0,
                                        cidr,
                                        community: default_comm,
                                        version: "v2c".to_string(),
                                        cursor_position: cursor_pos,
                                        error_msg: None,
                                    });
                                }
                                KeyCode::Char('n') => {
                                    if let Some(provider) = self.runner.notification_provider() {
                                        mode = TuiMode::Notifications(NotificationStatusState {
                                            status: provider.status(),
                                            selected: 0,
                                            test: None,
                                            message: None,
                                            message_is_error: false,
                                        });
                                    }
                                }
                                KeyCode::Up
                                | KeyCode::Char('k')
                                | KeyCode::Down
                                | KeyCode::Char('j') => {
                                    let forward =
                                        matches!(key.code, KeyCode::Down | KeyCode::Char('j'));
                                    match active_tab {
                                        TuiTab::Devices => {
                                            let visible = devices
                                                .iter()
                                                .enumerate()
                                                .filter(|(_, item)| {
                                                    tui_filter_matches(
                                                        &filter_query,
                                                        &[
                                                            &item.device.ip,
                                                            &item.device.name,
                                                            &item.device.status,
                                                        ],
                                                    )
                                                })
                                                .map(|(index, _)| index)
                                                .collect::<Vec<_>>();
                                            if let Some(next) = move_visible_selection(
                                                &visible,
                                                selected_device_idx,
                                                forward,
                                            ) && next != selected_device_idx
                                            {
                                                selected_device_idx = next;
                                                if let Some(dev) = devices.get(selected_device_idx)
                                                {
                                                    current_interfaces = self
                                                        .load_interface_summaries(
                                                            &dev.device,
                                                            &repository,
                                                        );
                                                }
                                                interface_idx = 0;
                                            }
                                        }
                                        TuiTab::Interfaces => {
                                            let visible = current_interfaces
                                                .iter()
                                                .enumerate()
                                                .filter(|(_, iface)| {
                                                    tui_filter_matches(
                                                        &filter_query,
                                                        &[
                                                            &iface.if_name,
                                                            &iface.if_index.to_string(),
                                                            &iface.link_status,
                                                            &iface.neighbor,
                                                        ],
                                                    )
                                                })
                                                .map(|(index, _)| index)
                                                .collect::<Vec<_>>();
                                            if let Some(next) = move_visible_selection(
                                                &visible,
                                                interface_idx,
                                                forward,
                                            ) {
                                                interface_idx = next;
                                            }
                                        }
                                        TuiTab::Traffic => {
                                            protocol_talker_idx = if forward {
                                                protocol_talker_idx.saturating_add(1)
                                            } else {
                                                protocol_talker_idx.saturating_sub(1)
                                            };
                                        }
                                        TuiTab::Alerts => {
                                            let visible = alerts
                                                .iter()
                                                .enumerate()
                                                .filter(|(_, alert)| {
                                                    tui_filter_matches(
                                                        &filter_query,
                                                        &[
                                                            &alert.device_name,
                                                            &alert.device_ip,
                                                            &alert.alert_type,
                                                            &alert.severity,
                                                            &alert.details,
                                                        ],
                                                    )
                                                })
                                                .map(|(index, _)| index)
                                                .collect::<Vec<_>>();
                                            if let Some(next) =
                                                move_visible_selection(&visible, alert_idx, forward)
                                            {
                                                alert_idx = next;
                                            }
                                        }
                                        TuiTab::Dashboard => {}
                                    }
                                }
                                KeyCode::Enter if active_tab == TuiTab::Alerts => {
                                    if let Some(alert) = alerts.get(alert_idx).filter(|alert| {
                                        tui_filter_matches(
                                            &filter_query,
                                            &[
                                                &alert.device_name,
                                                &alert.device_ip,
                                                &alert.alert_type,
                                                &alert.severity,
                                                &alert.details,
                                            ],
                                        )
                                    }) {
                                        mode = TuiMode::AlertDetails(alert.clone());
                                    }
                                }
                                KeyCode::Enter if active_tab == TuiTab::Traffic => {
                                    let exporter_ip = devices
                                        .get(selected_device_idx)
                                        .map(|item| item.device.ip.as_str());
                                    let mut talkers = flow_repository
                                        .as_ref()
                                        .and_then(|repo| {
                                            repo.top_talkers_for_context(
                                                protocol_window,
                                                5,
                                                exporter_ip,
                                                protocol_filter,
                                                protocol_sort.query_key(),
                                            )
                                            .ok()
                                        })
                                        .unwrap_or_else(|| {
                                            repository
                                                .top_talkers_for_context(
                                                    protocol_window,
                                                    5,
                                                    exporter_ip,
                                                    protocol_filter,
                                                    protocol_sort.query_key(),
                                                )
                                                .unwrap_or_default()
                                        });
                                    match protocol_sort {
                                        ProtocolSort::Bps => {
                                            talkers.sort_by_key(|a| std::cmp::Reverse(a.bps))
                                        }
                                        ProtocolSort::Pps => {
                                            talkers.sort_by_key(|a| std::cmp::Reverse(a.pps))
                                        }
                                        ProtocolSort::Bytes => {
                                            talkers.sort_by_key(|a| std::cmp::Reverse(a.bytes))
                                        }
                                    }
                                    let visible = talkers
                                        .into_iter()
                                        .filter(|talker| {
                                            tui_filter_matches(
                                                &filter_query,
                                                &[
                                                    &talker.source_ip,
                                                    &talker.destination_ip,
                                                    &talker.protocol,
                                                    &talker.app_name,
                                                ],
                                            )
                                        })
                                        .collect::<Vec<_>>();
                                    if let Some(talker) =
                                        visible.into_iter().nth(protocol_talker_idx)
                                    {
                                        mode = TuiMode::FlowInspector(talker);
                                    }
                                }
                                KeyCode::Enter if active_tab == TuiTab::Interfaces => {
                                    if let Some(iface) =
                                        current_interfaces.get(interface_idx).filter(|iface| {
                                            tui_filter_matches(
                                                &filter_query,
                                                &[
                                                    &iface.if_name,
                                                    &iface.if_index.to_string(),
                                                    &iface.link_status,
                                                    &iface.neighbor,
                                                ],
                                            )
                                        })
                                    {
                                        let device_id = devices
                                            .get(selected_device_idx)
                                            .and_then(|d| d.device.id)
                                            .unwrap_or(0);
                                        let delta = repository
                                            .get_interface_port_deltas(device_id)
                                            .ok()
                                            .and_then(|m| m.get(&iface.if_index).copied())
                                            .unwrap_or_default();
                                        mode = TuiMode::PortErrorBreakdown(
                                            PortErrorBreakdownState::from_delta(
                                                iface.if_name.clone(),
                                                iface.if_index,
                                                delta,
                                            ),
                                        );
                                    }
                                }
                                KeyCode::Char('r') => {
                                    match start_poll_task(
                                        self.runner.config.clone(),
                                        self.runner.alert_broadcaster(),
                                        self.runner.database_path(),
                                    ) {
                                        Ok(poll_task) => {
                                            mode = TuiMode::Polling(poll_task);
                                        }
                                        Err(err) => {
                                            status_msg = format!("Poll failed: {}", err);
                                        }
                                    }
                                }
                                _ => {}
                            },
                            TuiMode::FilterInput(ref mut query) => match key.code {
                                KeyCode::Esc | KeyCode::Enter => {
                                    mode = TuiMode::Normal;
                                }
                                KeyCode::Char('c') => {
                                    query.clear();
                                    filter_query.clear();
                                    mode = TuiMode::Normal;
                                }
                                KeyCode::Backspace => {
                                    query.pop();
                                    filter_query.clone_from(query);
                                }
                                KeyCode::Char(character) => {
                                    query.push(character);
                                    filter_query.clone_from(query);
                                }
                                _ => {}
                            },
                            TuiMode::FlowInspector(_) => match key.code {
                                KeyCode::Esc | KeyCode::Enter => {
                                    mode = TuiMode::Normal;
                                }
                                _ => {}
                            },
                            TuiMode::AlertDetails(_) => match key.code {
                                KeyCode::Esc | KeyCode::Enter => {
                                    mode = TuiMode::Normal;
                                }
                                _ => {}
                            },
                            TuiMode::PortErrorBreakdown(_) => match key.code {
                                KeyCode::Esc | KeyCode::Enter => {
                                    mode = TuiMode::Normal;
                                }
                                _ => {}
                            },
                            TuiMode::Notifications(ref mut state) => match key.code {
                                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('n') => {
                                    mode = TuiMode::Normal;
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if state.selected > 0 {
                                        state.selected -= 1;
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if state.selected + 1 < state.status.channels.len() {
                                        state.selected += 1;
                                    }
                                }
                                KeyCode::Char('t') | KeyCode::Enter | KeyCode::Char('a')
                                    if state.test.is_none() =>
                                {
                                    let target = if key.code == KeyCode::Char('a') {
                                        Some("all".to_string())
                                    } else {
                                        state
                                            .status
                                            .channels
                                            .get(state.selected)
                                            .map(|channel| channel.key.clone())
                                    };

                                    if let (Some(target), Some(provider)) =
                                        (target, self.runner.notification_provider())
                                    {
                                        state.message = None;
                                        state.test =
                                            Some(start_notification_test(provider, target));
                                    }
                                }
                                _ => {}
                            },
                            TuiMode::DiscoveryModal(ref mut modal) => match key.code {
                                KeyCode::Esc => {
                                    mode = TuiMode::Normal;
                                }
                                KeyCode::Tab | KeyCode::Down => {
                                    modal.set_field(modal.active_field + 1);
                                }
                                KeyCode::BackTab | KeyCode::Up => {
                                    modal.set_field(modal.active_field + 2);
                                }
                                KeyCode::Left => {
                                    modal.move_cursor_left();
                                }
                                KeyCode::Right => {
                                    modal.move_cursor_right();
                                }
                                KeyCode::Home => {
                                    modal.cursor_position = 0;
                                }
                                KeyCode::End => {
                                    modal.cursor_position =
                                        modal.current_field_ref().chars().count();
                                }
                                KeyCode::Char(c) => {
                                    modal.insert_char(c);
                                }
                                KeyCode::Backspace => {
                                    modal.delete_backspace();
                                }
                                KeyCode::Delete => {
                                    modal.delete_char();
                                }
                                KeyCode::Enter => match start_scan(&modal.cidr, &modal.community) {
                                    Ok(task) => {
                                        mode = TuiMode::Scanning(task);
                                    }
                                    Err(err) => {
                                        modal.error_msg = Some(err.to_string());
                                    }
                                },
                                _ => {}
                            },
                            TuiMode::Scanning(_) | TuiMode::Polling(_) => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn load_device_summaries(
        &self,
        repository: &Repository,
    ) -> Result<Vec<DeviceSummary>, AppError> {
        let db_devices = repository.list_devices()?;
        let mut summaries = Vec::new();

        for dev in db_devices {
            let dev_id = dev.id.unwrap_or(0);
            let metrics = repository.get_device_metrics_history(dev_id, 1).ok();
            let cpu = metrics
                .as_ref()
                .and_then(|m| m.first().and_then(|v| v.cpu_usage));
            let mem = metrics
                .as_ref()
                .and_then(|m| m.first().and_then(|v| v.memory_usage));

            let ifaces = repository.get_latest_interfaces(dev_id).unwrap_or_default();
            let total_ports = ifaces.len();
            let active_ports = ifaces
                .iter()
                .filter(|i| {
                    let status = effective_interface_link_status(&dev.status, &i.link_status);
                    status.to_lowercase() == "up" || status == "1"
                })
                .count();

            let alert_count = repository
                .get_alert_history(dev_id, 100)
                .map(|a| a.len())
                .unwrap_or(0);

            summaries.push(DeviceSummary {
                device: dev,
                cpu_usage: cpu,
                memory_usage: mem,
                active_ports,
                total_ports,
                alert_count,
            });
        }

        Ok(summaries)
    }

    fn load_interface_summaries(
        &self,
        device: &Device,
        repository: &Repository,
    ) -> Vec<InterfaceSummary> {
        let dev_id = device.id.unwrap_or(0);
        let ifaces = repository.get_latest_interfaces(dev_id).unwrap_or_default();

        let is_offline = device.status.eq_ignore_ascii_case("offline");

        let neighbors = if is_offline {
            // offline 機器への SNMP 接続試行によるタイムアウト遅延を回避
            std::collections::HashMap::new()
        } else {
            let cfg = DeviceConfig {
                id: device.id,
                name: device.name.clone(),
                ip: device.ip.clone(),
                community: device.community.clone(),
                device_type: device.device_type.clone(),
                status: device.status.clone(),
                last_seen_at: device.last_seen_at.clone(),
            };

            let client = SnmpClient::with_snmp_config(
                if device.community.is_empty() {
                    self.runner.config.snmp.default_community.clone()
                } else {
                    device.community.clone()
                },
                self.runner.config.snmp.clone(),
            );

            client.query_device_port_neighbors(&cfg)
        };

        ifaces
            .into_iter()
            .map(|iface| {
                let neighbor = neighbors
                    .get(&iface.if_index)
                    .cloned()
                    .unwrap_or_else(|| "N/A".to_string());

                let effective_status =
                    effective_interface_link_status(&device.status, &iface.link_status);
                let bandwidth = if is_offline {
                    0.0
                } else {
                    iface.bandwidth_utilization
                };
                InterfaceSummary {
                    if_index: iface.if_index,
                    if_name: iface.if_name,
                    link_status: effective_status,
                    bandwidth_utilization: bandwidth,
                    in_errors: iface.in_errors,
                    out_errors: iface.out_errors,
                    in_discards: iface.in_discards,
                    out_discards: iface.out_discards,
                    late_collisions: iface.late_collisions,
                    neighbor,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traffic_header_fits_terminal_and_separates_columns() {
        let header = flow_header_line();
        assert!(header.chars().count() <= 95);
        assert!(!header.contains("PPSFLAGS"));
        let talker = TopTalker {
            source_ip: "192.0.2.10".to_string(),
            destination_ip: "198.51.100.20".to_string(),
            source_port: 49152,
            destination_port: 443,
            protocol: "TCP".to_string(),
            bytes: 4096,
            bps: 8192,
            packets: 12,
            pps: 34,
            app_name: "HTTPS".to_string(),
            ingress_if_index: Some(1),
            egress_if_index: Some(2),
            ingress_if_name: None,
            egress_if_name: None,
            tcp_flags: 0x12,
        };
        assert!(flow_row_line(&talker).chars().count() <= 95);
        assert_eq!(format_tui_count(0.0), "0");
    }

    #[test]
    fn tui_tabs_wrap_and_map_numeric_keys() {
        assert_eq!(TuiTab::Dashboard.previous(), TuiTab::Alerts);
        assert_eq!(TuiTab::Alerts.next(), TuiTab::Dashboard);
        assert_eq!(
            TuiTab::from_key(KeyCode::Char('1')),
            Some(TuiTab::Dashboard)
        );
        assert_eq!(TuiTab::from_key(KeyCode::Char('5')), Some(TuiTab::Alerts));
        assert_eq!(TuiTab::from_key(KeyCode::Char('0')), None);

        let area = Rect::new(0, 0, 50, 3);
        assert_eq!(TuiTab::from_mouse(4, 1, area), Some(TuiTab::Dashboard));
        assert_eq!(TuiTab::from_mouse(12, 1, area), None);
        assert_eq!(TuiTab::from_mouse(13, 1, area), Some(TuiTab::Devices));
        assert_eq!(TuiTab::from_mouse(30, 1, area), Some(TuiTab::Interfaces));
        assert_eq!(TuiTab::from_mouse(39, 1, area), Some(TuiTab::Traffic));
        assert_eq!(TuiTab::from_mouse(47, 1, area), Some(TuiTab::Alerts));
        assert_eq!(TuiTab::from_mouse(47, 0, area), None);
        assert_eq!(TuiTab::from_mouse(49, 1, area), None);
        assert_eq!(TuiTab::from_mouse(47, 1, Rect::new(0, 0, 40, 3)), None);
    }

    #[test]
    fn filtered_selection_moves_only_through_visible_rows() {
        let visible = [1, 4, 8];
        assert_eq!(move_visible_selection(&visible, 1, true), Some(4));
        assert_eq!(move_visible_selection(&visible, 4, false), Some(1));
        assert_eq!(move_visible_selection(&visible, 8, true), Some(8));
        assert_eq!(move_visible_selection(&visible, 0, true), Some(1));
        assert_eq!(move_visible_selection(&[], 0, true), None);
    }

    #[test]
    fn tab_switch_restores_each_tabs_filter() {
        let mut active = TuiTab::Devices;
        let mut query = "router".to_string();
        let mut filters = std::array::from_fn(|_| String::new());
        change_tui_tab(&mut active, TuiTab::Interfaces, &mut query, &mut filters);
        assert_eq!(query, "");
        query = "uplink".to_string();
        change_tui_tab(&mut active, TuiTab::Devices, &mut query, &mut filters);
        assert_eq!(query, "router");
        change_tui_tab(&mut active, TuiTab::Interfaces, &mut query, &mut filters);
        assert_eq!(query, "uplink");
    }

    #[test]
    fn notify_shortcut_requires_provider() {
        assert!(!tui_help_text(false).contains("[n] Notify"));
        assert!(tui_help_text(true).contains("[n] Notify"));
    }

    #[test]
    fn test_offline_device_forces_interface_status_down() {
        assert_eq!(effective_interface_link_status("offline", "UP"), "down");
        assert_eq!(effective_interface_link_status("offline", "1"), "down");
        assert_eq!(effective_interface_link_status("OFFLINE", "UP"), "down");
    }

    #[test]
    fn formats_alert_timestamps_in_configured_timezone() {
        let timestamp = "2026-09-07T10:42:18+00:00";

        assert_eq!(
            format_tui_timestamp(timestamp, "utc"),
            "2026-09-07 10:42:18"
        );
        assert_eq!(
            format_tui_timestamp(timestamp, "jst"),
            "2026-09-07 19:42:18"
        );
    }

    #[test]
    fn test_online_device_preserves_interface_status() {
        assert_eq!(effective_interface_link_status("online", "UP"), "UP");
        assert_eq!(effective_interface_link_status("online", "DOWN"), "DOWN");
    }

    #[test]
    fn test_guess_local_cidr_returns_valid_cidr() {
        let cidr = guess_local_cidr();
        assert!(cidr.contains("/24"));
        assert!(crate::device::discovery::parse_cidr(&cidr).is_ok());
    }

    #[test]
    fn test_discovery_modal_cursor_navigation_and_editing() {
        let mut modal = DiscoveryModalState {
            active_field: 0,
            cidr: "192.168.1.0/24".to_string(),
            community: "public".to_string(),
            version: "v2c".to_string(),
            cursor_position: 14,
            error_msg: None,
        };

        // Move left 3 times
        modal.move_cursor_left();
        modal.move_cursor_left();
        modal.move_cursor_left();
        assert_eq!(modal.cursor_position, 11);

        // Delete '0' before /24 -> "192.168.1./24"
        modal.delete_backspace();
        assert_eq!(modal.cidr, "192.168.1./24");
        assert_eq!(modal.cursor_position, 10);

        // Insert '1' -> "192.168.1.1/24"
        modal.insert_char('1');
        assert_eq!(modal.cidr, "192.168.1.1/24");
        assert_eq!(modal.cursor_position, 11);
    }

    #[test]
    fn test_discovery_modal_snmp_version_toggle() {
        let mut modal = DiscoveryModalState {
            active_field: 2,
            cidr: "192.168.1.0/24".to_string(),
            community: "public".to_string(),
            version: "v2c".to_string(),
            cursor_position: 3,
            error_msg: None,
        };

        // Toggle next: v2c -> v1
        modal.move_cursor_right();
        assert_eq!(modal.version, "v1");

        // Toggle next: v1 -> v3
        modal.move_cursor_right();
        assert_eq!(modal.version, "v3");

        // Toggle next: v3 -> v2c
        modal.move_cursor_right();
        assert_eq!(modal.version, "v2c");

        // Direct key press '1', '2', '3'
        modal.insert_char('1');
        assert_eq!(modal.version, "v1");
        modal.insert_char('3');
        assert_eq!(modal.version, "v3");
        modal.insert_char('2');
        assert_eq!(modal.version, "v2c");
    }
}
