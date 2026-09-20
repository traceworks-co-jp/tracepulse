use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, TableState, Wrap},
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
        "{:<20}{:<4}{:<22}{:<7}{:>9}{:>9}{:>7}{:<8}{:<20}",
        "SOURCE IP:PORT",
        "->",
        "DESTINATION IP:PORT",
        "PROTO",
        "BYTES",
        "BPS",
        "PPS",
        "FLAGS",
        "IN/OUT IF"
    )
}

fn flow_row_line(talker: &TopTalker, selected: bool) -> String {
    let source = format!(
        "{}{}",
        if selected { ">" } else { " " },
        truncate_tui(&format!("{}:{}", talker.source_ip, talker.source_port), 19).trim_end()
    );
    let destination = truncate_tui(
        &format!("{}:{}", talker.destination_ip, talker.destination_port),
        22,
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
        "{:<20}{:<4}{:<22}{:<7}{:>9}{:>9}{:>7}{:<8}{:<20}",
        source,
        "->",
        destination,
        talker.protocol.chars().take(6).collect::<String>(),
        format_tui_bytes(talker.bytes),
        format_tui_rate(talker.bps as f64),
        talker.pps,
        format_tui_flags(talker.tcp_flags),
        format!("{}/{}", ingress, egress)
            .chars()
            .take(20)
            .collect::<String>(),
    )
}

fn protocol_app_header_line() -> String {
    format!("{:<36}│ {}", "Top Protocols", "Top Applications")
}

fn protocol_app_row_line(protocol: &str, application: &str) -> String {
    format!("{:<36}│ {}", protocol, application)
}

fn tui_filter_matches(filter: &str, values: &[&str]) -> bool {
    let needle = filter.trim().to_lowercase();
    needle.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(&needle))
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

fn tui_help_text() -> String {
    "Ready. [↑/↓/j/k] Move | [Tab] Devices/Interfaces | [Enter] Details\n[p] Protocol | [c] Clear | [1/2/3] Window | [s] Sort | [Space] Pause\n[/] Filter | [r] Poll | [d] Discovery | [n] Notify | [q] Quit".to_string()
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
) -> Result<PollTask, AppError> {
    let repository = Repository::new(
        rusqlite::Connection::open(crate::exe_dir().join("data.db"))
            .map_err(|e| AppError::Database(e.to_string()))?,
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
        execute!(stdout, EnterAlternateScreen).map_err(|e| AppError::Io(e.to_string()))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|e| AppError::Io(e.to_string()))?;
        // レイアウトの margin により端の行・列は描画されないため、切替前の画面の残骸を消す。
        terminal.clear().map_err(|e| AppError::Io(e.to_string()))?;

        let res = self.run_app(&mut terminal);

        // クリーンアップ処理
        let _ = disable_raw_mode();
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
        let _ = terminal.show_cursor();

        res
    }

    fn run_app<B: ratatui::backend::Backend>(
        &self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), AppError> {
        if self.runner.flow_repository().is_none() {
            let repository = Repository::new(
                rusqlite::Connection::open(crate::exe_dir().join("data.db"))
                    .expect("SQLite flow repository should open"),
            );
            let repository = Arc::new(Mutex::new(repository));
            crate::flow::FlowCollector::start(
                Arc::clone(&repository),
                crate::flow::FlowCollectorConfig::from_app_config(&self.runner.config),
            );
        }
        let repository = Repository::new(
            rusqlite::Connection::open(crate::exe_dir().join("data.db"))
                .map_err(|e| AppError::Database(e.to_string()))?,
        );

        // TUI モードでも定期監視とアラート配信を行う（Web モードと同じポーリングループ）。
        {
            let polling_config = Arc::new(Mutex::new(self.runner.config.clone()));
            let polling_repository = Arc::new(Mutex::new(Repository::new(
                rusqlite::Connection::open(crate::exe_dir().join("data.db"))
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
        let mut pane = Pane::Devices;
        let mut protocol_filter: Option<i32> = None;
        let mut protocol_window = 60_i64;
        let mut protocol_sort = ProtocolSort::Bps;
        let mut filter_query = String::new();
        let mut protocol_talker_idx = 0_usize;
        let tui_language = self.runner.config.display.language.as_str();
        let mut tui_paused = false;
        let mut interface_idx: usize = 0;
        let mut status_msg = tui_help_text();
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
                match start_poll_task(self.runner.config.clone(), self.runner.alert_broadcaster()) {
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
                                Constraint::Percentage(32), // 上ペイン: Device List
                                Constraint::Percentage(42), // 中ペイン: Interface & Topology
                                Constraint::Percentage(20), // 下ペイン: Alert Stream
                                Constraint::Length(footer_height), // フッター / フィルター入力
                            ]
                            .as_ref(),
                        )
                        .split(f.area());

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

                    let rows = devices.iter().filter(|item| {
                        tui_filter_matches(&filter_query, &[&item.device.ip, &item.device.name, &item.device.status])
                    }).map(|item| {
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

                    f.render_stateful_widget(device_table, chunks[0], &mut table_state);

                    // 2. 中ペイン: Interface & Topology / Protocol View
                    let selected_device_name = devices
                        .get(selected_device_idx)
                        .map(|d| format!("{} ({})", d.device.name, d.device.ip))
                        .unwrap_or_else(|| "No Device Selected".to_string());

                    if pane == Pane::Protocol {
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
                        let protocol_lines = shares.iter().map(|share| {
                            format!("{:<5} {:<16} {:>5.1}%", share.protocol, tui_progress_bar(share.percentage, 16), share.percentage)
                        }).collect::<Vec<_>>();
                        let application_lines = applications.iter().enumerate().map(|(index, app)| {
                            format!("{}. {:<14} {:<12} {:>5.1}%", index + 1, truncate_tui(&app.app_name, 14).trim(), tui_progress_bar(app.percentage, 12), app.percentage)
                        }).collect::<Vec<_>>();
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
                        let mut lines = vec![
                            "TracePulse TUI Monitor [Live 5s]          Protocol: NetFlow/IPFIX/sFlow".to_string(),
                            format!("Throughput: {:>8} | Packets: {:>8} | Active Flows: {:>6} | Active Exporters: {:>3}", format_tui_rate(summary.total_bps), format_tui_rate(summary.total_pps), summary.active_flows, active_exporters),
                            "────────────────────────────────────────────────────────────────────────────────".to_string(),
                            protocol_app_header_line(),
                        ];
                        for index in 0..protocol_lines.len().max(application_lines.len()).min(3) {
                            let left = protocol_lines.get(index).map(String::as_str).unwrap_or("");
                            let right = application_lines.get(index).map(String::as_str).unwrap_or("");
                            lines.push(protocol_app_row_line(left, right));
                        }
                        lines.extend([
                            String::new(),
                            format!("{} (Sort: {} down)", tui_text(tui_language, "top_talkers"), protocol_sort.label()),
                            flow_header_line(),
                        ]);
                        let mut selected_row = None;
                        if talkers.is_empty() {
                            lines.push(tui_text(tui_language, "no_flows").to_string());
                        } else {
                            for (index, talker) in visible_talkers.iter().enumerate() {
                                if index == protocol_talker_idx {
                                    selected_row = Some(lines.len());
                                }
                                lines.push(flow_row_line(talker, false));
                            }
                        }
                        lines.push(tui_text(tui_language, "corrected_note").to_string());
                        let styled_lines = lines.into_iter().enumerate().map(|(index, line)| {
                            let line = Line::from(line);
                            if selected_row == Some(index) {
                                line.style(Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))
                            } else {
                                line
                            }
                        }).collect::<Vec<_>>();
                        let visible_height = chunks[1].height.saturating_sub(2) as usize;
                        let scroll_offset = selected_row
                            .map(|row| row.saturating_sub(visible_height.saturating_sub(1)))
                            .unwrap_or(0);
                        f.render_widget(
                            Paragraph::new(Text::from(styled_lines))
                                .block(Block::default().borders(Borders::ALL).title(format!(" {} - [{} / {}s] ", tui_text(tui_language, "protocol_traffic"), filter_label, protocol_window)))
                                .scroll((scroll_offset.min(u16::MAX as usize) as u16, 0))
                                .wrap(Wrap { trim: true })
                                .style(Style::default().fg(Color::White)),
                            chunks[1],
                        );
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

                    let if_rows = current_interfaces.iter().filter(|iface| {
                        tui_filter_matches(&filter_query, &[&iface.if_name, &iface.if_index.to_string(), &iface.link_status, &iface.neighbor])
                    }).map(|iface| {
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
                                if pane == Pane::Interfaces { " [FOCUS: Enter=Error Breakdown]" } else { "" }
                            )),
                    )
                    .highlight_style(
                        if pane == Pane::Interfaces {
                            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        }
                    )
                    .highlight_symbol(">> ");

                    f.render_stateful_widget(if_table, chunks[1], &mut if_table_state);
                    }

                    // 3. 下ペイン: Alert Stream
                    let alert_header_cells = ["Time", "Device Name (IP)", "Type", "Severity", "Details"]
                        .iter()
                        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)));
                    let alert_header = Row::new(alert_header_cells)
                        .style(Style::default().bg(Color::DarkGray))
                        .height(1);

                    let alert_rows = alerts.iter().map(|alert| {
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
                    );

                    f.render_widget(alert_table, chunks[2]);

                    // 4. フッター / ステータスバー
                    let footer = Paragraph::new(status_msg.clone())
                        .wrap(Wrap { trim: true })
                        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
                    f.render_widget(footer, chunks[3]);

                    // モーダル・ダイアログ オーバーレイ描画
                    match mode {
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
                            f.render_widget(Clear, chunks[3]);
                            let prompt = Paragraph::new(format!(
                                "Filter: {}\n[Enter] Apply  [Esc] Cancel  [c] Clear",
                                query
                            ))
                                .block(Block::default().borders(Borders::ALL).title(" Incremental Filter "))
                                .style(Style::default().bg(Color::DarkGray).fg(Color::White));
                            f.render_widget(prompt, chunks[3]);
                            f.set_cursor_position((chunks[3].x + 1 + 8 + query.chars().count() as u16, chunks[3].y + 1));
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
            if event::poll(Duration::from_millis(50)).map_err(|e| AppError::Io(e.to_string()))?
                && let Event::Key(key) = event::read().map_err(|e| AppError::Io(e.to_string()))?
                && key.kind == KeyEventKind::Press
            {
                if status_msg.starts_with("Alert notifications are available") {
                    status_msg = tui_help_text();
                }
                if key.code == KeyCode::Char(' ') && !matches!(mode, TuiMode::FilterInput(_)) {
                    tui_paused = !tui_paused;
                    continue;
                }
                if key.code == KeyCode::Char('/') && !matches!(mode, TuiMode::FilterInput(_)) {
                    tui_paused = false;
                    mode = TuiMode::FilterInput(filter_query.clone());
                    continue;
                }
                match mode {
                    TuiMode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Tab => {
                            pane = match pane {
                                Pane::Devices => Pane::Interfaces,
                                Pane::Interfaces => Pane::Devices,
                                Pane::Protocol => Pane::Devices,
                            };
                            if pane == Pane::Interfaces {
                                if interface_idx >= current_interfaces.len() {
                                    interface_idx = current_interfaces.len().saturating_sub(1);
                                }
                                if !current_interfaces.is_empty() {
                                    if_table_state.select(Some(interface_idx));
                                }
                            }
                        }
                        KeyCode::Char('p') => {
                            if pane == Pane::Protocol {
                                protocol_filter = None;
                                pane = Pane::Interfaces;
                            } else {
                                protocol_filter = if pane == Pane::Interfaces {
                                    current_interfaces
                                        .get(interface_idx)
                                        .map(|iface| iface.if_index)
                                } else {
                                    None
                                };
                                pane = Pane::Protocol;
                            }
                        }
                        KeyCode::Char('c') if pane == Pane::Protocol => {
                            protocol_filter = None;
                            filter_query.clear();
                        }
                        KeyCode::Char('c') => {
                            filter_query.clear();
                        }
                        KeyCode::Char('1') if pane == Pane::Protocol => {
                            protocol_window = 60;
                        }
                        KeyCode::Char('2') if pane == Pane::Protocol => {
                            protocol_window = 300;
                        }
                        KeyCode::Char('3') if pane == Pane::Protocol => {
                            protocol_window = 3600;
                        }
                        KeyCode::Char('s') if pane == Pane::Protocol => {
                            protocol_sort = protocol_sort.next();
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
                        KeyCode::Char('n') => match self.runner.notification_provider() {
                            Some(provider) => {
                                mode = TuiMode::Notifications(NotificationStatusState {
                                    status: provider.status(),
                                    selected: 0,
                                    test: None,
                                    message: None,
                                    message_is_error: false,
                                });
                            }
                            None => {
                                status_msg = String::from(
                                    "Alert notifications are available in the Enterprise edition.",
                                );
                            }
                        },
                        KeyCode::Up | KeyCode::Char('k') => match pane {
                            Pane::Devices => {
                                if selected_device_idx > 0 {
                                    selected_device_idx -= 1;
                                    table_state.select(Some(selected_device_idx));
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
                                }
                            }
                            Pane::Interfaces => {
                                if interface_idx > 0 {
                                    interface_idx -= 1;
                                    if_table_state.select(Some(interface_idx));
                                }
                            }
                            Pane::Protocol => {
                                protocol_talker_idx = protocol_talker_idx.saturating_sub(1);
                            }
                        },
                        KeyCode::Down | KeyCode::Char('j') => match pane {
                            Pane::Devices => {
                                if !devices.is_empty() && selected_device_idx + 1 < devices.len() {
                                    selected_device_idx += 1;
                                    table_state.select(Some(selected_device_idx));
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
                                }
                            }
                            Pane::Interfaces => {
                                if !current_interfaces.is_empty()
                                    && interface_idx + 1 < current_interfaces.len()
                                {
                                    interface_idx += 1;
                                    if_table_state.select(Some(interface_idx));
                                }
                            }
                            Pane::Protocol => {
                                protocol_talker_idx = protocol_talker_idx.saturating_add(1);
                            }
                        },
                        KeyCode::Enter if pane == Pane::Protocol => {
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
                            if let Some(talker) = visible.into_iter().nth(protocol_talker_idx) {
                                mode = TuiMode::FlowInspector(talker);
                            }
                        }
                        KeyCode::Enter if pane == Pane::Interfaces => {
                            if let Some(iface) = current_interfaces.get(interface_idx) {
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
                                state.test = Some(start_notification_test(provider, target));
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
                            modal.cursor_position = modal.current_field_ref().chars().count();
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
