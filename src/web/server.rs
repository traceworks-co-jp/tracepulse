use crate::alert::AlertBroadcaster;
use crate::config::AppConfig;
use crate::db::models::InterfacePortDelta;
use crate::db::repository::{Repository, counter32_delta};
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::notifications::NotificationSettingsProvider;
use crate::snmp::SnmpClient;
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ─── Default web limits ──────────────────────────────────────────────────────
const COMMUNITY_MAX_DEVICES: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebLimits {
    pub max_devices: Option<usize>,
    pub max_discovery_cidrs: Option<usize>,
}

impl WebLimits {
    pub const fn community() -> Self {
        Self {
            max_devices: Some(COMMUNITY_MAX_DEVICES),
            max_discovery_cidrs: Some(1),
        }
    }

    pub const fn unrestricted() -> Self {
        Self {
            max_devices: None,
            max_discovery_cidrs: None,
        }
    }
}

// ─── Scan job state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum ScanState {
    Running,
    Done,
    Error(String),
}

#[derive(Debug, Clone)]
struct ScanJob {
    state: ScanState,
    started_at: Instant,
    total: usize,
    scanned: usize,
    found: Vec<DeviceConfig>,
}

impl ScanJob {
    fn new(total: usize) -> Self {
        Self {
            state: ScanState::Running,
            started_at: Instant::now(),
            total,
            scanned: 0,
            found: Vec::new(),
        }
    }
}

type JobStore = Arc<Mutex<HashMap<String, ScanJob>>>;
type NotificationProvider = Option<Arc<dyn NotificationSettingsProvider>>;

pub struct WebExtensionResponse {
    pub status: String,
    pub content_type: String,
    pub body: String,
    pub no_cache: bool,
}

pub trait WebExtensionProvider: Send + Sync {
    fn handle_request(
        &self,
        method: &str,
        path: &str,
        query: &str,
        body: &str,
    ) -> Option<WebExtensionResponse>;

    fn dashboard_html(&self) -> Option<String> {
        None
    }

    fn dashboard_styles(&self) -> String {
        String::new()
    }

    fn dashboard_scripts(&self) -> String {
        String::new()
    }

    fn navigation_html(&self) -> String {
        String::new()
    }

    fn settings_html(&self) -> String {
        String::new()
    }

    fn settings_scripts(&self) -> String {
        String::new()
    }

    fn observe_interface_sample(
        &self,
        _device: &DeviceConfig,
        _sample: &crate::db::models::InterfaceSample,
        _history: &[crate::db::models::InterfaceSample],
        _alerts: &AlertBroadcaster,
    ) {
    }
}

pub struct WebServer {
    pub host: String,
    pub port: u16,
    config: Arc<Mutex<AppConfig>>,
    config_path: std::path::PathBuf,
    repository: Arc<Mutex<Repository>>,
    jobs: JobStore,
    limits: WebLimits,
    alerts: AlertBroadcaster,
    notifications: NotificationProvider,
    web_extension: Option<Arc<dyn WebExtensionProvider>>,
    flow_repository: Option<Arc<dyn crate::flow::FlowRepository>>,
}

impl WebServer {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        config: AppConfig,
        repository: Repository,
    ) -> Self {
        Self::with_limits_and_broadcaster(
            host,
            port,
            config,
            repository,
            WebLimits::community(),
            AlertBroadcaster::new(),
        )
    }

    pub fn with_limits(
        host: impl Into<String>,
        port: u16,
        config: AppConfig,
        repository: Repository,
        limits: WebLimits,
    ) -> Self {
        Self::with_limits_and_broadcaster(
            host,
            port,
            config,
            repository,
            limits,
            AlertBroadcaster::new(),
        )
    }

    pub fn with_limits_and_broadcaster(
        host: impl Into<String>,
        port: u16,
        config: AppConfig,
        repository: Repository,
        limits: WebLimits,
        alerts: AlertBroadcaster,
    ) -> Self {
        let config_path = crate::config_path();
        Self {
            host: host.into(),
            port,
            config: Arc::new(Mutex::new(config)),
            config_path,
            repository: Arc::new(Mutex::new(repository)),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            limits,
            alerts,
            notifications: None,
            web_extension: None,
            flow_repository: None,
        }
    }

    pub fn with_flow_repository(
        mut self,
        repository: Arc<dyn crate::flow::FlowRepository>,
    ) -> Self {
        self.flow_repository = Some(repository);
        self
    }

    pub fn with_notification_provider(
        mut self,
        provider: Arc<dyn NotificationSettingsProvider>,
    ) -> Self {
        self.notifications = Some(provider);
        self
    }

    pub fn with_web_extension(mut self, provider: Arc<dyn WebExtensionProvider>) -> Self {
        self.web_extension = Some(provider);
        self
    }

    pub fn start(&self) -> Result<(), AppError> {
        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr).map_err(|err| AppError::Io(err.to_string()))?;

        println!("Web dashboard started on http://{}/", addr);
        open_browser(&format!("http://{}/", addr));

        // バックグラウンドポーリングスレッドを起動
        {
            let repo = Arc::clone(&self.repository);
            let cfg = Arc::clone(&self.config);
            let alerts = self.alerts.clone();
            let extension = self.web_extension.clone();
            std::thread::spawn(move || {
                run_polling_loop_with_observer(cfg, repo, alerts, extension, true);
            });
        }
        let flow_config = self
            .config
            .lock()
            .map(|config| crate::flow::FlowCollectorConfig::from_app_config(&config))
            .unwrap_or_default();
        if let Some(flow_repository) = &self.flow_repository {
            crate::flow::FlowCollector::start_with_repository(
                Arc::clone(flow_repository),
                flow_config,
            );
        } else {
            crate::flow::FlowCollector::start(Arc::clone(&self.repository), flow_config);
        }

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let _ = stream.set_nodelay(true);
                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(10)));
                    let repo = Arc::clone(&self.repository);
                    let jobs = Arc::clone(&self.jobs);
                    let cfg = Arc::clone(&self.config);
                    let cfg_path = self.config_path.clone();
                    let limits = self.limits;
                    let notifications = self.notifications.clone();
                    let web_extension = self.web_extension.clone();
                    std::thread::spawn(move || {
                        let _ = handle_connection(
                            stream,
                            repo,
                            jobs,
                            cfg,
                            cfg_path,
                            limits,
                            notifications,
                            web_extension,
                        );
                    });
                }
                Err(err) => {
                    eprintln!("accept failed: {err}");
                }
            }
        }

        Ok(())
    }
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd").args(["/C", "start", "", url]).spawn();

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).spawn();

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(url).spawn();

    if let Err(err) = result {
        eprintln!("failed to open browser: {err}");
    }
}

// ─── HTTP helpers ────────────────────────────────────────────────────────────

struct Request {
    method: String,
    path: String,
    body: String,
}

fn parse_request(stream: &TcpStream) -> Result<Request, AppError> {
    let mut reader = BufReader::new(stream.try_clone()?);

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header = header.trim();
        if header.is_empty() {
            break;
        }
        if let Some(val) = header.to_lowercase().strip_prefix("content-length:") {
            content_length = val.trim().parse().unwrap_or(0);
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    Ok(Request {
        method,
        path,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn respond(
    mut stream: TcpStream,
    status: &str,
    content_type: &str,
    body: String,
) -> Result<(), AppError> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn respond_app_js(mut stream: TcpStream, body: &str) -> Result<(), AppError> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/javascript; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-cache, must-revalidate\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn respond_json(stream: TcpStream, status: &str, body: String) -> Result<(), AppError> {
    respond(stream, status, "application/json; charset=utf-8", body)
}

fn respond_html(stream: TcpStream, body: String) -> Result<(), AppError> {
    respond(stream, "200 OK", "text/html; charset=utf-8", body)
}

fn respond_js(stream: TcpStream, body: &str) -> Result<(), AppError> {
    respond_app_js(stream, body)
}

fn respond_extension(
    mut stream: TcpStream,
    response: WebExtensionResponse,
) -> Result<(), AppError> {
    let cache_control = if response.no_cache {
        "Cache-Control: no-cache, must-revalidate"
    } else {
        "Cache-Control: public, max-age=86400"
    };
    let wire = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        response.status,
        response.content_type,
        response.body.len(),
        cache_control,
        response.body
    );
    stream.write_all(wire.as_bytes())?;
    stream.flush()?;
    Ok(())
}

// ─── Router ──────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn handle_connection(
    stream: TcpStream,
    repo: Arc<Mutex<Repository>>,
    jobs: JobStore,
    cfg: Arc<Mutex<AppConfig>>,
    cfg_path: std::path::PathBuf,
    limits: WebLimits,
    notifications: NotificationProvider,
    web_extension: Option<Arc<dyn WebExtensionProvider>>,
) -> Result<(), AppError> {
    let req = parse_request(&stream)?;
    let clean_path = req.path.split('?').next().unwrap_or(&req.path);
    let query = req
        .path
        .split_once('?')
        .map(|(_, value)| value)
        .unwrap_or("");

    if let Some(extension) = web_extension.as_deref()
        && let Some(response) = extension.handle_request(&req.method, clean_path, query, &req.body)
    {
        return respond_extension(stream, response);
    }

    if req.method == "GET" && clean_path == "/static/js/device-detail.js" {
        return respond_js(stream, DEVICE_DETAIL_BUNDLE_JS);
    }
    if req.method == "GET" && clean_path == "/static/js/dashboard.js" {
        return respond_js(stream, DASHBOARD_BUNDLE_JS);
    }
    if req.method == "GET" && clean_path == "/static/js/settings.js" {
        return respond_js(stream, SETTINGS_BUNDLE_JS);
    }
    if req.method == "GET" && clean_path == "/static/js/notifications.js" {
        return respond_js(stream, NOTIFICATIONS_BUNDLE_JS);
    }
    if req.method == "GET" && clean_path == "/static/js/diagnostics.js" {
        return respond_js(stream, DIAGNOSTICS_BUNDLE_JS);
    }
    if req.method == "GET" && clean_path == "/static/js/discovery.js" {
        return respond_js(stream, DISCOVERY_BUNDLE_JS);
    }
    if req.method == "GET" && clean_path == "/static/js/topology.js" {
        return respond_js(stream, TOPOLOGY_BUNDLE_JS);
    }
    if req.method == "GET" && clean_path == "/static/js/theme.js" {
        return respond_js(stream, THEME_BUNDLE_JS);
    }
    if req.method == "GET" && clean_path == "/static/js/i18n.js" {
        return respond_js(stream, I18N_BUNDLE_JS);
    }

    // Route: GET /api/discovery/scan/<job_id>
    if req.method == "GET" && req.path.starts_with("/api/discovery/scan/") {
        let job_id = req
            .path
            .trim_start_matches("/api/discovery/scan/")
            .to_string();
        let body = api_scan_status(&job_id, &jobs);
        return respond_json(stream, "200 OK", body);
    }

    // Route: GET /api/device/<ip>  (IP に . を含むためパスマッチで処理)
    if req.method == "GET" && req.path.starts_with("/api/device/") {
        let ip = req.path.trim_start_matches("/api/device/").to_string();
        let body = api_device_detail(&ip, &repo, &cfg);
        return respond_json(stream, "200 OK", body);
    }

    if req.method == "GET" && req.path.starts_with("/api/flow/analytics") {
        let (window_seconds, limit) = flow_analytics_options(&req.path);
        let body = api_flow_analytics(&repo, window_seconds, limit);
        return respond_json(stream, "200 OK", body);
    }
    if req.method == "GET" && req.path == "/api/system/metrics" {
        return respond_json(stream, "200 OK", api_system_metrics());
    }

    if req.method == "DELETE" && req.path.starts_with("/api/device/") {
        let ip = req.path.trim_start_matches("/api/device/").to_string();
        let (status, body) = api_delete_device(&ip, &repo);
        return respond_json(stream, &status, body);
    }

    // Route: GET /device/<ip>
    if req.method == "GET" && req.path.starts_with("/device/") {
        let ip = req.path.trim_start_matches("/device/").to_string();
        return respond_html(stream, page_device_detail(&ip, &repo));
    }

    if req.method == "GET" && req.path.starts_with("/diagnostics") {
        return respond_html(
            stream,
            page_diagnostics(&req.path, &cfg, web_extension.as_deref()),
        );
    }

    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") | ("GET", "/dashboard") => respond_html(
            stream,
            page_dashboard(&repo, &cfg, web_extension.as_deref()),
        ),
        ("GET", "/discovery") => respond_html(
            stream,
            page_discovery(&repo, limits, web_extension.as_deref()),
        ),
        ("GET", "/diagnostics") => respond_html(
            stream,
            page_diagnostics("/diagnostics", &cfg, web_extension.as_deref()),
        ),
        ("GET", "/settings") => respond_html(
            stream,
            page_settings(&cfg, notifications.is_some(), web_extension.as_deref()),
        ),
        ("GET", "/api/summary") => respond_json(stream, "200 OK", api_summary(&repo)),
        ("GET", "/api/devices") => respond_json(stream, "200 OK", api_devices(&repo, &cfg)),
        ("GET", "/api/settings") => respond_json(stream, "200 OK", api_get_settings(&cfg)),
        ("GET", "/api/notifications") => respond_json(
            stream,
            "200 OK",
            api_get_notifications(notifications.as_deref()),
        ),
        ("POST", "/api/notifications") => respond_json(
            stream,
            "200 OK",
            api_post_notifications(&req.body, notifications.as_deref()),
        ),
        ("POST", "/api/notifications/test") => respond_json(
            stream,
            "200 OK",
            api_test_notification(&req.body, notifications.as_deref()),
        ),
        ("POST", "/api/settings") => respond_json(
            stream,
            "200 OK",
            api_post_settings(&req.body, &cfg, &cfg_path),
        ),
        ("POST", "/api/diagnostics/oids") => respond_json(
            stream,
            "200 OK",
            api_post_oid_overrides(&req.body, &cfg, &cfg_path),
        ),
        ("POST", "/api/discovery/scan") => {
            respond_json(stream, "200 OK", api_scan_start(&req.body, jobs, limits))
        }
        ("POST", "/api/discovery/topology") => respond_json(
            stream,
            "200 OK",
            api_discovery_topology(&req.body, &cfg, &repo, limits),
        ),
        ("POST", "/api/discovery/register") => {
            respond_json(stream, "200 OK", api_register(&req.body, &repo, limits))
        }
        _ => respond_json(
            stream,
            "404 Not Found",
            r#"{"error":"not found"}"#.to_string(),
        ),
    }
}

// ─── API handlers ─────────────────────────────────────────────────────────────

fn api_summary(repo: &Arc<Mutex<Repository>>) -> String {
    let Ok(repo) = repo.lock() else {
        return r#"{"error":"repository lock failed"}"#.to_string();
    };
    let devices = repo.list_devices().unwrap_or_default();
    let healthy = devices.iter().filter(|d| d.status == "online").count();
    let offline = devices.iter().filter(|d| d.status == "offline").count();
    let warning = devices.iter().filter(|d| d.status == "warning").count();
    let critical = devices.iter().filter(|d| d.status == "critical").count();
    format!(
        r#"{{"healthy":{healthy},"warning":{warning},"critical":{critical},"offline":{offline},"total":{}}}"#,
        devices.len()
    )
}

fn api_system_metrics() -> String {
    let (received, parsed, dropped, latency) = crate::flow::metrics();
    let latency = latency.map_or_else(|| "null".to_string(), |value| format!("{value:.3}"));
    let process_memory = crate::flow::process_memory_bytes()
        .map_or_else(|| "null".to_string(), |value| value.to_string());
    let live_memory = crate::flow::live_memory_bytes();
    format!(
        r#"{{"received_flows_total":{received},"parsed_flows_total":{parsed},"dropped_flows_total":{dropped},"db_write_latency_ms":{latency},"process_memory_bytes":{process_memory},"live_buffer_bytes":{live_memory},"live_buffer_limit_bytes":{}}}"#,
        512_u64 * 1024 * 1024
    )
}

fn flow_analytics_options(path: &str) -> (i64, usize) {
    let query = path.split_once('?').map(|(_, value)| value).unwrap_or("");
    let mut window_seconds = 60_i64;
    let mut limit = 10_usize;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key {
            "window" => {
                window_seconds = match value {
                    "5m" => 300,
                    "1h" => 3600,
                    "24h" => 86400,
                    _ => value
                        .strip_suffix('s')
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(60),
                };
            }
            "limit" => limit = value.parse::<usize>().unwrap_or(10).clamp(1, 100),
            _ => {}
        }
    }
    (window_seconds.clamp(1, 86400), limit)
}

fn api_flow_analytics(repo: &Arc<Mutex<Repository>>, window_seconds: i64, limit: usize) -> String {
    if window_seconds <= 60 {
        let records = crate::flow::live_records(window_seconds);
        if !records.is_empty() {
            return api_live_flow_analytics(&records, window_seconds, limit);
        }
    }
    let Ok(repo) = repo.lock() else {
        return r#"{"error":"repository lock failed"}"#.to_string();
    };
    let shares = repo.protocol_shares(window_seconds).unwrap_or_default();
    let talkers = repo.top_talkers(window_seconds, limit).unwrap_or_default();
    let summary =
        repo.flow_summary(window_seconds)
            .unwrap_or_else(|_| crate::db::models::FlowSummary {
                total_bps: 0.0,
                total_pps: 0.0,
                active_flows: 0,
                top_protocol: "-".to_string(),
            });
    let applications = repo
        .flow_applications(window_seconds, limit)
        .unwrap_or_default();
    let sources = repo
        .flow_endpoints(window_seconds, true, limit)
        .unwrap_or_default();
    let destinations = repo
        .flow_endpoints(window_seconds, false, limit)
        .unwrap_or_default();
    let timeseries = repo.flow_timeseries(window_seconds).unwrap_or_default();
    let shares_json = shares
        .iter()
        .map(|share| {
            format!(
                r#"{{"protocol":"{}","bytes":{},"percentage":{:.2},"bps":{:.0},"pps":{:.2}}}"#,
                escape_json(&share.protocol),
                share.bytes,
                share.percentage,
                share.bps,
                share.pps,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let talkers_json = talkers
    .iter()
    .map(|talker| {
      format!(
        r#"{{"source_ip":"{}","destination_ip":"{}","source_port":{},"destination_port":{},"protocol":"{}","app_name":"{}","bytes":{},"packets":{},"bps":{},"pps":{},"tcp_flags":[{}],"ingress_if_index":{},"ingress_if_name":{},"egress_if_index":{},"egress_if_name":{}}}"#,
        escape_json(&talker.source_ip),
        escape_json(&talker.destination_ip),
        talker.source_port,
        talker.destination_port,
        escape_json(&talker.protocol),
        escape_json(&talker.app_name),
        talker.bytes,
        talker.packets,
        talker.bps,
        talker.pps,
        tcp_flags_json(talker.tcp_flags),
        talker.ingress_if_index.map_or_else(|| "null".to_string(), |value| value.to_string()),
        escape_json(&flow_interface_name(talker.ingress_if_index, talker.ingress_if_name.as_deref())),
        talker.egress_if_index.map_or_else(|| "null".to_string(), |value| value.to_string()),
        escape_json(&flow_interface_name(talker.egress_if_index, talker.egress_if_name.as_deref())),
      )
    })
    .collect::<Vec<_>>()
    .join(",");
    let applications_json = applications
        .iter()
        .map(|item| {
            format!(
                r#"{{"app_name":"{}","bytes":{},"percentage":{:.2},"bps":{:.0}}}"#,
                escape_json(&item.app_name),
                item.bytes,
                item.percentage,
                item.bps
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let endpoints_json = |items: &Vec<crate::db::models::FlowEndpointShare>| {
        items
            .iter()
            .map(|item| {
                format!(
                    r#"{{"ip":"{}","bps":{:.0},"percentage":{:.2}}}"#,
                    escape_json(&item.ip),
                    item.bps,
                    item.percentage
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let timeseries_json = timeseries
        .iter()
        .map(|item| {
            format!(
                r#"{{"timestamp":"{}","udp_bps":{:.0},"tcp_bps":{:.0},"icmp_bps":{:.0}}}"#,
                escape_json(&item.timestamp),
                item.udp_bps,
                item.tcp_bps,
                item.icmp_bps
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"window_seconds":{},"summary":{{"total_bps":{:.0},"total_pps":{:.2},"active_flows":{},"top_protocol":"{}"}},"timeseries":[{}],"protocols":[{}],"applications":[{}],"top_sources":[{}],"top_destinations":[{}],"top_talkers":[{}]}}"#,
        window_seconds,
        summary.total_bps,
        summary.total_pps,
        summary.active_flows,
        escape_json(&summary.top_protocol),
        timeseries_json,
        shares_json,
        applications_json,
        endpoints_json(&sources),
        endpoints_json(&destinations),
        talkers_json
    )
}

type ConversationKey = (String, String, u16, u16, String);
type ConversationValue = (u64, u64, u8);

fn api_live_flow_analytics(
    records: &[crate::db::models::FlowRecord],
    window_seconds: i64,
    limit: usize,
) -> String {
    let mut protocols: HashMap<String, (u64, u64)> = HashMap::new();
    let mut conversations: HashMap<ConversationKey, ConversationValue> = HashMap::new();
    let mut applications: HashMap<(String, u16), u64> = HashMap::new();
    let mut sources: HashMap<String, u64> = HashMap::new();
    let mut destinations: HashMap<String, u64> = HashMap::new();
    let mut timeseries: HashMap<String, (u64, u64, u64)> = HashMap::new();
    for record in records {
        let protocol = protocols.entry(record.protocol.clone()).or_default();
        protocol.0 = protocol.0.saturating_add(record.bytes);
        protocol.1 = protocol.1.saturating_add(record.packets);
        let key = (
            record.source_ip.clone(),
            record.destination_ip.clone(),
            record.source_port,
            record.destination_port,
            record.protocol.clone(),
        );
        let conversation = conversations.entry(key).or_default();
        conversation.0 = conversation.0.saturating_add(record.bytes);
        conversation.1 = conversation.1.saturating_add(record.packets);
        conversation.2 |= record.tcp_flags;
        *applications
            .entry((record.protocol.clone(), record.destination_port))
            .or_default() += record.bytes;
        *sources.entry(record.source_ip.clone()).or_default() += record.bytes;
        *destinations
            .entry(record.destination_ip.clone())
            .or_default() += record.bytes;
        let bucket = record
            .observed_at
            .get(..16)
            .unwrap_or(&record.observed_at)
            .to_string();
        let traffic = timeseries.entry(bucket).or_default();
        match record.protocol.as_str() {
            "UDP" => traffic.0 += record.bytes,
            "TCP" => traffic.1 += record.bytes,
            "ICMP" => traffic.2 += record.bytes,
            _ => {}
        }
    }
    let total_bytes: u64 = protocols.values().map(|value| value.0).sum();
    let total_packets: u64 = protocols.values().map(|value| value.1).sum();
    let protocol_json = protocols
        .iter()
        .map(|(protocol, (bytes, packets))| {
            format!(
                r#"{{"protocol":"{}","bytes":{},"percentage":{:.2},"bps":{:.0},"pps":{:.2}}}"#,
                escape_json(protocol),
                bytes,
                *bytes as f64 * 100.0 / total_bytes.max(1) as f64,
                *bytes as f64 * 8.0 / window_seconds.max(1) as f64,
                *packets as f64 / window_seconds.max(1) as f64
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut conversation_rows: Vec<_> = conversations
        .into_iter()
        .map(
            |(
                (source, destination, source_port, destination_port, protocol),
                (bytes, packets, flags),
            )| {
                (
                    bytes,
                    source,
                    destination,
                    source_port,
                    destination_port,
                    protocol,
                    packets,
                    flags,
                )
            },
        )
        .collect();
    conversation_rows.sort_by_key(|left| std::cmp::Reverse(left.0));
    let talker_json = conversation_rows.into_iter().take(limit).map(|(bytes, source, destination, source_port, destination_port, protocol, packets, flags)| format!(r#"{{"source_ip":"{}","destination_ip":"{}","source_port":{},"destination_port":{},"protocol":"{}","bytes":{},"packets":{},"bps":{:.0},"pps":{:.2},"tcp_flags":[{}]}}"#, escape_json(&source), escape_json(&destination), source_port, destination_port, escape_json(&protocol), bytes, packets, bytes as f64 * 8.0 / window_seconds.max(1) as f64, packets as f64 / window_seconds.max(1) as f64, tcp_flags_json(flags))).collect::<Vec<_>>().join(",");
    let applications_json = applications
        .into_iter()
        .map(|((protocol, port), bytes)| {
            format!(
                r#"{{"app_name":"{}","bytes":{},"percentage":{:.2},"bps":{:.0}}}"#,
                escape_json(&application_name_for_live(&protocol, port)),
                bytes,
                bytes as f64 * 100.0 / total_bytes.max(1) as f64,
                bytes as f64 * 8.0 / window_seconds.max(1) as f64
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let endpoint_json = |items: HashMap<String, u64>| {
        items
            .into_iter()
            .take(limit)
            .map(|(ip, bytes)| {
                format!(
                    r#"{{"ip":"{}","bps":{:.0},"percentage":{:.2}}}"#,
                    escape_json(&ip),
                    bytes as f64 * 8.0 / window_seconds.max(1) as f64,
                    bytes as f64 * 100.0 / total_bytes.max(1) as f64
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let timeseries_json = timeseries
        .into_iter()
        .map(|(timestamp, (udp, tcp, icmp))| {
            format!(
                r#"{{"timestamp":"{}","udp_bps":{:.0},"tcp_bps":{:.0},"icmp_bps":{:.0}}}"#,
                escape_json(&timestamp),
                udp as f64 * 8.0 / 60.0,
                tcp as f64 * 8.0 / 60.0,
                icmp as f64 * 8.0 / 60.0
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"window_seconds":{},"summary":{{"total_bps":{:.0},"total_pps":{:.2},"active_flows":{},"top_protocol":"-"}},"timeseries":[{}],"protocols":[{}],"applications":[{}],"top_sources":[{}],"top_destinations":[{}],"top_talkers":[{}]}}"#,
        window_seconds,
        total_bytes as f64 * 8.0 / window_seconds.max(1) as f64,
        total_packets as f64 / window_seconds.max(1) as f64,
        records.len(),
        timeseries_json,
        protocol_json,
        applications_json,
        endpoint_json(sources),
        endpoint_json(destinations),
        talker_json
    )
}

fn application_name_for_live(protocol: &str, port: u16) -> String {
    match port {
        22 => format!("SSH ({protocol}/{port})"),
        53 => format!("DNS ({protocol}/{port})"),
        80 => format!("HTTP ({protocol}/{port})"),
        443 => format!("HTTPS ({protocol}/{port})"),
        _ => format!("{protocol}/{port}"),
    }
}

fn tcp_flags_json(flags: u8) -> String {
    let mut names = Vec::new();
    if flags & 0x02 != 0 {
        names.push("\"SYN\"");
    }
    if flags & 0x10 != 0 {
        names.push("\"ACK\"");
    }
    if flags & 0x04 != 0 {
        names.push("\"RST\"");
    }
    if flags & 0x01 != 0 {
        names.push("\"FIN\"");
    }
    if flags & 0x08 != 0 {
        names.push("\"PSH\"");
    }
    if flags & 0x20 != 0 {
        names.push("\"URG\"");
    }
    names.join(",")
}

fn flow_interface_name(index: Option<u32>, name: Option<&str>) -> String {
    match index {
        Some(0) => "Internal/Local".to_string(),
        Some(value) => name
            .map(str::to_string)
            .unwrap_or_else(|| format!("if-{value}")),
        None => "-".to_string(),
    }
}

fn api_devices(repo: &Arc<Mutex<Repository>>, cfg: &Arc<Mutex<AppConfig>>) -> String {
    let spike_threshold = cfg.lock().map(|c| c.alert.spike_threshold).unwrap_or(10);
    let Ok(repo) = repo.lock() else {
        return r#"{"error":"repository lock failed"}"#.to_string();
    };
    let devices = repo.list_devices().unwrap_or_default();
    let items: Vec<String> = devices.iter().map(|d| {
        let spike = d.id
            .and_then(|id| repo.check_interface_spikes(id, spike_threshold).ok())
            .map(|spikes| !spikes.is_empty())
            .unwrap_or(false);
        format!(
            r#"{{"ip":"{}","name":"{}","status":"{}","community":"{}","last_seen":"{}","error_spike":{}}}"#,
            escape_json(&d.ip),
            escape_json(&d.name),
            escape_json(&d.status),
            escape_json(&d.community),
            escape_json(d.last_seen_at.as_deref().unwrap_or("")),
            spike,
        )
    }).collect();
    format!("[{}]", items.join(","))
}

fn api_delete_device(ip: &str, repo: &Arc<Mutex<Repository>>) -> (String, String) {
    let Ok(repo) = repo.lock() else {
        return (
            "500 Internal Server Error".to_string(),
            r#"{"error":"repository lock failed"}"#.to_string(),
        );
    };

    match repo.remove_device_by_ip(ip) {
        Ok(()) => ("200 OK".to_string(), r#"{"ok":true}"#.to_string()),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("was not found") {
                (
                    "404 Not Found".to_string(),
                    format!(r#"{{"error":"{}"}}"#, escape_json(&msg)),
                )
            } else {
                (
                    "400 Bad Request".to_string(),
                    format!(r#"{{"error":"{}"}}"#, escape_json(&msg)),
                )
            }
        }
    }
}

fn api_device_detail(
    ip: &str,
    repo: &Arc<Mutex<Repository>>,
    _cfg: &Arc<Mutex<AppConfig>>,
) -> String {
    let Ok(r) = repo.lock() else {
        return r#"{"error":"lock failed"}"#.to_string();
    };
    // デバイス基本情報
    let device = match r.find_device_by_ip(ip) {
        Ok(Some(d)) => d,
        Ok(None) => return r#"{"error":"device not found"}"#.to_string(),
        Err(e) => return format!(r#"{{"error":"{}"}}"#, escape_json(&e.to_string())),
    };
    let device_id = match device.id {
        Some(id) => id,
        None => return r#"{"error":"device has no id"}"#.to_string(),
    };

    let interface_spikes = r
        .get_recent_interface_spikes(device_id, 30)
        .unwrap_or_default();
    let hardware_sensors = SnmpClient::new(device.community.clone())
        .query_hardware_sensors(&DeviceConfig {
            id: device.id,
            name: device.name.clone(),
            ip: device.ip.clone(),
            community: device.community.clone(),
            device_type: device.device_type.clone(),
            status: device.status.clone(),
            last_seen_at: device.last_seen_at.clone(),
        })
        .unwrap_or_default();

    // 最新インターフェーススナップショット
    let latest_ifaces = r.get_latest_interfaces(device_id).unwrap_or_default();
    let ifaces_json: Vec<String> = latest_ifaces.iter().map(|s| {
        let recent = r.get_recent_interface_samples(device_id, s.if_index, 3).unwrap_or_default();
        let prev = recent.get(1);
        let in_errors_delta = prev.map(|p| counter32_delta(p.in_errors, s.in_errors)).unwrap_or(0);
        let out_errors_delta = prev.map(|p| counter32_delta(p.out_errors, s.out_errors)).unwrap_or(0);
        let in_discards_delta = prev.map(|p| counter32_delta(p.in_discards, s.in_discards)).unwrap_or(0);
        let out_discards_delta = prev.map(|p| counter32_delta(p.out_discards, s.out_discards)).unwrap_or(0);
        let late_collisions_delta = prev.map(|p| counter32_delta(p.late_collisions, s.late_collisions)).unwrap_or(0);
        // EtherLike-MIB (RFC 3635) 破損パケット内訳の差分
        let fcs_errors_delta = prev.map(|p| counter32_delta(p.fcs_errors, s.fcs_errors)).unwrap_or(0);
        let alignment_errors_delta = prev.map(|p| counter32_delta(p.alignment_errors, s.alignment_errors)).unwrap_or(0);
        let frame_too_longs_delta = prev.map(|p| counter32_delta(p.frame_too_longs, s.frame_too_longs)).unwrap_or(0);
        let internal_mac_receive_errors_delta = prev.map(|p| counter32_delta(p.internal_mac_receive_errors, s.internal_mac_receive_errors)).unwrap_or(0);
        let link_status = effective_interface_link_status(&device.status, &s.link_status);
        let health_status = classify_interface_diagnostic(
            &link_status,
            in_errors_delta,
            out_errors_delta,
            in_discards_delta,
            out_discards_delta,
            late_collisions_delta,
        );
        format!(
            r#"{{"if_index":{},"if_name":"{}","link_status":"{}","health_status":"{}","metrics":{{"in_errors":{},"in_errors_delta":{},"out_errors":{},"out_errors_delta":{},"in_discards":{},"in_discards_delta":{},"out_discards":{},"out_discards_delta":{},"late_collisions":{},"late_collisions_delta":{},"bandwidth_utilization":{:.2}}},"error_breakdown":{{"fcs_errors":{},"fcs_errors_delta":{},"alignment_errors":{},"alignment_errors_delta":{},"frame_too_longs":{},"frame_too_longs_delta":{},"internal_mac_receive_errors":{},"internal_mac_receive_errors_delta":{}}},"sampled_at":"{}"}}"#,
            s.if_index, escape_json(&s.if_name), escape_json(&link_status), health_status,
            s.in_errors, in_errors_delta,
            s.out_errors, out_errors_delta,
            s.in_discards, in_discards_delta,
            s.out_discards, out_discards_delta,
            s.late_collisions, late_collisions_delta,
            s.bandwidth_utilization,
            s.fcs_errors, fcs_errors_delta,
            s.alignment_errors, alignment_errors_delta,
            s.frame_too_longs, frame_too_longs_delta,
            s.internal_mac_receive_errors, internal_mac_receive_errors_delta,
            escape_json(&s.sampled_at),
        )
    }).collect();

    // インターフェース時系列（直近 48 件 = 約24時間 / 30秒ポーリング → 実際は1440件/24hだが表示は間引き）
    let if_history = r.get_interface_history(device_id, 288).unwrap_or_default();
    // if_index ごとに分けてグラフデータにする
    let mut if_series: std::collections::HashMap<i32, Vec<String>> =
        std::collections::HashMap::new();
    for s in &if_history {
        let entry = if_series.entry(s.if_index).or_default();
        entry.push(format!(
            r#"{{"t":"{}","in_err":{},"out_err":{},"in_dis":{},"out_dis":{},"bw":{:.2}}}"#,
            escape_json(&s.sampled_at),
            s.in_errors,
            s.out_errors,
            s.in_discards,
            s.out_discards,
            s.bandwidth_utilization
        ));
    }
    let mut if_series_json: Vec<(i32, String)> = if_series
        .iter()
        .map(|(idx, pts)| {
            (
                *idx,
                format!(r#"{{"if_index":{},"points":[{}]}}"#, idx, pts.join(",")),
            )
        })
        .collect();
    if_series_json.sort_by_key(|(idx, _)| *idx);

    // CPU履歴（直近 288 件 = 約2.4時間 @ 30s）
    let metrics = r
        .get_device_metrics_history(device_id, 288)
        .unwrap_or_default();
    let metrics_json: Vec<String> = metrics
        .iter()
        .map(|m| {
            let cpu = m
                .cpu_usage
                .filter(|v| *v <= 100)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string());
            let memory = m
                .memory_usage
                .filter(|v| *v <= 100)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string());
            let memory_bytes = m
                .memory_used_bytes
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string());
            format!(
                r#"{{"t":"{}","cpu":{},"memory":{},"memory_bytes":{}}}"#,
                escape_json(&m.sampled_at),
                cpu,
                memory,
                memory_bytes
            )
        })
        .collect();

    let latest_m = metrics.last();
    let cpu_util = latest_m
        .and_then(|m| m.cpu_usage)
        .filter(|v| *v <= 100)
        .map(|v| v as f64);
    let mem_util = latest_m
        .and_then(|m| m.memory_usage)
        .filter(|v| *v <= 100)
        .map(|v| v as f64);
    let mem_status = match mem_util {
        Some(u) if u >= 90.0 => "critical",
        Some(u) if u >= 80.0 => "warning",
        Some(_) => "normal",
        None => "unknown",
    };
    let system_metrics_json = format!(
        r#"{{"cpu_utilization":{},"memory_utilization":{},"memory_status":"{}"}}"#,
        cpu_util
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string()),
        mem_util
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string()),
        mem_status
    );

    // アラート履歴（直近 30 件）
    let alerts = r.get_alert_history(device_id, 30).unwrap_or_default();
    let alerts_json: Vec<String> = alerts
        .iter()
        .map(|a| {
            let interface = extract_interface_from_alert(&a.alert_type, &a.details);
            format!(
                r#"{{"type":"{}","severity":"{}","details":"{}","interface":"{}","at":"{}"}}"#,
                escape_json(&a.alert_type),
                escape_json(&a.severity),
                escape_json(&a.details),
                escape_json(&interface),
                escape_json(a.created_at.as_deref().unwrap_or("")),
            )
        })
        .collect();

    let spikes_json: Vec<String> = interface_spikes.iter().map(|s| {
        let link_status = effective_interface_link_status(&device.status, &s.link_status);
        format!(
            r#"{{"if_index":{},"if_name":"{}","link_status":"{}","in_errors_delta":{},"out_errors_delta":{},"in_discards_delta":{},"out_discards_delta":{},"total_delta":{},"latest_sampled_at":"{}","previous_sampled_at":"{}"}}"#,
            s.if_index,
            escape_json(&s.if_name),
            escape_json(&link_status),
            s.in_errors_delta,
            s.out_errors_delta,
            s.in_discards_delta,
            s.out_discards_delta,
            s.total_delta,
            escape_json(&s.latest_sampled_at),
            escape_json(&s.previous_sampled_at),
        )
    }).collect();

    let sensors_json: Vec<String> = hardware_sensors.iter().map(|s| {
        let value = s.value.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        let unit = s.unit.as_deref().unwrap_or("");
        let status = s.status.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        format!(
            r#"{{"index":{},"name":"{}","sensor_type":"{}","source":"{}","oid":"{}","value":{},"unit":"{}","status":{},"status_text":"{}","is_alarm":{}}}"#,
            s.index,
            escape_json(&s.name),
            escape_json(&s.sensor_type),
            escape_json(&s.source),
            escape_json(&s.oid),
            value,
            escape_json(unit),
            status,
            escape_json(s.status_text.as_deref().unwrap_or("")),
            s.is_alarm,
        )
    }).collect();

    format!(
        r#"{{"device_id":"{}","ip":"{}","name":"{}","status":"{}","community":"{}","last_seen":"{}","system_metrics":{},"interfaces":[{}],"if_series":[{}],"metrics":[{}],"alerts":[{}],"spikes":[{}],"hardware_sensors":[{}]}}"#,
        escape_json(&device.ip),
        escape_json(&device.ip),
        escape_json(&device.name),
        escape_json(&device.status),
        escape_json(&device.community),
        escape_json(device.last_seen_at.as_deref().unwrap_or("")),
        system_metrics_json,
        ifaces_json.join(","),
        if_series_json
            .into_iter()
            .map(|(_, s)| s)
            .collect::<Vec<_>>()
            .join(","),
        metrics_json.join(","),
        alerts_json.join(","),
        spikes_json.join(","),
        sensors_json.join(","),
    )
}

fn api_get_settings(cfg: &Arc<Mutex<AppConfig>>) -> String {
    let Ok(c) = cfg.lock() else {
        return r#"{"error":"lock failed"}"#.to_string();
    };
    format!(
        r#"{{"polling":{{"interval_seconds":{}}},"snmp":{{"default_community":"{}"}},"display":{{"timezone":"{}"}},"alert":{{"error_rate_threshold":{},"spike_threshold":{},"health_warning_threshold":{},"health_critical_threshold":{}}},"retention":{{"history_days":{}}}}}"#,
        c.polling.interval_seconds,
        escape_json(&c.snmp.default_community),
        escape_json(&c.display.timezone),
        c.alert.error_rate_threshold,
        c.alert.spike_threshold,
        c.alert.health_warning_threshold,
        c.alert.health_critical_threshold,
        c.retention.history_days,
    )
}

fn api_post_settings(
    body: &str,
    cfg: &Arc<Mutex<AppConfig>>,
    cfg_path: &std::path::Path,
) -> String {
    // JSON フィールドを手動パース（依存追加なし）
    fn parse_u64(body: &str, key: &str) -> Option<u64> {
        let pat = format!("\"{}\":", key);
        let start = body.find(&pat)? + pat.len();
        let s = body[start..].trim_start();
        let end = s.find(|c: char| !c.is_ascii_digit())?;
        s[..end].parse().ok()
    }
    fn parse_u32(body: &str, key: &str) -> Option<u32> {
        parse_u64(body, key).map(|v| v as u32)
    }
    fn parse_f64(body: &str, key: &str) -> Option<f64> {
        let pat = format!("\"{}\":", key);
        let start = body.find(&pat)? + pat.len();
        let s = body[start..].trim_start();
        let end = s.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')?;
        s[..end].parse().ok()
    }
    fn parse_str_field(body: &str, key: &str) -> Option<String> {
        let pat = format!("\"{}\":\"", key);
        let start = body.find(&pat)? + pat.len();
        let end = body[start..].find('"')?;
        Some(body[start..start + end].to_string())
    }

    let Ok(mut c) = cfg.lock() else {
        return r#"{"error":"lock failed"}"#.to_string();
    };

    // バリデーション付き更新
    if let Some(v) = parse_u64(body, "interval_seconds") {
        if !(5..=600).contains(&v) {
            return r#"{"error":"interval_seconds must be 5-600"}"#.to_string();
        }
        c.polling.interval_seconds = v;
    }
    if let Some(v) = parse_str_field(body, "default_community")
        && !v.is_empty()
    {
        c.snmp.default_community = v;
    }
    if let Some(v) = parse_str_field(body, "timezone") {
        let tz = v.to_lowercase();
        if tz != "utc" && tz != "jst" {
            return r#"{"error":"timezone must be utc or jst"}"#.to_string();
        }
        c.display.timezone = tz;
    }
    if let Some(v) = parse_f64(body, "error_rate_threshold") {
        if v < 0.0 || v > 1.0 {
            return r#"{"error":"error_rate_threshold must be 0.0-1.0"}"#.to_string();
        }
        c.alert.error_rate_threshold = v;
    }
    if let Some(v) = parse_u64(body, "spike_threshold") {
        if v < 1 {
            return r#"{"error":"spike_threshold must be >= 1"}"#.to_string();
        }
        c.alert.spike_threshold = v;
    }
    if let Some(v) = parse_u32(body, "health_warning_threshold") {
        if v > 100 {
            return r#"{"error":"health_warning_threshold must be 0-100"}"#.to_string();
        }
        c.alert.health_warning_threshold = v;
    }
    if let Some(v) = parse_u32(body, "health_critical_threshold") {
        if v > 100 {
            return r#"{"error":"health_critical_threshold must be 0-100"}"#.to_string();
        }
        c.alert.health_critical_threshold = v;
    }
    if let Some(v) = parse_u32(body, "history_days") {
        if v < 1 {
            return r#"{"error":"history_days must be >= 1"}"#.to_string();
        }
        c.retention.history_days = v;
    }

    // Warning >= Critical の整合性チェック
    if c.alert.health_warning_threshold <= c.alert.health_critical_threshold {
        return r#"{"error":"warning threshold must be greater than critical threshold"}"#
            .to_string();
    }

    if let Err(e) = persist_config(&c, cfg_path) {
        return format!(
            r#"{{"error":"failed to write config: {}"}}"#,
            escape_json(&e)
        );
    }

    r#"{"ok":true}"#.to_string()
}

fn api_get_notifications(provider: Option<&dyn NotificationSettingsProvider>) -> String {
    provider
        .map(NotificationSettingsProvider::load_json)
        .unwrap_or_else(|| r#"{"error":"notification settings unavailable"}"#.to_string())
}

fn api_post_notifications(
    body: &str,
    provider: Option<&dyn NotificationSettingsProvider>,
) -> String {
    let Some(provider) = provider else {
        return r#"{"error":"notification settings unavailable"}"#.to_string();
    };
    match provider.save_json(body) {
        Ok(()) => r#"{"ok":true}"#.to_string(),
        Err(error) => format!(r#"{{"error":"{}"}}"#, escape_json(&error)),
    }
}

fn api_test_notification(
    body: &str,
    provider: Option<&dyn NotificationSettingsProvider>,
) -> String {
    let Some(provider) = provider else {
        return r#"{"error":"notification settings unavailable"}"#.to_string();
    };
    let channel = extract_json_str(body, "channel").unwrap_or_else(|| "all".to_string());
    match provider.send_test(&channel) {
        Ok(message) => format!(r#"{{"ok":true,"message":"{}"}}"#, escape_json(&message)),
        Err(error) => format!(r#"{{"error":"{}"}}"#, escape_json(&error)),
    }
}

fn api_post_oid_overrides(
    body: &str,
    cfg: &Arc<Mutex<AppConfig>>,
    cfg_path: &std::path::Path,
) -> String {
    fn parse_str_field(body: &str, key: &str) -> Option<String> {
        let pat = format!("\"{}\":\"", key);
        let start = body.find(&pat)? + pat.len();
        let end = body[start..].find('"')?;
        Some(body[start..start + end].to_string())
    }

    // "hardware_oid_overrides":["1.2.3","4.5.6"] のような文字列配列を抽出する簡易パーサ。
    fn parse_str_array_field(body: &str, key: &str) -> Option<Vec<String>> {
        let pat = format!("\"{}\":[", key);
        let start = body.find(&pat)? + pat.len();
        let end = body[start..].find(']')?;
        let inner = &body[start..start + end];
        let items: Vec<String> = inner
            .split(',')
            .filter_map(|part| {
                let trimmed = part.trim();
                let unquoted = trimmed.strip_prefix('"')?.strip_suffix('"')?;
                let value = unquoted.trim();
                if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            })
            .collect();
        Some(items)
    }

    let Ok(mut c) = cfg.lock() else {
        return r#"{"error":"lock failed"}"#.to_string();
    };

    if let Some(v) = parse_str_field(body, "cpu_oid_override") {
        c.snmp.cpu_oid_override = v.trim().to_string();
    }
    if let Some(v) = parse_str_field(body, "memory_oid_override") {
        c.snmp.memory_oid_override = v.trim().to_string();
    }
    if let Some(v) = parse_str_array_field(body, "hardware_oid_overrides") {
        c.snmp.hardware_oid_overrides = v;
    }

    if let Err(e) = persist_config(&c, cfg_path) {
        return format!(
            r#"{{"error":"failed to write config: {}"}}"#,
            escape_json(&e)
        );
    }

    r#"{"ok":true}"#.to_string()
}

fn persist_config(c: &AppConfig, cfg_path: &std::path::Path) -> Result<(), String> {
    // [notifier] や [license] など、AppConfig が管理しないセクションを壊さないようマージ保存する。
    let mut root: toml::value::Table = std::fs::read_to_string(cfg_path)
        .ok()
        .and_then(|content| content.parse::<toml::Table>().ok())
        .unwrap_or_default();

    fn section<'a>(root: &'a mut toml::value::Table, name: &str) -> &'a mut toml::value::Table {
        let entry = root
            .entry(name.to_string())
            .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
        if !entry.is_table() {
            *entry = toml::Value::Table(toml::value::Table::new());
        }
        entry.as_table_mut().expect("section is a table")
    }

    let polling = section(&mut root, "polling");
    polling.insert(
        "interval_seconds".to_string(),
        toml::Value::Integer(c.polling.interval_seconds as i64),
    );

    let snmp = section(&mut root, "snmp");
    snmp.insert(
        "default_community".to_string(),
        toml::Value::String(c.snmp.default_community.clone()),
    );
    snmp.insert(
        "cpu_oid_override".to_string(),
        toml::Value::String(c.snmp.cpu_oid_override.clone()),
    );
    snmp.insert(
        "memory_oid_override".to_string(),
        toml::Value::String(c.snmp.memory_oid_override.clone()),
    );
    snmp.insert(
        "hardware_oid_overrides".to_string(),
        toml::Value::Array(
            c.snmp
                .hardware_oid_overrides
                .iter()
                .map(|v| toml::Value::String(v.clone()))
                .collect(),
        ),
    );

    let display = section(&mut root, "display");
    display.insert(
        "timezone".to_string(),
        toml::Value::String(c.display.timezone.clone()),
    );

    let alert = section(&mut root, "alert");
    alert.insert(
        "error_rate_threshold".to_string(),
        toml::Value::Float(c.alert.error_rate_threshold),
    );
    alert.insert(
        "spike_threshold".to_string(),
        toml::Value::Integer(c.alert.spike_threshold as i64),
    );
    alert.insert(
        "health_warning_threshold".to_string(),
        toml::Value::Integer(c.alert.health_warning_threshold as i64),
    );
    alert.insert(
        "health_critical_threshold".to_string(),
        toml::Value::Integer(c.alert.health_critical_threshold as i64),
    );

    let retention = section(&mut root, "retention");
    retention.insert(
        "history_days".to_string(),
        toml::Value::Integer(c.retention.history_days as i64),
    );

    let toml_content = toml::to_string_pretty(&root).map_err(|e| e.to_string())?;
    std::fs::write(cfg_path, &toml_content).map_err(|e| e.to_string())
}

fn extract_interface_from_alert(alert_type: &str, details: &str) -> String {
    if alert_type != "interface_error_spike" {
        return String::new();
    }

    let Some(rest) = details.strip_prefix("Interface ") else {
        return String::new();
    };
    let Some(end) = rest.find(" spiked") else {
        return String::new();
    };
    rest[..end].to_string()
}

/// POST /api/discovery/scan
/// Body: cidr=...&community=...&max_hosts=...
/// Returns immediately with {"job_id":"..."} then caller polls GET /api/discovery/scan/<id>
fn api_scan_start(body: &str, jobs: JobStore, limits: WebLimits) -> String {
    let params = parse_form(body);
    let cidr = params.get("cidr").cloned().unwrap_or_default();
    let community = params
        .get("community")
        .cloned()
        .unwrap_or_else(|| "public".to_string());
    // max_hosts is calculated by the client from the requested CIDRs.
    let max_hosts: usize = params
        .get("max_hosts")
        .and_then(|s| s.parse().ok())
        .unwrap_or(65534);

    if cidr.is_empty() {
        return r#"{"error":"cidr is required"}"#.to_string();
    }

    let hosts = match enumerate_discovery_hosts(&cidr, limits) {
        Ok(hosts) => hosts,
        Err(err) => return format!(r#"{{"error":"{}"}}"#, escape_json(&err.to_string())),
    };
    let host_count = hosts.len().min(max_hosts);

    let job_id = format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );

    // Register job as Running
    {
        let mut store = jobs.lock().unwrap();
        store.insert(job_id.clone(), ScanJob::new(host_count));
        // Evict old jobs (keep at most 16)
        if store.len() > 16
            && let Some(oldest) = store.keys().min_by_key(|k| store[*k].started_at).cloned()
        {
            store.remove(&oldest);
        }
    }

    // Spawn background scan thread
    let job_id_thread = job_id.clone();
    std::thread::spawn(move || {
        run_scan_job(job_id_thread, cidr, community, max_hosts, jobs, limits);
    });

    format!(r#"{{"job_id":"{job_id}","total":{host_count}}}"#)
}

fn run_scan_job(
    job_id: String,
    cidr: String,
    community: String,
    max_hosts: usize,
    jobs: JobStore,
    limits: WebLimits,
) {
    use crate::snmp::SnmpClient;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const CONCURRENCY: usize = 256;

    let hosts = match enumerate_discovery_hosts(&cidr, limits) {
        Ok(h) => h,
        Err(err) => {
            if let Ok(mut store) = jobs.lock()
                && let Some(job) = store.get_mut(&job_id)
            {
                job.state = ScanState::Error(err.to_string());
            }
            return;
        }
    };

    let hosts: Vec<_> = hosts.into_iter().take(max_hosts).collect();
    let total = hosts.len();

    // 進捗カウンターをスレッド間共有
    let scanned_counter = Arc::new(AtomicUsize::new(0));

    // ホストリストをチャンクに分割して並列スレッドで処理
    let chunk_size = (total + CONCURRENCY - 1).max(1) / CONCURRENCY.min(total.max(1));
    let chunks: Vec<Vec<_>> = hosts.chunks(chunk_size).map(|c| c.to_vec()).collect();

    let mut handles = Vec::new();

    for chunk in chunks {
        let job_id2 = job_id.clone();
        let community2 = community.clone();
        let jobs2 = Arc::clone(&jobs);
        let counter2 = Arc::clone(&scanned_counter);

        let handle = std::thread::spawn(move || {
            let client = SnmpClient::new(&community2);
            for ip in chunk {
                let mut device = DeviceConfig::new(ip.to_string(), &community2);
                if let Ok(sys_name) = client.probe_device(&device) {
                    device.name = sys_name;
                    device.status = "online".to_string();
                    if let Ok(mut store) = jobs2.lock()
                        && let Some(job) = store.get_mut(&job_id2)
                    {
                        job.found.push(device);
                    }
                }
                let done = counter2.fetch_add(1, Ordering::Relaxed) + 1;
                if let Ok(mut store) = jobs2.lock()
                    && let Some(job) = store.get_mut(&job_id2)
                {
                    job.scanned = done;
                }
            }
        });
        handles.push(handle);
    }

    // 全スレッド完了を待つ
    for h in handles {
        let _ = h.join();
    }

    // Mark done
    if let Ok(mut store) = jobs.lock()
        && let Some(job) = store.get_mut(&job_id)
    {
        job.scanned = total;
        job.state = ScanState::Done;
    }
}

fn enumerate_discovery_hosts(
    cidr_input: &str,
    limits: WebLimits,
) -> Result<Vec<std::net::Ipv4Addr>, AppError> {
    let cidrs = cidr_input
        .split(',')
        .map(str::trim)
        .filter(|cidr| !cidr.is_empty())
        .collect::<Vec<_>>();

    if cidrs.is_empty() {
        return Err(AppError::Validation("cidr is required".to_string()));
    }
    if limits
        .max_discovery_cidrs
        .is_some_and(|limit| cidrs.len() > limit)
    {
        return Err(AppError::Validation(
            "too many CIDR ranges requested".to_string(),
        ));
    }

    let mut hosts = Vec::new();
    for cidr in cidrs {
        if limits.max_discovery_cidrs.is_some() {
            crate::device::discovery::validate_community_scan_range(cidr)?;
        }
        hosts.extend(crate::device::discovery::enumerate_cidr_hosts(cidr)?);
    }

    hosts.sort();
    hosts.dedup();
    Ok(hosts)
}

/// GET /api/discovery/scan/<job_id>
fn api_scan_status(job_id: &str, jobs: &JobStore) -> String {
    let store = match jobs.lock() {
        Ok(s) => s,
        Err(_) => return r#"{"error":"internal lock error"}"#.to_string(),
    };

    let Some(job) = store.get(job_id) else {
        return r#"{"error":"job not found"}"#.to_string();
    };

    let elapsed = job.started_at.elapsed().as_secs();

    match &job.state {
        ScanState::Running => {
            format!(
                r#"{{"status":"running","scanned":{},"total":{},"elapsed":{elapsed}}}"#,
                job.scanned, job.total
            )
        }
        ScanState::Done => {
            let items: Vec<String> = job
                .found
                .iter()
                .map(|d| {
                    format!(
                        r#"{{"ip":"{}","name":"{}","status":"{}","community":"{}"}}"#,
                        escape_json(&d.ip),
                        escape_json(&d.name),
                        escape_json(&d.status),
                        escape_json(&d.community),
                    )
                })
                .collect();
            format!(
                r#"{{"status":"done","scanned":{},"total":{},"elapsed":{elapsed},"devices":[{}]}}"#,
                job.scanned,
                job.total,
                items.join(",")
            )
        }
        ScanState::Error(msg) => {
            format!(
                r#"{{"status":"error","elapsed":{elapsed},"error":"{}"}}"#,
                escape_json(msg)
            )
        }
    }
}

fn api_discovery_topology(
    body: &str,
    cfg: &Arc<Mutex<AppConfig>>,
    repo: &Arc<Mutex<Repository>>,
    limits: WebLimits,
) -> String {
    let params = parse_form(body);
    let seed_ip = params.get("seed_ip").cloned().unwrap_or_default();
    if seed_ip.trim().is_empty() {
        return r#"{"error":"seed_ip is required"}"#.to_string();
    }

    let default_community = cfg
        .lock()
        .map(|c| c.snmp.default_community.clone())
        .unwrap_or_else(|_| "public".to_string());
    let community = params
        .get("community")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(default_community);

    match discover_topology_once(&seed_ip, &community) {
        Ok(mut report) => {
            enrich_topology_nodes(&mut report, repo);
            annotate_edge_health(&mut report);
            report.apply_node_limit(limits);
            report.to_json()
        }
        Err(err) => format!(r#"{{"error":"{}"}}"#, escape_json(&err.to_string())),
    }
}

/// LLDP と CDP が同一物理リンクを重複検知した場合、対向ポート名が一致するものを1本へ統合する。
/// 統合しないと同一座標に2本の線・ラベルが重なって描画され、文字が二重に潰れて見える。
fn merge_duplicate_edges(edges: Vec<WebTopologyEdge>) -> Vec<WebTopologyEdge> {
    let mut merged: Vec<WebTopologyEdge> = Vec::new();
    for edge in edges {
        let remote_port = edge.remote_port.as_deref().unwrap_or("").trim().to_string();
        let duplicate = if remote_port.is_empty() {
            None
        } else {
            merged.iter_mut().find(|candidate| {
                candidate.target == edge.target
                    && candidate.remote_port.as_deref().unwrap_or("").trim() == remote_port
            })
        };

        match duplicate {
            Some(candidate) => {
                if !candidate.protocol.split('+').any(|p| p == edge.protocol) {
                    candidate.protocol = format!("{}+{}", candidate.protocol, edge.protocol);
                }
                candidate.local_if_index = candidate.local_if_index.or(edge.local_if_index);
                candidate.local_port = candidate.local_port.clone().or(edge.local_port);
                candidate.remote_ip = candidate.remote_ip.clone().or(edge.remote_ip);
                candidate.remote_hostname =
                    candidate.remote_hostname.clone().or(edge.remote_hostname);
            }
            None => merged.push(edge),
        }
    }
    merged
}

fn discover_topology_once(seed_ip: &str, community: &str) -> Result<WebTopologyReport, AppError> {
    let client = SnmpClient::new(community);
    let device = DeviceConfig::new(seed_ip.to_string(), community.to_string());
    let mut warnings = Vec::new();
    let mut edges = Vec::new();

    let local_if_names = match client.walk_oid(&device, &IF_NAME_OID) {
        Ok(varbinds) => parse_if_names(&varbinds),
        Err(err) => {
            warnings.push(format!("ifName walk failed: {err}"));
            BTreeMap::new()
        }
    };

    match client.walk_oid(&device, &LLDP_REMOTE_SYSTEMS_DATA_OID) {
        Ok(varbinds) => edges.extend(parse_lldp_edges(seed_ip, &varbinds)),
        Err(err) => warnings.push(format!("LLDP walk failed: {err}")),
    }
    match client.walk_oid(&device, &LLDP_REM_MAN_ADDR_OID) {
        Ok(varbinds) => merge_lldp_management_addresses(&mut edges, &varbinds),
        Err(err) => warnings.push(format!("LLDP management address walk failed: {err}")),
    }
    match client.walk_oid(&device, &CDP_CACHE_TABLE_OID) {
        Ok(varbinds) => edges.extend(parse_cdp_edges(seed_ip, &varbinds)),
        Err(err) => warnings.push(format!("CDP walk failed: {err}")),
    }

    for (position, edge) in edges.iter_mut().enumerate() {
        edge.id = format!("e{}", position);
        edge.local_port = edge
            .local_if_index
            .and_then(|if_index| local_if_names.get(&if_index).cloned());
        edge.target = remote_node_id(edge, position);
    }
    let edges = merge_duplicate_edges(edges);

    let mut report = WebTopologyReport {
        seed_ip: seed_ip.to_string(),
        nodes: Vec::new(),
        edges,
        warnings,
        total_nodes: 0,
        hidden_nodes: 0,
        node_limit: None,
    };
    report.rebuild_nodes(&local_if_names);
    Ok(report)
}

/// LLDP/CDP で対向 IP が取れない隣接も 1 ノードとして描画できるよう ID を割り当てる。
fn remote_node_id(edge: &WebTopologyEdge, position: usize) -> String {
    if let Some(ip) = edge.remote_ip.as_ref().filter(|ip| !ip.trim().is_empty()) {
        return ip.clone();
    }
    if let Some(hostname) = edge
        .remote_hostname
        .as_ref()
        .filter(|name| !name.trim().is_empty())
    {
        return format!("name:{hostname}");
    }
    format!("unknown:{position}")
}

fn parse_if_names(varbinds: &[crate::snmp::SnmpVarBind]) -> BTreeMap<i64, String> {
    let mut names = BTreeMap::new();
    for varbind in varbinds {
        let Some(suffix) = varbind.index_suffix(&IF_NAME_OID) else {
            continue;
        };
        let Some(if_index) = suffix.first().map(|value| i64::from(*value)) else {
            continue;
        };
        if let Some(name) = varbind.value.as_string() {
            names.insert(if_index, name);
        }
    }
    names
}

/// 登録済みデバイスの状態と最新インターフェース情報をノードへ反映する。
fn enrich_topology_nodes(report: &mut WebTopologyReport, repo: &Arc<Mutex<Repository>>) {
    let Ok(repo) = repo.lock() else {
        return;
    };

    for node in report.nodes.iter_mut() {
        let Ok(Some(device)) = repo.find_device_by_ip(&node.ip) else {
            continue;
        };
        node.status = device.status.clone();
        if node.hostname.is_none() && !device.name.trim().is_empty() {
            node.hostname = Some(device.name.clone());
        }
        if matches!(
            device.device_type.to_ascii_lowercase().as_str(),
            "switch" | "router" | "firewall"
        ) {
            node.kind = "switch";
        }
        if let Some(device_id) = device.id
            && let Ok(samples) = repo.get_latest_interfaces(device_id)
        {
            let deltas = repo
                .get_interface_port_deltas(device_id)
                .unwrap_or_default();
            node.interfaces = samples
                .into_iter()
                .map(|sample| {
                    let delta = deltas.get(&sample.if_index).copied().unwrap_or_default();
                    let (health_status, alerts) = evaluate_port_health(&delta);
                    WebTopologyInterface {
                        if_index: Some(i64::from(sample.if_index)),
                        if_name: sample.if_name,
                        link_status: sample.link_status,
                        metrics: delta,
                        health_status,
                        alerts,
                    }
                })
                .collect();
        }
    }
}

/// エラー種別ごとの障害判定しきい値（CRC エラーはこの件数以上で Critical）。
const PORT_ERROR_CRITICAL_THRESHOLD: u64 = 50;

/// SNMP エラーカウンター差分から3パターンの障害を判定する。
/// 1) L1 物理障害（CRC/フレームエラー） 2) L2 Duplex ミスマッチ（Late Collision） 3) 輻輳（バッファ溢れ）
fn evaluate_port_health(delta: &InterfacePortDelta) -> (String, Vec<String>) {
    let mut alerts = Vec::new();
    let mut critical = false;

    if delta.in_errors_delta > 0 {
        alerts.push("L1 Physical Error (CRC/Frame Error)".to_string());
        if delta.in_errors_delta >= PORT_ERROR_CRITICAL_THRESHOLD {
            critical = true;
        }
    }
    if delta.late_collisions_delta > 0 {
        alerts.push("Duplex Mismatch / Late Collision".to_string());
    }
    if delta.in_discards_delta > 0 || delta.out_discards_delta > 0 {
        alerts.push("Buffer Overflow / Congestion (Packet Discards)".to_string());
    }

    let status = if critical {
        "Critical"
    } else if !alerts.is_empty() {
        "Warning"
    } else {
        "Healthy"
    };

    (status.to_string(), alerts)
}

/// エッジのローカル側ポートの健全性を、対応するソースノードのインターフェース情報から反映する。
fn annotate_edge_health(report: &mut WebTopologyReport) {
    for edge in report.edges.iter_mut() {
        let Some(local_if_index) = edge.local_if_index else {
            continue;
        };
        let Some(source_node) = report.nodes.iter().find(|node| node.id == edge.source) else {
            continue;
        };
        let Some(iface) = source_node
            .interfaces
            .iter()
            .find(|iface| iface.if_index == Some(local_if_index))
        else {
            continue;
        };
        if iface.health_status != "Healthy" {
            edge.health_status = Some(iface.health_status.clone());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebTopologyReport {
    seed_ip: String,
    nodes: Vec<WebTopologyNode>,
    edges: Vec<WebTopologyEdge>,
    warnings: Vec<String>,
    total_nodes: usize,
    hidden_nodes: usize,
    node_limit: Option<usize>,
}

impl WebTopologyReport {
    /// エッジ集合からノード一覧を再構築する。シード機器を先頭に固定する。
    fn rebuild_nodes(&mut self, local_if_names: &BTreeMap<i64, String>) {
        let mut nodes: Vec<WebTopologyNode> = vec![WebTopologyNode {
            id: self.seed_ip.clone(),
            ip: self.seed_ip.clone(),
            hostname: None,
            kind: "switch",
            status: "unknown".to_string(),
            is_seed: true,
            interfaces: local_if_names
                .iter()
                .map(|(if_index, if_name)| WebTopologyInterface {
                    if_index: Some(*if_index),
                    if_name: if_name.clone(),
                    link_status: "unknown".to_string(),
                    metrics: InterfacePortDelta::default(),
                    health_status: "Healthy".to_string(),
                    alerts: Vec::new(),
                })
                .collect(),
        }];

        let mut degrees: HashMap<String, usize> = HashMap::new();
        for edge in &self.edges {
            *degrees.entry(edge.target.clone()).or_insert(0) += 1;
            if !nodes.iter().any(|node| node.id == edge.target) {
                nodes.push(WebTopologyNode {
                    id: edge.target.clone(),
                    ip: edge.remote_ip.clone().unwrap_or_default(),
                    hostname: edge.remote_hostname.clone(),
                    kind: "endpoint",
                    status: "unknown".to_string(),
                    is_seed: false,
                    interfaces: Vec::new(),
                });
            }
        }

        for node in nodes.iter_mut().filter(|node| !node.is_seed) {
            if degrees.get(&node.id).copied().unwrap_or(0) >= 2 {
                node.kind = "switch";
            }
        }

        self.total_nodes = nodes.len();
        self.nodes = nodes;
    }

    fn apply_node_limit(&mut self, limits: WebLimits) {
        self.node_limit = limits.max_devices;
        let Some(limit) = self.node_limit else {
            self.hidden_nodes = 0;
            return;
        };
        if self.nodes.len() <= limit {
            self.hidden_nodes = 0;
            return;
        }

        self.hidden_nodes = self.nodes.len() - limit;
        self.nodes.truncate(limit);
        let visible: std::collections::HashSet<String> =
            self.nodes.iter().map(|node| node.id.clone()).collect();
        self.edges
            .retain(|edge| visible.contains(&edge.source) && visible.contains(&edge.target));
    }

    fn to_json(&self) -> String {
        let nodes = self
            .nodes
            .iter()
            .map(WebTopologyNode::to_json)
            .collect::<Vec<_>>()
            .join(",");
        let edges = self
            .edges
            .iter()
            .map(WebTopologyEdge::to_json)
            .collect::<Vec<_>>()
            .join(",");
        let warnings = self
            .warnings
            .iter()
            .map(|warning| format!(r#""{}""#, escape_json(warning)))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"{{"seed_ip":"{}","edition":"{}","node_limit":{},"total_nodes":{},"hidden_nodes":{},"nodes":[{}],"edges":[{}],"warnings":[{}]}}"#,
            escape_json(&self.seed_ip),
            "configured",
            self.node_limit
                .map(|limit| limit.to_string())
                .unwrap_or_else(|| "null".to_string()),
            self.total_nodes,
            self.hidden_nodes,
            nodes,
            edges,
            warnings
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebTopologyInterface {
    if_index: Option<i64>,
    if_name: String,
    link_status: String,
    metrics: InterfacePortDelta,
    health_status: String,
    alerts: Vec<String>,
}

impl WebTopologyInterface {
    fn to_json(&self) -> String {
        let alerts = self
            .alerts
            .iter()
            .map(|alert| format!(r#""{}""#, escape_json(alert)))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"if_index":{},"if_name":"{}","link_status":"{}","metrics":{{"in_errors_delta":{},"out_errors_delta":{},"in_discards_delta":{},"out_discards_delta":{},"late_collisions_delta":{}}},"health_status":"{}","alerts":[{}]}}"#,
            json_opt_i64(self.if_index),
            escape_json(&self.if_name),
            escape_json(&self.link_status),
            self.metrics.in_errors_delta,
            self.metrics.out_errors_delta,
            self.metrics.in_discards_delta,
            self.metrics.out_discards_delta,
            self.metrics.late_collisions_delta,
            escape_json(&self.health_status),
            alerts
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebTopologyNode {
    id: String,
    ip: String,
    hostname: Option<String>,
    kind: &'static str,
    status: String,
    is_seed: bool,
    interfaces: Vec<WebTopologyInterface>,
}

impl WebTopologyNode {
    fn to_json(&self) -> String {
        let interfaces = self
            .interfaces
            .iter()
            .map(WebTopologyInterface::to_json)
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"{{"id":"{}","ip":"{}","hostname":"{}","label":"{}","kind":"{}","status":"{}","seed":{},"interfaces":[{}]}}"#,
            escape_json(&self.id),
            escape_json(&self.ip),
            escape_json(self.hostname.as_deref().unwrap_or("")),
            escape_json(self.label()),
            self.kind,
            escape_json(&self.status),
            self.is_seed,
            interfaces
        )
    }

    fn label(&self) -> &str {
        self.hostname
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .or(Some(self.ip.as_str()))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(self.id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebTopologyEdge {
    id: String,
    protocol: String,
    source: String,
    target: String,
    local_ip: String,
    local_if_index: Option<i64>,
    local_port: Option<String>,
    remote_ip: Option<String>,
    remote_hostname: Option<String>,
    remote_port: Option<String>,
    health_status: Option<String>,
}

impl WebTopologyEdge {
    fn to_json(&self) -> String {
        format!(
            r#"{{"id":"{}","protocol":"{}","source":"{}","target":"{}","label":"{}","local_ip":"{}","local_if_index":{},"local_port":"{}","remote_ip":"{}","remote_hostname":"{}","remote_port":"{}","health_status":"{}"}}"#,
            escape_json(&self.id),
            escape_json(&self.protocol),
            escape_json(&self.source),
            escape_json(&self.target),
            escape_json(&self.label()),
            escape_json(&self.local_ip),
            json_opt_i64(self.local_if_index),
            escape_json(self.local_port.as_deref().unwrap_or("")),
            escape_json(self.remote_ip.as_deref().unwrap_or("")),
            escape_json(self.remote_hostname.as_deref().unwrap_or("")),
            escape_json(self.remote_port.as_deref().unwrap_or("")),
            escape_json(self.health_status.as_deref().unwrap_or("Healthy")),
        )
    }

    /// 例: `Gi1/0/11 \u{2194} Gi1/0/23`
    fn label(&self) -> String {
        let local = self
            .local_port
            .clone()
            .or_else(|| self.local_if_index.map(|index| format!("if-{index}")))
            .unwrap_or_else(|| "?".to_string());
        let remote = self
            .remote_port
            .clone()
            .filter(|port| !port.trim().is_empty())
            .unwrap_or_else(|| "?".to_string());
        format!("{local} \u{2194} {remote}")
    }
}

fn json_opt_i64(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

#[derive(Debug, Clone, Default)]
struct NeighborRow {
    local_if_index: Option<i64>,
    remote_ip: Option<String>,
    remote_hostname: Option<String>,
    remote_port: Option<String>,
}

const LLDP_REMOTE_SYSTEMS_DATA_OID: [u32; 10] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1];
const LLDP_REM_CHASSIS_ID_OID: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 5];
const LLDP_REM_PORT_ID_OID: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 7];
const LLDP_REM_SYS_NAME_OID: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 9];
const LLDP_REM_MAN_ADDR_OID: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 2, 1, 4];
const CDP_CACHE_TABLE_OID: [u32; 13] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1];
const CDP_CACHE_ADDRESS_OID: [u32; 14] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1, 4];
const CDP_CACHE_DEVICE_ID_OID: [u32; 14] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1, 6];
const CDP_CACHE_DEVICE_PORT_OID: [u32; 14] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1, 7];
const IF_NAME_OID: [u32; 11] = [1, 3, 6, 1, 2, 1, 31, 1, 1, 1, 1];

fn parse_lldp_edges(seed_ip: &str, varbinds: &[crate::snmp::SnmpVarBind]) -> Vec<WebTopologyEdge> {
    let mut rows: BTreeMap<String, NeighborRow> = BTreeMap::new();
    for varbind in varbinds {
        if let Some(suffix) = varbind.index_suffix(&LLDP_REM_SYS_NAME_OID) {
            // lldpRemEntry の index は [timeMark, localPortNum, index] の3要素
            let row = neighbor_row(&mut rows, suffix, 1);
            row.remote_hostname = varbind.value.as_string();
        } else if let Some(suffix) = varbind.index_suffix(&LLDP_REM_PORT_ID_OID) {
            let row = neighbor_row(&mut rows, suffix, 1);
            row.remote_port = varbind.value.as_string();
        } else if let Some(suffix) = varbind.index_suffix(&LLDP_REM_CHASSIS_ID_OID) {
            let row = neighbor_row(&mut rows, suffix, 1);
            if row.remote_hostname.is_none() {
                row.remote_hostname = varbind.value.as_string();
            }
        }
    }

    rows.into_values()
        .map(|row| WebTopologyEdge {
            id: String::new(),
            protocol: "LLDP".to_string(),
            source: seed_ip.to_string(),
            target: String::new(),
            local_ip: seed_ip.to_string(),
            local_if_index: row.local_if_index,
            local_port: None,
            remote_ip: row.remote_ip,
            remote_hostname: row.remote_hostname,
            remote_port: row.remote_port,
            health_status: None,
        })
        .collect()
}

fn merge_lldp_management_addresses(
    edges: &mut [WebTopologyEdge],
    varbinds: &[crate::snmp::SnmpVarBind],
) {
    let mut addresses = BTreeMap::new();
    for varbind in varbinds {
        if let Some(suffix) = varbind.index_suffix(&LLDP_REM_MAN_ADDR_OID) {
            let key = lldp_remote_row_key(suffix).unwrap_or_else(|| suffix_to_key(suffix));
            if let Some(ip) = snmp_value_to_ip(&varbind.value) {
                addresses.insert(key, ip);
            }
        }
    }

    for edge in edges.iter_mut().filter(|edge| edge.protocol == "LLDP") {
        if edge.remote_ip.is_none()
            && let Some(local_if_index) = edge.local_if_index
            && let Some((_, ip)) = addresses
                .iter()
                .find(|(key, _)| key.split('.').nth(1) == Some(&local_if_index.to_string()))
        {
            edge.remote_ip = Some(ip.clone());
        }
    }
}

fn parse_cdp_edges(seed_ip: &str, varbinds: &[crate::snmp::SnmpVarBind]) -> Vec<WebTopologyEdge> {
    let mut rows: BTreeMap<String, NeighborRow> = BTreeMap::new();
    for varbind in varbinds {
        if let Some(suffix) = varbind.index_suffix(&CDP_CACHE_DEVICE_ID_OID) {
            // cdpCacheEntry の index は [ifIndex, deviceIndex] の2要素（LLDPとは位置が違う）
            let row = neighbor_row(&mut rows, suffix, 0);
            row.remote_hostname = varbind.value.as_string();
        } else if let Some(suffix) = varbind.index_suffix(&CDP_CACHE_DEVICE_PORT_OID) {
            let row = neighbor_row(&mut rows, suffix, 0);
            row.remote_port = varbind.value.as_string();
        } else if let Some(suffix) = varbind.index_suffix(&CDP_CACHE_ADDRESS_OID) {
            let row = neighbor_row(&mut rows, suffix, 0);
            row.remote_ip = snmp_value_to_ip(&varbind.value);
        }
    }

    rows.into_values()
        .map(|row| WebTopologyEdge {
            id: String::new(),
            protocol: "CDP".to_string(),
            source: seed_ip.to_string(),
            target: String::new(),
            local_ip: seed_ip.to_string(),
            local_if_index: row.local_if_index,
            local_port: None,
            remote_ip: row.remote_ip,
            remote_hostname: row.remote_hostname,
            remote_port: row.remote_port,
            health_status: None,
        })
        .collect()
}

fn neighbor_row<'a>(
    rows: &'a mut BTreeMap<String, NeighborRow>,
    suffix: &[u32],
    local_index_pos: usize,
) -> &'a mut NeighborRow {
    let key = suffix_to_key(suffix);
    rows.entry(key).or_insert_with(|| NeighborRow {
        local_if_index: suffix
            .get(local_index_pos)
            .or_else(|| suffix.first())
            .map(|value| i64::from(*value)),
        ..NeighborRow::default()
    })
}

fn lldp_remote_row_key(suffix: &[u32]) -> Option<String> {
    (suffix.len() >= 3).then(|| suffix_to_key(&suffix[..3]))
}

fn suffix_to_key(suffix: &[u32]) -> String {
    suffix
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

fn snmp_value_to_ip(value: &crate::snmp::SnmpValue) -> Option<String> {
    if let crate::snmp::SnmpValue::IpAddress(addr) = value {
        return Some(format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3]));
    }

    if let Some(bytes) = value.as_bytes()
        && bytes.len() >= 4
    {
        return Some(format!(
            "{}.{}.{}.{}",
            bytes[0], bytes[1], bytes[2], bytes[3]
        ));
    }

    value
        .as_string()
        .filter(|text| text.parse::<std::net::Ipv4Addr>().is_ok())
}

/// POST /api/discovery/register
fn api_register(body: &str, repo: &Arc<Mutex<Repository>>, limits: WebLimits) -> String {
    let devices = parse_register_body(body);
    if devices.is_empty() {
        return r#"{"error":"no devices provided"}"#.to_string();
    }

    let Ok(repo) = repo.lock() else {
        return r#"{"error":"repository lock failed"}"#.to_string();
    };

    let current_count = repo.list_devices().map(|d| d.len()).unwrap_or(0);
    let mut remaining_slots = limits
        .max_devices
        .map(|limit| limit.saturating_sub(current_count));

    let mut registered = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();

    for mut device in devices {
        // Discovery で online を確認済みなので登録時に status を反映
        if device.status.is_empty() || device.status == "unknown" {
            device.status = "online".to_string();
        }
        let now = chrono::Utc::now().to_rfc3339();
        if device.last_seen_at.is_none() {
            device.last_seen_at = Some(now);
        }
        match repo.find_device_by_ip(&device.ip) {
            Ok(Some(_)) => skipped.push(device.ip.clone()),
            Ok(None) => {
                if remaining_slots == Some(0) {
                    errors.push(format!(
                        "{}: Community版の登録上限（{}台）に達しています",
                        device.ip, COMMUNITY_MAX_DEVICES
                    ));
                    continue;
                }
                match repo.save_device_config(&device) {
                    Ok(_) => {
                        registered.push(device.ip.clone());
                        if let Some(slots) = remaining_slots.as_mut() {
                            *slots = slots.saturating_sub(1);
                        }
                    }
                    Err(err) => errors.push(format!("{}: {}", device.ip, err)),
                }
            }
            Err(err) => errors.push(format!("{}: {}", device.ip, err)),
        }
    }

    let reg_json: Vec<String> = registered
        .iter()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .collect();
    let skip_json: Vec<String> = skipped
        .iter()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .collect();
    let err_json: Vec<String> = errors
        .iter()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .collect();

    format!(
        r#"{{"registered":[{}],"skipped":[{}],"errors":[{}]}}"#,
        reg_json.join(","),
        skip_json.join(","),
        err_json.join(","),
    )
}

// ─── Pages ───────────────────────────────────────────────────────────────────

fn page_dashboard(
    repo: &Arc<Mutex<Repository>>,
    cfg: &Arc<Mutex<AppConfig>>,
    extension: Option<&dyn WebExtensionProvider>,
) -> String {
    let spike_threshold = cfg.lock().map(|c| c.alert.spike_threshold).unwrap_or(10);
    let (devices, spikes_count, devices_json, recent_alerts) = {
        if let Ok(r) = repo.lock() {
            let devs = r.list_devices().unwrap_or_default();
            let mut sc = 0usize;
            let json: Vec<String> = devs.iter().map(|d| {
                let spike = d.id
                    .and_then(|id| r.check_interface_spikes(id, spike_threshold).ok())
                    .map(|spikes| !spikes.is_empty())
                    .unwrap_or(false);
                if spike { sc += 1; }
                format!(
                    r#"{{"ip":"{}","name":"{}","status":"{}","community":"{}","last_seen":"{}","error_spike":{}}}"#,
                    escape_json(&d.ip),
                    escape_json(&d.name),
                    escape_json(&d.status),
                    escape_json(&d.community),
                    escape_json(d.last_seen_at.as_deref().unwrap_or("")),
                    spike,
                )
            }).collect();
            let alerts = r.get_recent_alerts(10).unwrap_or_default();
            (devs, sc, json, alerts)
        } else {
            (vec![], 0, vec![], vec![])
        }
    };

    let healthy = devices.iter().filter(|d| d.status == "online").count();
    let offline = devices.iter().filter(|d| d.status == "offline").count();
    let warning = devices.iter().filter(|d| d.status == "warning").count();
    let total = devices.len();
    let spike_card_cls = if spikes_count > 0 {
        "card card-spike"
    } else {
        "card"
    };

    let mut html = String::new();
    html.push_str(HTML_DOCTYPE);
    html.push_str("<html lang='en'><head>");
    html.push_str("<meta charset='utf-8'>");
    html.push_str("<meta name='viewport' content='width=device-width,initial-scale=1'>");
    html.push_str("<script src='/static/js/theme.js'></script>");
    html.push_str("<title>TracePulse \u{2013} Dashboard</title>");
    html.push_str(COMMON_CSS);
    html.push_str(DASHBOARD_CSS);
    if let Some(extension) = extension {
        html.push_str(&extension.dashboard_styles());
    }
    html.push_str("</head><body data-title-key='dashboard_title'>");
    html.push_str(&navigation_html(extension));
    html.push_str("<main><h1 data-i18n='dashboard_title'>Dashboard</h1><div id='summary-cards' class='summary-cards'>");
    html.push_str(&format!("<div class='card card-online'><div class='card-num'>{healthy}</div><div class='card-label' data-i18n='online'>Online</div></div>"));
    html.push_str(&format!("<div class='card card-warning'><div class='card-num'>{warning}</div><div class='card-label' data-i18n='warning'>Warning</div></div>"));
    html.push_str(&format!("<div class='card card-offline'><div class='card-num'>{offline}</div><div class='card-label' data-i18n='offline'>Offline</div></div>"));
    html.push_str(&format!("<div class='card'><div class='card-num'>{total}</div><div class='card-label' data-i18n='total'>Total</div></div>"));
    html.push_str(&format!("<div class='{spike_card_cls}'><div class='card-num'>{spikes_count}</div><div class='card-label' data-i18n='error_spikes'>Error Spikes</div></div>"));
    html.push_str("</div>");
    html.push_str("<p id='last-refreshed' style='font-size:.8rem;color:#64748b;margin-bottom:.75rem;text-align:right'></p>");
    if let Some(extension) = extension {
        if let Some(markup) = extension.dashboard_html() {
            html.push_str(&markup);
        }
    }
    html.push_str("<table id='dash-table'><thead><tr>");
    html.push_str("<th id='th-ip' onclick='sortBy(\"ip\")' data-i18n='ip_address'>IP <span class='sort-icon' id='sort-ip'></span></th>");
    html.push_str("<th id='th-name' onclick='sortBy(\"name\")' data-i18n='hostname'>Name <span class='sort-icon' id='sort-name'></span></th>");
    html.push_str("<th id='th-status' onclick='sortBy(\"status\")' data-i18n='status'>Status <span class='sort-icon' id='sort-status'></span></th>");
    html.push_str("<th id='th-community' onclick='sortBy(\"community\")' data-i18n='community'>Community <span class='sort-icon' id='sort-community'></span></th>");
    html.push_str("<th id='th-last_seen' onclick='sortBy(\"last_seen\")' data-i18n='time'>Last Seen <span class='sort-icon' id='sort-last_seen'></span></th>");
    html.push_str("<th data-i18n='actions'>Actions</th>");
    html.push_str("</tr></thead><tbody id='dash-tbody'></tbody></table>");
    html.push_str("<section class='detail-section' style='margin-top:1.25rem'>");
    html.push_str("<h2 data-i18n='recent_alerts_title'>Recent Alerts</h2>");
    html.push_str("<table id='alert-table'><thead><tr><th data-i18n='time'>Time</th><th data-i18n='hostname'>Device</th><th data-i18n='type'>Type</th><th data-i18n='severity'>Severity</th><th data-i18n='details'>Details</th></tr></thead><tbody>");
    if recent_alerts.is_empty() {
        html.push_str("<tr><td colspan='5' class='empty'>No alerts recorded</td></tr>");
    } else {
        for alert in recent_alerts {
            let cls = match alert.severity.as_str() {
                "critical" => "sev-critical",
                "warning" => "sev-warning",
                _ => "sev-info",
            };
            html.push_str(&format!(
              "<tr><td class='timestamp' data-timestamp='{}'>{}</td><td><a href='/device/{}'>{}</a></td><td>{}</td><td><span class='{}'>{}</span></td><td>{}</td></tr>",
                escape_html(alert.created_at.as_deref().unwrap_or("")),
              escape_html(alert.created_at.as_deref().unwrap_or("")),
                escape_html(&alert.device_ip),
                escape_html(&alert.device_name),
                escape_html(&alert.alert_type),
                cls,
                escape_html(&alert.severity),
                escape_html(&alert.details),
            ));
        }
    }
    html.push_str("</tbody></table></section>");
    html.push_str("</main>");
    html.push_str("<script>\nvar DEVICES = [");
    html.push_str(&devices_json.join(","));
    html.push_str("];\n");
    html.push_str("</script><script src='/static/js/i18n.js?v=2'></script><script src='/static/js/dashboard.js?v=2'></script>");
    if let Some(extension) = extension {
        html.push_str(&extension.dashboard_scripts());
    }
    html.push_str("</body></html>");
    html
}

fn page_settings(
    cfg: &Arc<Mutex<AppConfig>>,
    notifications_enabled: bool,
    extension: Option<&dyn WebExtensionProvider>,
) -> String {
    let mut html = String::new();
    html.push_str(HTML_DOCTYPE);
    html.push_str("<html lang='en'><head>");
    html.push_str("<meta charset='utf-8'>");
    html.push_str("<meta name='viewport' content='width=device-width,initial-scale=1'>");
    html.push_str("<script src='/static/js/theme.js'></script>");
    html.push_str("<title>TracePulse \u{2013} Settings</title>");
    html.push_str(COMMON_CSS);
    html.push_str(SETTINGS_CSS);
    html.push_str("</head><body data-title-key='settings_title'>");
    html.push_str(&navigation_html(extension));
    html.push_str(
        SETTINGS_HTML
            .strip_suffix("</main>")
            .unwrap_or(SETTINGS_HTML),
    );
    if notifications_enabled {
        html.push_str(NOTIFICATIONS_HTML);
    }
    if let Some(extension) = extension {
        html.push_str(&extension.settings_html());
    }
    html.push_str(SETTINGS_ACTIONS_HTML);
    html.push_str("</main>");
    html.push_str("<script>\n");
    // 現在値をサーバー側でレンダリングして初期値として埋め込む
    if let Ok(c) = cfg.lock() {
        html.push_str(&format!(
            "var INIT = {{interval:{},community:'{}',error_rate:{},spike:{},warn:{},crit:{},days:{},timezone:'{}'}};\n",
            c.polling.interval_seconds,
            escape_json(&c.snmp.default_community),
            c.alert.error_rate_threshold,
            c.alert.spike_threshold,
            c.alert.health_warning_threshold,
            c.alert.health_critical_threshold,
            c.retention.history_days,
            escape_json(&c.display.timezone),
        ));
    } else {
        html.push_str("var INIT = {interval:30,community:'public',error_rate:0.05,spike:10,warn:80,crit:60,days:7,timezone:'utc'};\n");
    }
    html.push_str("</script><script src='/static/js/i18n.js'></script><script src='/static/js/settings.js'></script>");
    if notifications_enabled {
        html.push_str("<script src='/static/js/notifications.js'></script>");
    }
    if let Some(extension) = extension {
        html.push_str(&extension.settings_scripts());
    }
    html.push_str("</body></html>");
    html
}

fn page_device_detail(ip: &str, repo: &Arc<Mutex<Repository>>) -> String {
    let mut html = String::new();
    html.push_str(HTML_DOCTYPE);
    html.push_str("<html lang='en'><head>");
    html.push_str("<meta charset='utf-8'>");
    html.push_str("<meta name='viewport' content='width=device-width,initial-scale=1'>");
    html.push_str("<script src='/static/js/theme.js'></script>");
    html.push_str(&format!(
        "<title>TracePulse \u{2013} {}</title>",
        escape_json(ip)
    ));
    html.push_str(COMMON_CSS);
    html.push_str(DEVICE_DETAIL_CSS);
    html.push_str("</head><body data-title-key='device_title_prefix'>");
    html.push_str(NAV_HTML);

    // 404 check
    let exists = repo
        .lock()
        .ok()
        .and_then(|r| r.find_device_by_ip(ip).ok())
        .flatten()
        .is_some();

    if !exists {
        html.push_str(
            "<main><p style='color:#f87171;margin-top:2rem'>Device not found.</p></main>",
        );
        html.push_str("</body></html>");
        return html;
    }

    html.push_str("<main><div style='display:flex;align-items:center;gap:1rem;margin-bottom:1rem;flex-wrap:wrap'>\
         <a href='/' style='color:#94a3b8;font-size:.85rem;text-decoration:none' data-i18n='nav_dashboard'>&#8592; Dashboard</a>\
         <h1 id='dev-title' style='margin:0'>Loading...</h1>\
         <span id='dev-status' class='status-unknown'>unknown</span>\
         <span id='device-refresh' style='font-size:.8rem;color:#64748b;margin-left:auto'>Device data auto-refresh: 30s</span>\
         </div>");

    // セクション: インターフェース一覧
    html.push_str("<section class='detail-section'>");
    html.push_str("<h2 data-i18n='interface_status'>Interface Status</h2>");
    html.push_str("<div class='table-scroll interface-table-scroll'><table id='if-table'><thead><tr>\
        <th>#</th><th data-i18n='interface'>Interface</th><th data-i18n='status'>Link</th>\
        <th data-i18n='diagnostic_status'>Diagnostic Status</th>\
        <th data-i18n='in_errors'>In Errors</th><th data-i18n='out_errors'>Out Errors</th>\
        <th data-i18n='in_discards'>In Discards</th><th data-i18n='out_discards'>Out Discards</th>\
        <th data-i18n='late_collisions'>Late Collisions</th>\
        <th><span data-i18n='bandwidth_line1'>Bandwidth</span><br><span data-i18n='bandwidth_line2'>Utilization (%)</span></th><th data-i18n='time'>Sampled At</th>\
        </tr></thead><tbody id='if-tbody'><tr><td colspan=11>Loading...</td></tr></tbody></table></div>");
    html.push_str("</section>");

    // セクション: 破損パケット内訳（EtherLike-MIB Error Breakdown）
    html.push_str("<section class='detail-section'>");
    html.push_str("<h2 data-i18n='error_breakdown_title'>Error Breakdown</h2>");
    html.push_str("<div id='error-breakdown' class='error-breakdown'></div>");
    html.push_str("</section>");

    html.push_str(
        "<section class='detail-section' id='traffic-protocols-section' style='display:none'>",
    );
    html.push_str("<h2 data-i18n='traffic_protocols_title'>Traffic &amp; Protocols</h2>");
    html.push_str("<p class='traffic-note' data-i18n='traffic_protocols_note'>Values are estimates based on received flows; device-side sampling is not corrected.</p>");
    html.push_str("<div id='traffic-protocols' class='traffic-protocols'></div>");
    html.push_str("</section>");

    html.push_str("<section class='detail-section'>");
    html.push_str("<h2 data-i18n='interface_filter_title'>Interface Selection</h2>");
    html.push_str("<div class='iface-filter-actions'>");
    html.push_str("<button class='btn btn-secondary' type='button' onclick='selectAllInterfaces(true)' data-i18n='select_all_interfaces'>Select All</button>");
    html.push_str("<button class='btn btn-secondary' type='button' onclick='selectAllInterfaces(false)' data-i18n='clear_all_interfaces'>Clear All</button>");
    html.push_str("<span class='iface-filter-note' id='iface-filter-note'></span>");
    html.push_str("</div>");
    html.push_str("<div id='iface-filter' class='iface-filter'></div>");
    html.push_str("</section>");

    // セクション: 帯域利用率グラフ（SVG インライン）
    html.push_str("<section class='detail-section'>");
    html.push_str("<h2><span data-i18n='bandwidth_title'>Bandwidth Utilization (%)</span> <span class='chart-legend' id='bw-legend'></span></h2>");
    html.push_str("<div class='chart-wrap'><svg id='bw-chart' class='chart-svg' viewBox='0 0 800 160' preserveAspectRatio='none'></svg></div>");
    html.push_str("</section>");

    // セクション: エラーカウンタグラフ
    html.push_str("<section class='detail-section'>");
    html.push_str("<h2><span data-i18n='error_discard_title'>Error &amp; Discard Counters</span> <span class='chart-legend' id='err-legend'></span></h2>");
    html.push_str("<div class='chart-wrap'><svg id='err-chart' class='chart-svg' viewBox='0 0 800 160' preserveAspectRatio='none'></svg></div>");
    html.push_str("</section>");

    // セクション: CPUグラフ
    html.push_str("<section class='detail-section'>");
    html.push_str("<h2 data-i18n='cpu_usage_title'>CPU Usage (%)</h2>");
    html.push_str("<div class='chart-wrap'><svg id='cpu-chart' class='chart-svg' viewBox='0 0 800 160' preserveAspectRatio='none'></svg></div>");
    html.push_str("</section>");

    html.push_str("<section class='detail-section'>");
    html.push_str("<h2 data-i18n='memory_usage_title'>Memory Usage (%)</h2>");
    html.push_str("<div class='chart-wrap'><svg id='memory-chart' class='chart-svg' viewBox='0 0 800 160' preserveAspectRatio='none'></svg></div>");
    html.push_str("</section>");

    html.push_str("<section class='detail-section'>");
    html.push_str("<h2 data-i18n='hardware_status_title'>Hardware Status</h2>");
    html.push_str("<div id='hardware-status' class='hardware-status'>Loading...</div>");
    html.push_str("</section>");

    // セクション: インターフェース別スパイク
    html.push_str("<section class='detail-section'>");
    html.push_str("<h2 data-i18n='recent_spikes_title'>Recent Interface Spikes</h2>");
    html.push_str("<table id='spike-table'><thead><tr><th data-i18n='time'>Time</th><th data-i18n='interface'>Interface</th><th data-i18n='status'>Link</th><th data-i18n='in_errors'>In Errors</th><th data-i18n='out_errors'>Out Errors</th><th data-i18n='in_discards'>In Discards</th><th data-i18n='out_discards'>Out Discards</th><th>Total Delta</th></tr></thead>");
    html.push_str("<tbody id='spike-tbody'><tr><td colspan=8>Loading...</td></tr></tbody></table>");
    html.push_str("</section>");

    // セクション: アラート履歴
    html.push_str("<section class='detail-section'>");
    html.push_str("<h2 data-i18n='alert_history'>Alert History</h2>");
    html.push_str("<table id='alert-table'><thead><tr><th data-i18n='time'>Time</th><th data-i18n='interface'>Interface</th><th data-i18n='type'>Type</th><th data-i18n='severity'>Severity</th><th data-i18n='details'>Details</th></tr></thead>");
    html.push_str("<tbody id='alert-tbody'><tr><td colspan=5>Loading...</td></tr></tbody></table>");
    html.push_str("</section>");

    html.push_str("</main>");
    html.push_str(&format!(
        "<script>\nvar DEVICE_IP = '{}';\n",
        escape_json(ip)
    ));
    html.push_str("</script><script src='/static/js/i18n.js'></script><script src='/static/js/device-detail.js'></script></body></html>");
    html
}

fn page_discovery(
    repo: &Arc<Mutex<Repository>>,
    limits: WebLimits,
    extension: Option<&dyn WebExtensionProvider>,
) -> String {
    let existing_ips: Vec<String> = repo
        .lock()
        .ok()
        .and_then(|r| r.list_devices().ok())
        .unwrap_or_default()
        .into_iter()
        .map(|d| d.ip)
        .collect();

    let existing_json: Vec<String> = existing_ips
        .iter()
        .map(|ip| format!("\"{}\"", escape_json(ip)))
        .collect();

    let existing_set = format!("[{}]", existing_json.join(","));

    let mut html = String::new();
    html.push_str(HTML_DOCTYPE);
    html.push_str("<html lang='en'><head>");
    html.push_str("<meta charset='utf-8'>");
    html.push_str("<meta name='viewport' content='width=device-width,initial-scale=1'>");
    html.push_str("<script src='/static/js/theme.js'></script>");
    html.push_str("<title>TracePulse \u{2013} Discovery</title>");
    html.push_str(COMMON_CSS);
    html.push_str(DISCOVERY_CSS);
    html.push_str("</head><body data-title-key='discovery_title'>");
    html.push_str(&navigation_html(extension));
    html.push_str(DISCOVERY_HTML);
    html.push_str("<script>\nconst EXISTING = new Set(");
    html.push_str(&existing_set);
    html.push_str(");\nwindow.DISCOVERY_EXISTING = new Set(");
    html.push_str(&existing_set);
    html.push_str(");\n");
    html.push_str(&format!(
        "window.TRACEPULSE_DISCOVERY_LIMITS = {{maxCidrs:{},nodeLimit:{}}};\n",
        limits
            .max_discovery_cidrs
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "null".to_string()),
        limits
            .max_devices
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "null".to_string())
    ));
    html.push_str("</script><script src='/static/js/i18n.js'></script><script src='/static/js/topology.js'></script><script src='/static/js/discovery.js'></script></body></html>");
    html
}

fn page_diagnostics(
    path: &str,
    cfg: &Arc<Mutex<AppConfig>>,
    extension: Option<&dyn WebExtensionProvider>,
) -> String {
    use crate::snmp::SnmpClient;

    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    let params = parse_form(query);
    let default_community = cfg
        .lock()
        .map(|c| c.snmp.default_community.clone())
        .unwrap_or_else(|_| "public".to_string());
    let saved_cpu_oid = cfg
        .lock()
        .map(|c| c.snmp.cpu_oid_override.clone())
        .unwrap_or_else(|_| String::new());
    let saved_memory_oid = cfg
        .lock()
        .map(|c| c.snmp.memory_oid_override.clone())
        .unwrap_or_else(|_| String::new());
    let saved_hardware_oids = cfg
        .lock()
        .map(|c| c.snmp.hardware_oid_overrides.clone())
        .unwrap_or_default();
    let ip = params.get("ip").cloned().unwrap_or_default();
    let community = params
        .get("community")
        .cloned()
        .unwrap_or_else(|| default_community.clone());

    fn fmt_opt(value: Option<u32>) -> String {
        value
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string())
    }

    fn fmt_bytes_opt(value: Option<u64>) -> String {
        value.map(format_bytes).unwrap_or_else(|| "N/A".to_string())
    }

    fn render_probe_rows(probes: &[crate::snmp::SnmpOidProbe]) -> String {
        probes.iter().map(|probe| {
            let class = if probe.selected { " class='selected'" } else { "" };
            let status_class = match probe.status.as_str() {
                "ok" => "ok",
                "n/a" => "na",
                _ => "err",
            };
            format!(
                "<tr{class}><td>{}</td><td><code>{}</code></td><td>{}</td><td><span class='diag-pill {}'>{}</span></td></tr>",
                escape_html(&probe.label),
                escape_html(&probe.oid),
                fmt_opt(probe.value),
                status_class,
                escape_html(&probe.status),
            )
        }).collect()
    }

    fn render_memory_probe_rows(probes: &[crate::snmp::SnmpMemoryProbe]) -> String {
        if probes.is_empty() {
            return "<tr><td colspan='4' class='diag-empty'>No memory candidates</td></tr>"
                .to_string();
        }
        probes.iter().map(|probe| {
            let class = if probe.selected { " class='selected'" } else { "" };
            let status_class = match probe.status.as_str() {
                "ok" => "ok",
                "n/a" => "na",
                _ => "err",
            };
            format!(
                "<tr{class}><td>{}</td><td><code>{}</code></td><td>{}</td><td><span class='diag-pill {}'>{}</span></td></tr>",
                escape_html(&probe.label),
                escape_html(&probe.oid),
                fmt_bytes_opt(probe.value),
                status_class,
                escape_html(&probe.status),
            )
        }).collect()
    }

    fn render_hardware_sensors(sensors: &[crate::snmp::SnmpHardwareSensor]) -> String {
        if sensors.is_empty() {
            return "<tr><td colspan='6' class='diag-empty'>No Sensors Detected</td></tr>"
                .to_string();
        }
        sensors.iter().map(|sensor| {
            let status = sensor.status.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string());
            let value = sensor.value.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string());
            let alarm = if sensor.is_alarm { "diag-pill err" } else { "diag-pill ok" };
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td><span class='{}'>{}</span></td></tr>",
                escape_html(&sensor.sensor_type),
                escape_html(&sensor.source),
                escape_html(&sensor.name),
                escape_html(&sensor.oid),
                escape_html(&value),
                alarm,
                escape_html(&status),
            )
        }).collect()
    }

    fn render_hardware_probes(probes: &[crate::snmp::SnmpOidProbe]) -> String {
        if probes.is_empty() {
            return "<tr><td colspan='4' class='diag-empty'>No hardware candidates</td></tr>"
                .to_string();
        }
        probes.iter().map(|probe| {
            let class = if probe.selected { " class='selected'" } else { "" };
            let status_class = match probe.status.as_str() {
                "ok" => "ok",
                "n/a" => "na",
                _ => "err",
            };
            let value = probe.value.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string());
            format!(
                "<tr{class}><td>{}</td><td><code>{}</code></td><td>{}</td><td><span class='diag-pill {}'>{}</span></td></tr>",
                escape_html(&probe.label),
                escape_html(&probe.oid),
                escape_html(&value),
                status_class,
                escape_html(&probe.status),
            )
        }).collect()
    }

    let mut result_html = String::new();
    let mut attempted = false;

    if !ip.trim().is_empty() {
        attempted = true;
        let client = {
            let overrides = cfg.lock().ok().map(|c| c.snmp.clone()).unwrap_or_default();
            SnmpClient::with_snmp_config(community.clone(), overrides)
        };
        let device = DeviceConfig::new(ip.clone(), community.clone());
        match client.diagnose_device(&device) {
            Ok(report) => {
                let cpu_value = fmt_opt(report.cpu_usage);
                let memory_bytes_value = fmt_bytes_opt(report.memory_used_bytes);
                let vendor = report
                    .vendor_name
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());
                let enterprise = report
                    .enterprise_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                let sys_object_id = report
                    .sys_object_id
                    .clone()
                    .unwrap_or_else(|| "N/A".to_string());

                result_html.push_str("<section class='settings-section'>");
                result_html.push_str("<h2 data-i18n='diagnostics_title'>SNMP Diagnostics</h2>");
                result_html.push_str("<div class='diag-grid'>");
                result_html.push_str("<div class='diag-card'>");
                result_html.push_str("<h3 data-i18n='target_device'>Target Device</h3>");
                result_html.push_str("<dl class='diag-kv'>");
                result_html.push_str(&format!(
                    "<dt data-i18n='ip_address'>IP</dt><dd>{}</dd>",
                    escape_html(&ip)
                ));
                result_html.push_str(&format!(
                    "<dt data-i18n='hostname'>sysName</dt><dd>{}</dd>",
                    escape_html(&report.sys_name)
                ));
                result_html.push_str(&format!(
                    "<dt>sysDescr</dt><dd>{}</dd>",
                    escape_html(&report.sys_descr)
                ));
                result_html.push_str(&format!(
                    "<dt data-i18n='sys_object_id'>sysObjectID</dt><dd><code>{}</code></dd>",
                    escape_html(&sys_object_id)
                ));
                result_html.push_str(&format!(
                    "<dt data-i18n='enterprise_id'>Enterprise ID</dt><dd>{}</dd>",
                    escape_html(&enterprise)
                ));
                result_html.push_str(&format!(
                    "<dt data-i18n='vendor'>Vendor</dt><dd>{}</dd>",
                    escape_html(&vendor)
                ));
                result_html.push_str("</dl>");
                result_html.push_str("</div>");
                result_html.push_str("<div class='diag-card'>");
                result_html.push_str("<h3>Summary</h3>");
                result_html.push_str("<dl class='diag-kv'>");
                result_html.push_str(&format!(
                    "<dt data-i18n='cpu_usage_title'>CPU Usage (%)</dt><dd>{}</dd>",
                    cpu_value
                ));
                result_html.push_str(&format!(
                    "<dt data-i18n='memory_used_title'>Memory Used</dt><dd>{}</dd>",
                    memory_bytes_value
                ));
                result_html.push_str(&format!(
                    "<dt data-i18n='if_index_list'>ifIndex Count</dt><dd>{}</dd>",
                    report.interfaces.len()
                ));
                result_html.push_str("</dl>");
                result_html.push_str("</div>");
                result_html.push_str("</div>");

                result_html.push_str("<div class='diag-card' style='margin-top:1rem'>");
                result_html.push_str("<h3 data-i18n='cpu_candidates'>CPU Candidates</h3>");
                result_html.push_str("<table class='probe-table'><thead><tr><th data-i18n='probe'>Probe</th><th>OID</th><th data-i18n='value'>Value</th><th data-i18n='status'>Status</th></tr></thead><tbody>");
                result_html.push_str(&render_probe_rows(&report.cpu_probes));
                result_html.push_str("</tbody></table>");
                result_html.push_str("</div>");

                result_html.push_str("<div class='diag-card' style='margin-top:1rem'>");
                result_html.push_str("<h3 data-i18n='memory_candidates'>Memory Candidates</h3>");
                result_html.push_str("<table class='probe-table'><thead><tr><th data-i18n='probe'>Probe</th><th>OID</th><th data-i18n='value'>Value</th><th data-i18n='status'>Status</th></tr></thead><tbody>");
                result_html.push_str(&render_memory_probe_rows(&report.memory_byte_probes));
                result_html.push_str("</tbody></table>");
                result_html.push_str("</div>");

                result_html.push_str("<div class='diag-card' style='margin-top:1rem'>");
                result_html
                    .push_str("<h3 data-i18n='hardware_candidates'>Hardware Candidates</h3>");
                result_html.push_str("<table class='probe-table'><thead><tr><th>Probe</th><th>OID</th><th>Value</th><th>Status</th></tr></thead><tbody>");
                result_html.push_str(&render_hardware_probes(&report.hardware_probes));
                result_html.push_str("</tbody></table>");
                result_html.push_str("</div>");

                result_html.push_str("<div class='diag-card' style='margin-top:1rem'>");
                result_html.push_str("<h3 data-i18n='hardware_sensors'>Hardware Sensors</h3>");
                result_html.push_str("<table class='probe-table'><thead><tr><th>Type</th><th>Source</th><th>Name</th><th>OID</th><th>Value</th><th>Status</th></tr></thead><tbody>");
                result_html.push_str(&render_hardware_sensors(&report.hardware_sensors));
                result_html.push_str("</tbody></table>");
                result_html.push_str("</div>");
                result_html.push_str("</section>");
            }
            Err(err) => {
                result_html.push_str("<section class='settings-section'>");
                result_html.push_str("<p class='diag-error'>");
                result_html.push_str(&escape_html(&err.to_string()));
                result_html.push_str("</p></section>");
            }
        }
    }

    if !attempted {
        result_html.push_str("<section class='settings-section'><p class='diag-note' data-i18n='diagnostics_help'>Check sysObjectID, vendor presets, CPU candidates, and ifIndex mappings.</p></section>");
    }

    let mut html = String::new();
    html.push_str(HTML_DOCTYPE);
    html.push_str("<html lang='en'><head>");
    html.push_str("<meta charset='utf-8'>");
    html.push_str("<meta name='viewport' content='width=device-width,initial-scale=1'>");
    html.push_str("<script src='/static/js/theme.js'></script>");
    html.push_str("<title>TracePulse \u{2013} Diagnostics</title>");
    html.push_str(COMMON_CSS);
    html.push_str(SETTINGS_CSS);
    html.push_str(DIAGNOSTICS_CSS);
    html.push_str("</head><body data-title-key='diagnostics_title'>");
    html.push_str(&navigation_html(extension));
    html.push_str("<main>");
    html.push_str("<h1 data-i18n='diagnostics_title'>SNMP Diagnostics</h1>");
    html.push_str("<div class='settings-section'>");
    html.push_str("<h2 data-i18n='target_device'>Target Device</h2>");
    html.push_str(
        "<form method='GET' action='/diagnostics' onsubmit='return startDiagnostics(event)'>",
    );
    html.push_str("<div class='field-row'>");
    html.push_str("<div class='field-group'>");
    html.push_str("<label data-i18n='ip_address'>IP Address</label>");
    html.push_str(&format!(
        "<input type='text' name='ip' value='{}' placeholder='192.168.1.1'>",
        escape_html(&ip)
    ));
    html.push_str("</div>");
    html.push_str("<div class='field-group'>");
    html.push_str("<label data-i18n='community'>Community</label>");
    html.push_str(&format!(
        "<input type='text' name='community' value='{}' placeholder='public'>",
        escape_html(&community)
    ));
    html.push_str("</div>");
    html.push_str("</div>");
    html.push_str("<div class='actions'>");
    html.push_str("<button class='btn btn-primary' id='diag-run-btn' type='submit' data-i18n='run_diagnostics'>Run Diagnostics</button>");
    html.push_str("<span id='diag-progress' class='diag-progress' aria-live='polite'></span>");
    html.push_str("</div>");
    html.push_str("</form>");
    html.push_str("<p class='diag-note' data-i18n='diagnostics_help'>Check sysObjectID, vendor presets, CPU candidates, and ifIndex mappings.</p>");
    html.push_str("</div>");
    html.push_str("<div class='settings-section'>");
    html.push_str("<h2 data-i18n='oid_overrides'>OID Overrides</h2>");
    html.push_str("<div class='field-row'>");
    html.push_str("<div class='field-group'>");
    html.push_str("<label data-i18n='cpu_oid_override'>CPU OID</label>");
    html.push_str(&format!(
        "<input type='text' id='cpu-oid-override' value='{}' placeholder='1.3.6.1.4.1....'>",
        escape_html(&saved_cpu_oid)
    ));
    html.push_str("</div>");
    html.push_str("<div class='field-group'>");
    html.push_str("<label data-i18n='memory_oid_override'>Memory OID</label>");
    html.push_str(&format!(
        "<input type='text' id='memory-oid-override' value='{}' placeholder='1.3.6.1.4.1....'>",
        escape_html(&saved_memory_oid)
    ));
    html.push_str("</div>");
    html.push_str("</div>");
    html.push_str("<div class='hardware-oid-fields'>");
    fn hardware_oid_field_html(
        sensor_type: &str,
        label_key: &str,
        label_text: &str,
        oid: &str,
    ) -> String {
        format!(
            "<div class='field-group'><label data-i18n='{}'>{}</label><input type='text' class='hardware-oid-input' data-sensor-type='{}' value='{}' placeholder='1.3.6.1.4.1....'></div>",
            label_key,
            label_text,
            sensor_type,
            escape_html(oid),
        )
    }
    let hardware_oid_map: std::collections::HashMap<&str, &str> = saved_hardware_oids
        .iter()
        .filter_map(|entry| entry.split_once('|'))
        .collect();
    html.push_str(&hardware_oid_field_html(
        "temperature",
        "hardware_oid_temperature",
        "Temperature OID",
        hardware_oid_map.get("temperature").copied().unwrap_or(""),
    ));
    html.push_str(&hardware_oid_field_html(
        "power",
        "hardware_oid_power",
        "Power OID",
        hardware_oid_map.get("power").copied().unwrap_or(""),
    ));
    html.push_str(&hardware_oid_field_html(
        "fan",
        "hardware_oid_fan",
        "Fan OID",
        hardware_oid_map.get("fan").copied().unwrap_or(""),
    ));
    html.push_str("</div>");
    html.push_str("<div class='actions'>");
    html.push_str("<button class='btn btn-primary' type='button' onclick='saveOidOverrides()' data-i18n='save_oid_overrides'>OID の手動保存</button>");
    html.push_str("<span id='oid-save-result' class='diag-note'></span>");
    html.push_str("</div>");
    html.push_str("</div>");
    html.push_str(&result_html);
    html.push_str("</main>");
    html.push_str("<script>\n");
    html.push_str("</script><script src='/static/js/i18n.js'></script><script src='/static/js/diagnostics.js'></script></body></html>");
    html
}

// ─── Shared HTML components ───────────────────────────────────────────────────

const HTML_DOCTYPE: &str = "<!doctype html>";
const COMMON_CSS: &str = "<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  :root { color-scheme: dark; }
  html[data-theme='light'] { color-scheme: light; }
  body { font-family: system-ui, sans-serif; background: #0f172a; color: #f1f5f9; min-height: 100vh; }
  html[data-theme='light'] body { background: #f8fafc; color: #0f172a; }
  nav { background: #1e293b; border-bottom: 1px solid #334155; padding: .75rem 1.5rem; display: flex; align-items: center; gap: 1.5rem; }
  html[data-theme='light'] nav { background: #ffffff; border-bottom-color: #cbd5e1; }
  nav .brand { font-weight: 700; font-size: 1.1rem; color: #38bdf8; text-decoration: none; }
  html[data-theme='light'] nav .brand { color: #0284c7; }
  nav a { color: #94a3b8; text-decoration: none; font-size: .9rem; }
  html[data-theme='light'] nav a { color: #475569; }
  nav a:hover { color: #f1f5f9; }
  html[data-theme='light'] nav a:hover { color: #0f172a; }
  nav .nav-spacer { flex: 1; }
  nav .lang-wrap { display:flex; align-items:center; gap:.5rem; color:#94a3b8; font-size:.85rem; }
  html[data-theme='light'] nav .lang-wrap { color:#475569; }
  nav .theme-wrap { display:flex; align-items:center; gap:.5rem; color:#94a3b8; font-size:.85rem; }
  html[data-theme='light'] nav .theme-wrap { color:#475569; }
  nav .lang-select, nav .theme-select { background:#0f172a; color:#f1f5f9; border:1px solid #334155; border-radius:.375rem; padding:.35rem .5rem; font-size:.85rem; }
  html[data-theme='light'] nav .lang-select, html[data-theme='light'] nav .theme-select { background:#fff; color:#0f172a; border-color:#cbd5e1; }
  nav .lang-select:focus, nav .theme-select:focus { outline:none; border-color:#3b82f6; }
  main { max-width: 1100px; margin: 2rem auto; padding: 0 1.5rem; }
  h1 { font-size: 1.5rem; margin-bottom: 1.5rem; color: #e2e8f0; }
  html[data-theme='light'] h1 { color:#0f172a; }
  h2 { font-size: 1.15rem; color: #cbd5e1; }
  html[data-theme='light'] h2 { color:#0f172a; }
  h3 { font-size: 1rem; color: #cbd5e1; }
  html[data-theme='light'] h3 { color:#0f172a; }
  .summary-cards { display: flex; gap: 1rem; margin-bottom: 1.5rem; flex-wrap: wrap; }
  .card { background: #1e293b; border: 1px solid #334155; border-radius: .5rem; padding: 1rem 1.5rem; min-width: 120px; text-align: center; }
  html[data-theme='light'] .card { background:#fff; border-color:#cbd5e1; }
  .card-num { font-size: 2rem; font-weight: 700; }
  .card-label { font-size: .8rem; color: #94a3b8; margin-top: .25rem; }
  html[data-theme='light'] .card-label { color:#64748b; }
  .card-online .card-num { color: #4ade80; }
  .card-warning .card-num { color: #fbbf24; }
  .card-offline .card-num { color: #f87171; }
  table { width: 100%; border-collapse: collapse; background: #1e293b; border: 1px solid #334155; border-radius: .5rem; overflow: hidden; }
  html[data-theme='light'] table { background:#fff; border-color:#cbd5e1; }
  th, td { padding: .7rem 1rem; text-align: left; font-size: .9rem; }
  th { background: #0f172a; color: #94a3b8; font-weight: 600; font-size: .8rem; text-transform: uppercase; letter-spacing: .05em; border-bottom: 1px solid #334155; }
  html[data-theme='light'] th { background:#e2e8f0; color:#334155; border-bottom-color:#cbd5e1; }
  tr:not(:last-child) td { border-bottom: 1px solid #1e293b; }
  html[data-theme='light'] tr:not(:last-child) td { border-bottom-color:#e2e8f0; }
  tr:hover td { background: #263044; }
  html[data-theme='light'] tr:hover td { background:#f1f5f9; }
  .empty { text-align: center; color: #64748b; padding: 2rem; }
  .empty a { color: #38bdf8; }
  html[data-theme='light'] .empty { color:#64748b; }
  .status-online { color: #4ade80; }
  .status-offline { color: #f87171; }
  .status-warning { color: #fbbf24; }
  .status-critical { color: #f87171; font-weight: 700; }
  .status-unknown { color: #94a3b8; }
  .btn { background: #3b82f6; color: #fff; border: none; padding: .55rem 1.1rem; border-radius: .375rem; font-size: .9rem; font-weight: 600; cursor: pointer; white-space: nowrap; }
  html[data-theme='light'] .btn { background:#2563eb; }
  .btn:hover:not(:disabled) { background: #2563eb; }
  .btn:disabled { opacity: .6; cursor: not-allowed; }
  .btn-secondary { background: #334155; }
  html[data-theme='light'] .btn-secondary { background:#e2e8f0; color:#0f172a; }
  .btn-secondary:hover:not(:disabled) { background: #475569; }
  html[data-theme='light'] .btn-secondary:hover:not(:disabled) { background:#cbd5e1; }
  a { color: #38bdf8; }
  html[data-theme='light'] a { color:#0284c7; }
  html[data-theme='light'] .detail-section,
  html[data-theme='light'] .settings-section,
  html[data-theme='light'] .manual-box,
  html[data-theme='light'] #scan-progress,
  html[data-theme='light'] #register-result {
    background:#fff;
    border-color:#cbd5e1;
    color:#0f172a;
  }
  html[data-theme='light'] .detail-section h2,
  html[data-theme='light'] .settings-section h2 {
    color:#0f172a;
    border-bottom-color:#e2e8f0;
  }
  html[data-theme='light'] .field-group input,
  html[data-theme='light'] .field-group select {
    background:#fff;
    border-color:#cbd5e1;
    color:#0f172a;
  }
  html[data-theme='light'] .chart-svg { background:#fff; border:1px solid #cbd5e1; }
  html[data-theme='light'] .chart-svg.chart-empty { opacity:.55; }
  html[data-theme='light'] .chart-legend { color:#64748b; }
  html[data-theme='light'] .progress-label,
  html[data-theme='light'] .progress-elapsed,
  html[data-theme='light'] .host-hint,
  html[data-theme='light'] .field-hint,
  html[data-theme='light'] .card-label { color:#64748b; }
  html[data-theme='light'] .spike-row { background:rgba(245,158,11,.12); }
  html[data-theme='light'] .row-warn td { background: rgba(245, 158, 11, .10); }
  html[data-theme='light'] .row-crit td { background: rgba(248, 113, 113, .12); }
  html[data-theme='light'] .counter-total { color:#0f172a; }
  html[data-theme='light'] .counter-delta.none { color:#64748b; }
  html[data-theme='light'] .counter-delta.warn { color:#b45309; }
  html[data-theme='light'] .counter-delta.crit { color:#b91c1c; }
  html[data-theme='light'] .counter-delta.duplex { color:#6d28d9; }
</style>";

fn navigation_html(extension: Option<&dyn WebExtensionProvider>) -> String {
    let mut navigation = NAV_HTML.to_string();
    if let Some(extension) = extension {
        navigation = navigation.replacen(
            "<a href='/discovery' data-i18n='nav_discovery'>Discovery</a>",
            &format!(
                "{}<a href='/discovery' data-i18n='nav_discovery'>Discovery</a>",
                extension.navigation_html()
            ),
            1,
        );
    }
    navigation
}

const NAV_HTML: &str = "<nav>
  <a class='brand' href='/'>TracePulse</a>
    <a href='/' data-i18n='nav_dashboard'>Dashboard</a>
    <a href='/discovery' data-i18n='nav_discovery'>Discovery</a>
  <a href='/diagnostics' data-i18n='nav_diagnostics'>Diagnostics</a>
  <a href='/settings' data-i18n='nav_settings'>Settings</a>
  <div class='nav-spacer'></div>
  <div class='theme-wrap'>
    <span data-i18n='theme_label'>Theme</span>
    <select id='theme-select' class='theme-select' onchange='setTheme(this.value)'>
      <option value='dark' data-i18n='theme_dark'>Dark</option>
      <option value='light' data-i18n='theme_light'>Light</option>
    </select>
  </div>
  <div class='lang-wrap'>
    <span data-i18n='language_label'>Language</span>
    <select id='lang-select' class='lang-select' onchange='setLanguage(this.value)'>
      <option value='en' data-i18n='english'>English</option>
      <option value='ja' data-i18n='japanese'>日本語</option>
    </select>
  </div>
</nav>";

const SETTINGS_CSS: &str = "<style>
  .settings-section { background:#1e293b; border:1px solid #334155; border-radius:.5rem; padding:1.5rem; margin-bottom:1.5rem; }
  .settings-section h2 { margin-bottom:1rem; font-size:1rem; color:#38bdf8; border-bottom:1px solid #334155; padding-bottom:.5rem; }
  .field-row { display:grid; grid-template-columns:1fr 1fr; gap:1rem; margin-bottom:1rem; }
  .hardware-oid-fields { display:grid; grid-template-columns:1fr 1fr 1fr; gap:1rem; margin-top:.75rem; margin-bottom:1rem; }
  .field-group { display:flex; flex-direction:column; gap:.3rem; }
  .field-group label { font-size:.82rem; color:#94a3b8; }
  .field-group input, .field-group select { background:#0f172a; border:1px solid #334155; color:#f1f5f9; padding:.45rem .7rem; border-radius:.375rem; font-size:.95rem; }
  .field-group input:focus, .field-group select:focus { outline:none; border-color:#3b82f6; }
  .field-hint { font-size:.75rem; color:#64748b; margin-top:.15rem; }
  .score-bar { display:flex; align-items:center; gap:.75rem; margin-top:.5rem; }
  .score-bar span { font-size:.82rem; color:#94a3b8; min-width:6rem; }
  .threshold-visual { flex:1; height:.5rem; border-radius:.25rem; background:linear-gradient(to right, #ef4444 0%, #ef4444 var(--crit), #f59e0b var(--crit), #f59e0b var(--warn), #4ade80 var(--warn), #4ade80 100%); }
  .actions { display:flex; align-items:center; gap:1rem; margin-top:1rem; }
  .toast { padding:.5rem 1rem; border-radius:.375rem; font-size:.9rem; display:none; }
  .toast-ok { background:#14532d; color:#4ade80; border:1px solid #166534; }
  .toast-err { background:#450a0a; color:#f87171; border:1px solid #7f1d1d; }
</style>";

const SETTINGS_HTML: &str = "<main>
<h1 data-i18n='settings_title'>Settings</h1>

<div class='settings-section'>
  <h2 data-i18n='polling'>Polling</h2>
  <div class='field-row'>
    <div class='field-group'>
      <label data-i18n='polling_interval_label'>Polling Interval (seconds)</label>
      <input type='number' id='interval' min='5' max='600' step='5'>
      <div class='field-hint' data-i18n='polling_interval_hint'>Range: 5 – 600 seconds (default: 30)</div>
    </div>
  </div>
</div>

<div class='settings-section'>
  <h2 data-i18n='snmp'>SNMP</h2>
  <div class='field-row'>
    <div class='field-group'>
      <label data-i18n='default_community'>Default Community String</label>
      <input type='text' id='community' placeholder='public' data-i18n-placeholder='public_placeholder'>
      <div class='field-hint' data-i18n='default_community_hint'>Used when no community is specified per device</div>
    </div>
  </div>
</div>

<div class='settings-section'>
  <h2 data-i18n='display'>Display</h2>
  <div class='field-row'>
  <div class='field-group'>
    <label data-i18n='time_zone'>Time Zone</label>
    <select id='timezone'>
      <option value='utc' data-i18n='timezone_utc'>UTC</option>
      <option value='jst' data-i18n='timezone_jst'>JST</option>
    </select>
    <div class='field-hint' data-i18n='time_zone_hint'>Controls how timestamps are shown in the web UI</div>
  </div>
  </div>
</div>

<div class='settings-section'>
  <h2 data-i18n='alert_thresholds'>Alert Thresholds</h2>
  <div class='field-row'>
    <div class='field-group'>
      <label data-i18n='error_rate_threshold'>Error Rate Threshold (0.0 – 1.0)</label>
      <input type='number' id='error_rate' min='0' max='1' step='0.01'>
      <div class='field-hint' data-i18n='error_rate_hint'>e.g. 0.05 = alert when error rate exceeds 5%</div>
    </div>
    <div class='field-group'>
      <label data-i18n='spike_threshold'>Spike Threshold (error count delta)</label>
      <input type='number' id='spike' min='1' step='1'>
      <div class='field-hint' data-i18n='spike_threshold_hint'>Alert when error count jumps by this amount between polls</div>
    </div>
  </div>
  <div class='field-row'>
    <div class='field-group'>
      <label data-i18n='warning_threshold'>Health Score — Warning threshold (0 – 100)</label>
      <input type='number' id='warn_t' min='1' max='100' step='1' oninput='updateBar()'>
      <div class='field-hint' data-i18n='warning_threshold_hint'>Score below this value turns status yellow (Warning)</div>
    </div>
    <div class='field-group'>
      <label data-i18n='critical_threshold'>Health Score — Critical threshold (0 – 100)</label>
      <input type='number' id='crit_t' min='0' max='99' step='1' oninput='updateBar()'>
      <div class='field-hint' data-i18n='critical_threshold_hint'>Score below this value turns status red (Critical)</div>
    </div>
  </div>
  <div class='score-bar'>
    <span data-i18n='score_preview'>Score preview</span>
    <div class='threshold-visual' id='tbar' style='--warn:80%;--crit:60%'></div>
    <span id='bar-label' style='font-size:.78rem;color:#64748b'></span>
  </div>
  <div class='field-hint' style='margin-top:.5rem'>
    <span data-i18n='score_formula'>Health score (0–100) = 100 − error_rate×1.5 − bandwidth×0.3 − cpu/3 − memory/3</span><br>
    <span style='color:#4ade80'>■</span> ≥ Warning: Online &nbsp;
    <span style='color:#f59e0b'>■</span> ≥ Critical: Warning &nbsp;
    <span style='color:#ef4444'>■</span> &lt; Critical: Critical
  </div>
</div>

<div class='settings-section'>
  <h2 data-i18n='data_retention'>Data Retention</h2>
  <div class='field-row'>
    <div class='field-group'>
      <label data-i18n='history_retention'>History Retention (days)</label>
      <input type='number' id='days' min='1' step='1'>
      <div class='field-hint' data-i18n='history_retention_hint'>Polling history older than this is purged automatically</div>
    </div>
  </div>
</div>

</main>";

const SETTINGS_ACTIONS_HTML: &str = "<div class='actions'>
    <button class='btn btn-primary' onclick='saveSettings()' data-i18n='save_settings'>Save Settings</button>
    <button class='btn' onclick='resetDefaults()' style='background:#334155' data-i18n='reset_defaults'>Reset to Defaults</button>
    <span class='toast toast-ok' id='toast-ok' data-i18n='settings_saved'>&#10003; Settings saved</span>
    <span class='toast toast-err' id='toast-err'></span>
</div>";

const NOTIFICATIONS_HTML: &str = "<section class='settings-section'><h2 data-i18n='notifications'>Alert Notifications (Slack / Teams)</h2><div class='field-row'><div class='field-group'><label><input type='checkbox' id='slack_enabled'> <span data-i18n='slack_enabled'>Enable Slack notifications</span></label><input type='text' id='slack_url' placeholder='https://hooks.slack.com/services/...'><input type='text' id='slack_url_env' placeholder='TRACEPULSE_SLACK_WEBHOOK_URL'></div><div class='field-group'><label><input type='checkbox' id='teams_enabled'> <span data-i18n='teams_enabled'>Enable Teams notifications</span></label><input type='text' id='teams_url' placeholder='https://outlook.office.com/webhook/...'><input type='text' id='teams_url_env' placeholder='TRACEPULSE_TEAMS_WEBHOOK_URL'></div></div><div class='field-row'><div class='field-group'><label data-i18n='flap_window'>Flap Guard Window (seconds)</label><input type='number' id='flap_window' min='0' max='3600'></div><div class='field-group'><label data-i18n='retry_max_attempts'>Retry Attempts</label><input type='number' id='retry_attempts' min='1' max='10'></div></div><div class='actions'><button class='btn btn-primary' onclick='saveNotifications()' data-i18n='save_notifications'>Save Notification Settings</button><button class='btn' onclick=\"testNotification('all')\" data-i18n='send_test_all'>Test enabled channels</button><span class='toast toast-ok' id='ntoast-ok'></span><span class='toast toast-err' id='ntoast-err'></span></div></section>";

const DIAGNOSTICS_CSS: &str = "<style>
  .diag-grid { display:grid; grid-template-columns:1fr 1fr; gap:1rem; }
  .diag-card { background:#0f172a; border:1px solid #334155; border-radius:.5rem; padding:1rem; }
  .diag-card h3 { margin-bottom:.5rem; font-size:.9rem; color:#38bdf8; }
  .diag-kv { display:grid; grid-template-columns: 10rem 1fr; gap:.35rem .75rem; font-size:.9rem; }
  .diag-kv dt { color:#94a3b8; }
  .diag-kv dd { color:#f1f5f9; word-break:break-word; }
  .probe-table { width:100%; border-collapse:collapse; margin-top:.75rem; }
  .probe-table th, .probe-table td { padding:.55rem .7rem; border-bottom:1px solid #334155; font-size:.88rem; }
  .probe-table th { text-align:left; color:#94a3b8; background:#0f172a; }
  .probe-table tr.selected td { background:rgba(74,222,128,.08); }
  .diag-pill { display:inline-flex; align-items:center; padding:.15rem .45rem; border-radius:999px; font-size:.75rem; font-weight:700; }
  .diag-pill.ok { background:#14532d; color:#4ade80; }
  .diag-pill.na { background:#334155; color:#cbd5e1; }
  .diag-pill.err { background:#450a0a; color:#f87171; }
  .diag-note { color:#94a3b8; font-size:.9rem; line-height:1.5; }
  .diag-progress { margin-left:.75rem; color:#38bdf8; font-size:.9rem; font-weight:600; }
  .diag-error { color:#f87171; font-weight:600; }
  .diag-empty { color:#64748b; font-style:italic; }
  html[data-theme='light'] .diag-card { background:#fff; border-color:#cbd5e1; }
  html[data-theme='light'] .diag-kv dt { color:#64748b; }
  html[data-theme='light'] .diag-kv dd { color:#0f172a; }
  html[data-theme='light'] .probe-table th, html[data-theme='light'] .probe-table td { border-bottom-color:#e2e8f0; }
  html[data-theme='light'] .probe-table th { background:#fff; color:#64748b; }
  html[data-theme='light'] .probe-table tr.selected td { background:rgba(74,222,128,.12); }
</style>";

const DEVICE_DETAIL_CSS: &str = "<style>
  /* このページはテーブル・グラフの情報量が多いため、共通レイアウトより幅広く画面幅に追従させる */
  body[data-title-key='device_title_prefix'] main { max-width: min(1800px, 96vw); width: 100%; }
  .detail-section { background:#1e293b; border:1px solid #334155; border-radius:.5rem; padding:1.25rem 1.5rem; margin-bottom:1.5rem; }
  .detail-section h2 { font-size:.95rem; color:#38bdf8; margin-bottom:.85rem; border-bottom:1px solid #334155; padding-bottom:.4rem; }
  .hardware-status { display:flex; flex-wrap:wrap; gap:.75rem; }
  .hardware-card { min-width:190px; background:#0f172a; border:1px solid #334155; border-radius:.5rem; padding:.75rem; }
  .hardware-card.warn { border-color:#d97706; }
  .hardware-card.crit { border-color:#dc2626; }
  .hardware-card .label { font-size:.78rem; color:#94a3b8; margin-bottom:.25rem; }
  .hardware-card .value { font-size:.95rem; color:#f8fafc; }
  .hardware-card .state { font-size:.78rem; margin-top:.35rem; color:#cbd5e1; }
  .chart-wrap { width:100%; overflow-x:auto; }
  .table-scroll { width:100%; overflow-x:auto; }
    .interface-table-scroll { overflow:visible; }
  .chart-svg { width:100%; height:160px; background:#0f172a; border-radius:.375rem; display:block; }
  .chart-svg.chart-empty { opacity:.5; }
  .chart-legend { font-size:.75rem; font-weight:400; color:#64748b; margin-left:.5rem; }
  .iface-filter { display:flex; flex-wrap:wrap; gap:.45rem .85rem; margin:0 0 1rem; padding:.75rem .85rem; border:1px solid #334155; border-radius:.45rem; background:#0f172a; }
  .iface-filter-item { display:inline-flex; align-items:center; gap:.35rem; font-size:.84rem; color:#cbd5e1; white-space:nowrap; }
  .iface-filter-item input { accent-color:#38bdf8; }
  .iface-filter-actions { display:flex; gap:.5rem; align-items:center; margin:0 0 1rem; flex-wrap:wrap; }
  .iface-filter-note { font-size:.78rem; color:#64748b; }
  #if-table th, #alert-table th { cursor:default; white-space:nowrap; }
  #if-table { min-width: 980px; table-layout: auto; }
  #if-table th, #if-table td { white-space: normal; word-break: break-word; }
  #if-table th { line-height: 1.15; font-size: .78rem; }
  #if-table td { vertical-align: top; }
  .counter-cell { font-variant-numeric: tabular-nums; }
  .counter-total { color:#f1f5f9; }
  .counter-delta { margin-left:.35rem; font-size:.82rem; font-weight:700; }
  .counter-delta.none { color:#64748b; }
  .counter-delta.warn { color:#fbbf24; }
  .counter-delta.crit { color:#f87171; }
  .counter-delta.duplex { color:#a78bfa; }
  .diagnostic-badge { display:inline-block; padding:.2rem .55rem; border-radius:9999px; font-size:.76rem; font-weight:700; white-space:nowrap; }
  .diagnostic-badge.healthy { background:rgba(16,185,129,.15); color:#10b981; }
  .diagnostic-badge.l1_error { background:rgba(239,68,68,.15); color:#ef4444; }
  .diagnostic-badge.duplex_mismatch { background:rgba(139,92,246,.18); color:#8b5cf6; }
  .diagnostic-badge.congestion { background:rgba(245,158,11,.15); color:#f59e0b; }
  .diagnostic-badge.down { background:rgba(107,114,128,.2); color:#9ca3af; }
  .row-warn td { background: rgba(245, 158, 11, .08); }
  .row-crit td { background: rgba(248, 113, 113, .10); }
  .pred-badge { display:inline-block; margin-left:.3rem; padding:.12rem .4rem; border-radius:999px; background:rgba(251,191,36,.16); color:#fbbf24; font-size:.72rem; font-weight:700; }
  .dom-meter { display:inline-flex; align-items:center; gap:.35rem; min-width:7rem; }
  .dom-meter-track { width:4rem; height:.42rem; border-radius:999px; background:#334155; overflow:hidden; }
  .dom-meter-fill { height:100%; background:#fbbf24; }
  .dom-meter-fill.ok { background:#4ade80; }
  .link-up { color:#4ade80; font-weight:600; }
  .link-down { color:#f87171; font-weight:600; }
  .sev-warning { color:#f59e0b; }
  .sev-critical { color:#f87171; }
  .sev-info { color:#64748b; }
  .no-data { color:#475569; font-style:italic; padding:.5rem 0; }
  #dev-title { font-size:1.4rem; }
  #spike-table th { white-space:nowrap; }
  /* 破損パケット内訳（EtherLike-MIB Error Breakdown）: ホバーポップオーバー */
  .err-cell { position:relative; cursor:help; border-bottom:1px dotted #64748b; }
  .err-pop { display:none; position:absolute; left:0; top:100%; margin-top:.35rem; min-width:220px; z-index:20; background:#0f172a; border:1px solid #334155; border-radius:.4rem; padding:.55rem .7rem; font-size:.78rem; color:#cbd5e1; box-shadow:0 8px 20px rgba(0,0,0,.35); white-space:normal; }
    .err-cell.flip .err-pop { top:auto; bottom:100%; margin-top:0; margin-bottom:.35rem; }
  .err-cell:hover .err-pop, .err-cell:focus .err-pop { display:block; }
  .err-pop-title { font-weight:700; color:#f1f5f9; margin-bottom:.3rem; }
  .err-pop-row { display:flex; justify-content:space-between; gap:.75rem; padding:.1rem 0; }
  .err-pop-row .n { color:#94a3b8; }
  .err-pop-row .v { font-variant-numeric:tabular-nums; }
    .err-pop-divider { border-top:1px solid #334155; margin:.35rem 0; }
  /* 破損パケット内訳カード: セレクタ + Stacked Bar + 数値テーブル */
  .error-breakdown { display:flex; flex-direction:column; gap:.85rem; }
  .error-breakdown-select { display:flex; align-items:center; gap:.6rem; flex-wrap:wrap; }
  .error-breakdown-select select { background:#0f172a; border:1px solid #334155; color:#f1f5f9; padding:.4rem .6rem; border-radius:.375rem; font-size:.88rem; }
  .breakdown-bar { display:flex; width:100%; height:1.6rem; border-radius:.375rem; overflow:hidden; background:#0f172a; border:1px solid #334155; }
  .breakdown-bar-seg { height:100%; }
  .breakdown-bar-seg.fcs { background:#f87171; }
  .breakdown-bar-seg.alignment { background:#fbbf24; }
  .breakdown-bar-seg.frametoolong { background:#a78bfa; }
  .breakdown-bar-seg.macreceive { background:#38bdf8; }
  .breakdown-legend { display:flex; flex-wrap:wrap; gap:.9rem; font-size:.8rem; color:#cbd5e1; }
  .breakdown-legend .swatch { display:inline-block; width:.7rem; height:.7rem; border-radius:.2rem; margin-right:.35rem; vertical-align:middle; }
  .breakdown-table { width:100%; border-collapse:collapse; }
  .breakdown-table th, .breakdown-table td { padding:.5rem .65rem; border-bottom:1px solid #334155; font-size:.85rem; text-align:left; }
  .breakdown-table th { color:#94a3b8; font-weight:600; }
    .traffic-protocols { display:grid; grid-template-columns:repeat(12,minmax(0,1fr)); gap:1rem; align-items:stretch; }
    .traffic-card { min-width:0; background:rgba(15,23,42,.6); border:1px solid #334155; border-radius:.5rem; padding:1rem; }
    .traffic-card h3 { margin:0 0 .75rem; color:#e2e8f0; font-size:.95rem; }
    .traffic-summary-card, .traffic-talkers-card { grid-column:span 12; }
    .traffic-timeseries-card { grid-column:span 8; }
    .traffic-protocol-card { grid-column:span 4; }
    .traffic-applications-card, .traffic-endpoints-card { grid-column:span 4; }
    .traffic-controls { display:flex; align-items:center; flex-wrap:wrap; gap:.35rem; margin-bottom:.8rem; }
    .traffic-card-label { color:#94a3b8; font-size:.72rem; font-weight:600; text-transform:uppercase; letter-spacing:.04em; margin-right:.2rem; }
    .traffic-refresh-select { background:#334155; color:#cbd5e1; border:1px solid #475569; border-radius:.3rem; padding:.3rem .45rem; font-size:.75rem; }
    .traffic-controls button { background:#334155; color:#cbd5e1; border:1px solid #475569; border-radius:.3rem; padding:.3rem .55rem; font-size:.75rem; cursor:pointer; }
    .traffic-controls button:hover { background:#475569; color:#fff; }
    .traffic-kpi-row { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:.75rem; }
    .traffic-kpi-row div { display:grid; gap:.2rem; padding:.65rem .75rem; background:rgba(30,41,59,.62); border-radius:.35rem; }
    .traffic-kpi-row span { color:#94a3b8; font-size:.72rem; }
    .traffic-kpi-row strong { color:#f8fafc; font-size:1.05rem; font-weight:600; }
  .traffic-donut-wrap { display:flex; align-items:center; gap:1rem; min-height:180px; }
  .traffic-donut { width:150px; height:150px; border-radius:50%; background:conic-gradient(#38bdf8 0 100%); position:relative; flex:0 0 auto; }
  .traffic-donut::after { content:''; position:absolute; inset:32px; border-radius:50%; background:#1e293b; }
  .traffic-legend { display:grid; gap:.35rem; font-size:.8rem; color:#cbd5e1; }
    .traffic-legend-item { display:flex; justify-content:space-between; gap:.75rem; align-items:center; }
  .traffic-legend-swatch { width:.7rem; height:.7rem; border-radius:.15rem; }
    .traffic-scroll-panel { max-height:260px; overflow-y:auto; padding-right:.25rem; }
    .traffic-share-list { display:grid; gap:.7rem; }
    .traffic-share-item { display:grid; gap:.3rem; }
    .traffic-share-label { display:flex; justify-content:space-between; gap:.6rem; color:#cbd5e1; font-size:.78rem; }
    .traffic-share-label span:last-child { color:#94a3b8; white-space:nowrap; }
    .traffic-share-track { height:.35rem; overflow:hidden; border-radius:999px; background:#1e293b; }
    .traffic-share-track span { display:block; height:100%; border-radius:inherit; background:#38bdf8; }
    .traffic-table-scroll { overflow-x:auto; }
    .traffic-timeseries-scroll { max-height:260px; overflow:auto; }
  .talker-table { width:100%; border-collapse:collapse; }
  .talker-table th, .talker-table td { padding:.45rem .55rem; border-bottom:1px solid #334155; text-align:left; font-size:.8rem; }
  .talker-table th { color:#94a3b8; }
    .traffic-talkers-table { min-width:980px; }
  .traffic-empty { color:#64748b; font-style:italic; padding:.75rem 0; }
  .traffic-note { margin:-.35rem 0 .85rem; color:#94a3b8; font-size:.82rem; }
    @media (max-width:900px) { .traffic-timeseries-card, .traffic-protocol-card { grid-column:span 12; } }
    @media (max-width:760px) { .traffic-protocols { grid-template-columns:1fr; } .traffic-summary-card, .traffic-timeseries-card, .traffic-protocol-card, .traffic-applications-card, .traffic-endpoints-card, .traffic-talkers-card { grid-column:span 1; } .traffic-kpi-row { grid-template-columns:repeat(2,minmax(0,1fr)); } .traffic-donut-wrap { justify-content:center; flex-wrap:wrap; } }
</style>";

const DEVICE_DETAIL_BUNDLE_JS: &str = include_str!("../../frontend/dist/device-detail.js");
const DASHBOARD_BUNDLE_JS: &str = include_str!("../../frontend/dist/dashboard.js");
const SETTINGS_BUNDLE_JS: &str = include_str!("../../frontend/dist/settings.js");
const NOTIFICATIONS_BUNDLE_JS: &str = include_str!("../../frontend/dist/notifications.js");
const DIAGNOSTICS_BUNDLE_JS: &str = include_str!("../../frontend/dist/diagnostics.js");
const DISCOVERY_BUNDLE_JS: &str = include_str!("../../frontend/dist/discovery.js");
const TOPOLOGY_BUNDLE_JS: &str = include_str!("../../frontend/dist/topology.js");
const THEME_BUNDLE_JS: &str = include_str!("../../frontend/dist/theme.js");
const I18N_BUNDLE_JS: &str = include_str!("../../frontend/dist/i18n.js");

const DASHBOARD_CSS: &str = "<style>
  #dash-table th { cursor:pointer; user-select:none; white-space:nowrap; }
  #dash-table th:hover { color:#e2e8f0; }
  #dash-table th.sort-asc, #dash-table th.sort-desc { color:#38bdf8; }
  .sort-icon { font-size:.7rem; margin-left:.25rem; color:#38bdf8; opacity:0; }
  th.sort-asc .sort-icon, th.sort-desc .sort-icon { opacity:1; }
  th.sort-desc .sort-icon { display:inline-block; transform:rotate(180deg); }
  .spike-row { background:rgba(239,68,68,.08); outline:1px solid rgba(239,68,68,.4); }
  .spike-badge { display:inline-flex; align-items:center; gap:.3rem; background:#450a0a; color:#f87171; border:1px solid #7f1d1d; border-radius:.375rem; padding:.25rem .6rem; font-size:.8rem; font-weight:600; margin-left:.75rem; }
  .card-spike { border-color:#7f1d1d !important; }
  .card-spike .card-num { color:#f87171; }
  .row-actions { display:flex; gap:.45rem; align-items:center; }
  .btn-row-danger { background:#7f1d1d; color:#fecaca; border:1px solid #991b1b; padding:.3rem .55rem; border-radius:.35rem; font-size:.78rem; cursor:pointer; }
  .btn-row-danger:hover:not(:disabled) { background:#991b1b; }
  .btn-row-danger:disabled { opacity:.55; cursor:not-allowed; }
</style>";

const DISCOVERY_CSS: &str = "<style>
    .scan-form { margin-bottom:1.5rem; }
    .discovery-community { max-width:260px; margin-bottom:1rem; }
    .discovery-workflows { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:1rem; }
    .discovery-workflow { display:flex; align-items:flex-end; gap:.75rem; padding:.9rem 1rem; border:1px solid #334155; border-radius:.5rem; background:#0f172a; }
    .discovery-workflow .form-group { flex:1; min-width:0; }
    .discovery-workflow .btn { flex:none; }
  .form-group { display:flex; flex-direction:column; gap:.25rem; }
  .form-group label { font-size:.85rem; color:#94a3b8; }
  .form-group input { background:#1e293b; border:1px solid #334155; color:#f1f5f9; padding:.5rem .75rem; border-radius:.375rem; font-size:.95rem; width:100%; box-sizing:border-box; }
  .form-group input:focus { outline:none; border-color:#3b82f6; }
    .discovery-actions { display:flex; justify-content:flex-end; align-items:center; gap:.75rem; margin-top:.75rem; }
    .manual-link { font-size:.85rem; }
    .manual-box .scan-form { display:flex; flex-wrap:wrap; gap:.75rem; align-items:flex-start; }
    .manual-box .scan-form .btn { margin-top:1.45rem; }
    @media (max-width:760px) { .discovery-workflows { grid-template-columns:1fr; } }
    @media (max-width:480px) { .discovery-workflow { flex-direction:column; align-items:stretch; } .discovery-workflow .btn { width:100%; } }
  #scan-progress { display:none; background:#1e293b; border:1px solid #334155; border-radius:.5rem; padding:1rem 1.25rem; margin-bottom:1.25rem; }
  .progress-header { display:flex; justify-content:space-between; align-items:center; margin-bottom:.6rem; font-size:.9rem; }
  .progress-label { color:#94a3b8; }
  .progress-stats { color:#cbd5e1; font-variant-numeric: tabular-nums; }
  .progress-bar-track { background:#0f172a; border-radius:9999px; height:8px; overflow:hidden; }
  .progress-bar-fill { background:#3b82f6; height:100%; border-radius:9999px; width:0%; transition:width .4s ease; }
  .progress-elapsed { font-size:.8rem; color:#64748b; margin-top:.5rem; }
  #results-section { display:none; }
  #scan-error { display:none; background:#450a0a; border:1px solid #7f1d1d; color:#fca5a5; padding:.75rem 1rem; border-radius:.375rem; margin-bottom:1rem; }
  .btn-register { background:#16a34a; font-size:1rem; padding:.65rem 1.5rem; }
  .btn-register:hover:not(:disabled) { background:#15803d; }
  #register-result { display:none; margin-top:1rem; padding:.75rem 1rem; border-radius:.375rem; }
  .reg-success { background:#052e16; border:1px solid #166534; color:#86efac; }
  .reg-error { background:#450a0a; border:1px solid #7f1d1d; color:#fca5a5; }
  table input[type='checkbox']:disabled { opacity:.4; cursor:not-allowed; }
  .badge-registered { font-size:.75rem; background:#1e3a5f; color:#93c5fd; padding:.15rem .5rem; border-radius:9999px; }
  .manual-box { background:#1e293b; border:1px solid #334155; border-radius:.5rem; padding:1rem; margin-bottom:1.5rem; }
  .host-hint { font-size:.78rem; color:#64748b; margin-top:.25rem; line-height:1.5; }
    .host-hint:empty { display:none; }
  .host-hint.warn { color:#fbbf24; }
  #topology-section { display:none; margin-top:1.25rem; }
  .topology-header { display:flex; flex-wrap:wrap; align-items:center; gap:.6rem; margin-bottom:.6rem; }
  .topology-toolbar { margin-left:auto; display:flex; flex-wrap:wrap; gap:.35rem; }
  .btn-xs { padding:.3rem .6rem; font-size:.78rem; }
  .topology-badge { font-size:.72rem; font-weight:600; letter-spacing:.02em; padding:.2rem .55rem; border-radius:9999px; background:#1e3a5f; color:#93c5fd; border:1px solid #1d4ed8; }
  .topology-badge.enterprise { background:#052e16; color:#86efac; border-color:#166534; }
  #topology-canvas-wrap { position:relative; overflow:hidden; background:#0b1220; border:1px solid #334155; border-radius:.5rem; height:520px; }
  html[data-theme='light'] #topology-canvas-wrap { background:#f8fafc; border-color:#cbd5e1; }
  #topology-canvas { width:100%; height:100%; display:block; cursor:grab; touch-action:none; }
  #topology-canvas.panning { cursor:grabbing; }
  .topo-node-shape { fill:#1e293b; stroke:#38bdf8; stroke-width:2; }
  html[data-theme='light'] .topo-node-shape { fill:#ffffff; stroke:#0284c7; }
  .topo-node.endpoint .topo-node-shape { stroke:#94a3b8; }
  .topo-node.seed .topo-node-shape { stroke:#fbbf24; stroke-width:3; }
  .topo-node.offline .topo-node-shape { stroke:#f87171; }
  .topo-node.selected .topo-node-shape { stroke:#f472b6; stroke-width:3; }
  .topo-node { cursor:pointer; }
  .topo-node-label { fill:#e2e8f0; font-size:12px; font-weight:600; text-anchor:middle; dominant-baseline:middle; pointer-events:none; }
  .topo-node-sub { fill:#94a3b8; font-size:10px; text-anchor:middle; dominant-baseline:middle; pointer-events:none; }
  html[data-theme='light'] .topo-node-label { fill:#0f172a; }
  html[data-theme='light'] .topo-node-sub { fill:#64748b; }
  .topo-edge-line { stroke:#475569; stroke-width:2; }
  .topo-edge-line.cdp { stroke-dasharray:6 4; }
  .topo-edge-line.warn { stroke:#f59e0b; stroke-dasharray:6 4; }
  .topo-edge-line.predictive { stroke:#fbbf24; stroke-dasharray:8 6; animation:topo-predictive-dash 1.2s linear infinite; }
  .topo-edge-line.crit { stroke:#ef4444; stroke-width:4; }
  .topo-edge-hit { stroke:transparent; stroke-width:14; cursor:pointer; }
  .topo-edge.selected .topo-edge-line { stroke:#f472b6; stroke-width:3; }
  .topo-edge-label { fill:#94a3b8; font-size:10px; text-anchor:middle; dominant-baseline:middle; pointer-events:none; }
  html[data-theme='light'] .topo-edge-label { fill:#475569; }
  .topology-hidden-indicator { position:absolute; right:.75rem; bottom:.75rem; background:rgba(251,191,36,.14); border:1px solid #b45309; color:#fbbf24; font-size:.78rem; padding:.35rem .7rem; border-radius:.375rem; }
  .topology-empty { position:absolute; inset:0; display:flex; align-items:center; justify-content:center; color:#64748b; font-size:.9rem; text-align:center; padding:1rem; }
  .topology-inspector { position:absolute; top:0; right:0; width:min(320px,80%); height:100%; background:#111c30; border-left:1px solid #334155; transform:translateX(100%); transition:transform .25s ease; overflow-y:auto; }
  html[data-theme='light'] .topology-inspector { background:#ffffff; border-left-color:#cbd5e1; }
  .topology-inspector.open { transform:translateX(0); }
  .inspector-head { display:flex; align-items:center; gap:.5rem; padding:.7rem .9rem; border-bottom:1px solid #334155; font-size:.95rem; }
  html[data-theme='light'] .inspector-head { border-bottom-color:#e2e8f0; }
  .inspector-close { margin-left:auto; background:none; border:none; color:#94a3b8; font-size:1.1rem; cursor:pointer; line-height:1; }
  .inspector-body { padding:.8rem .9rem; font-size:.85rem; color:#cbd5e1; }
  html[data-theme='light'] .inspector-body { color:#334155; }
  .inspector-row { display:flex; gap:.5rem; padding:.25rem 0; border-bottom:1px solid rgba(148,163,184,.15); }
  .inspector-row span:first-child { color:#94a3b8; min-width:96px; }
  .inspector-body h4 { font-size:.8rem; color:#94a3b8; margin:.9rem 0 .35rem; text-transform:uppercase; letter-spacing:.05em; }
  .inspector-if { display:flex; justify-content:space-between; gap:.5rem; padding:.2rem 0; font-size:.8rem; }
  .inspector-if.warn { color:#f59e0b; }
  .inspector-if.crit { color:#ef4444; font-weight:600; }
  .inspector-if-icon { margin-right:.25rem; }
  .topology-list { margin-top:.5rem; display:grid; gap:.4rem; }
  .topology-edge { background:#1e293b; border:1px solid #334155; border-radius:.375rem; padding:.55rem .7rem; font-size:.85rem; color:#cbd5e1; }
  @keyframes topo-predictive-dash { to { stroke-dashoffset:-28; } }
</style>";

const DISCOVERY_HTML: &str = "<main>
  <h1 data-i18n='discovery_title'>Device Discovery</h1>

  <div class='scan-form' id='scan-form'>
        <div class='form-group discovery-community'>
      <label for='community' data-i18n='community'>Community</label>
      <input id='community' type='text' value='public' data-i18n-placeholder='public_placeholder'>
    </div>
        <div class='discovery-workflows'>
            <div class='discovery-workflow'>
                <div class='form-group'>
                    <label for='cidr' data-i18n='cidr_range'>CIDR Range</label>
                    <input id='cidr' type='text' placeholder='192.168.1.0/24' data-i18n-placeholder='cidr_range_placeholder' required oninput='updateHint()'>
                    <span id='host-hint' class='host-hint'></span>
                </div>
                <button class='btn' id='scan-btn' onclick='startScan()' data-i18n='scan'>Scan</button>
            </div>
            <div class='discovery-workflow'>
                <div class='form-group'>
                    <label for='seed-ip' data-i18n='seed_device_ip'>Seed Device IP (LLDP/CDP)</label>
                    <input id='seed-ip' type='text' placeholder='192.168.11.23'>
                </div>
                <button class='btn btn-secondary' id='topology-btn' onclick='runTopologyOnly()' data-i18n='draw_topology'>Draw Topology</button>
            </div>
    </div>
        <div class='discovery-actions'>
            <button class='btn btn-secondary' id='cancel-btn' style='display:none' onclick='cancelScan()' data-i18n='cancel'>Cancel</button>
            <span class='manual-link'><a href='#' onclick='showManual();return false;' data-i18n='add_manually'>+ Add manually</a></span>
        </div>
  </div>

  <div id='scan-progress'>
    <div class='progress-header'>
      <span class='progress-label' id='progress-label' data-i18n='scanning'>Scanning\u{2026}</span>
      <span class='progress-stats' id='progress-stats'>0 / 0</span>
    </div>
    <div class='progress-bar-track'>
      <div class='progress-bar-fill' id='progress-bar'></div>
    </div>
    <div class='progress-elapsed' id='progress-elapsed'>Elapsed: 0s</div>
  </div>

  <div id='manual-box' class='manual-box' style='display:none'>
    <h3 style='margin:0 0 .75rem' data-i18n='add_device_manually'>Add Device Manually</h3>
    <div class='scan-form' style='margin-bottom:0'>
      <div class='form-group' style='flex:1;min-width:150px'>
        <label data-i18n='ip_address'>IP Address</label>
        <input id='manual-ip' type='text' placeholder='192.168.1.1' data-i18n-placeholder='ip_address_placeholder'>
      </div>
      <div class='form-group' style='flex:1;min-width:130px'>
        <label data-i18n='name'>Name</label>
        <input id='manual-name' type='text' placeholder='router-01' data-i18n-placeholder='name_placeholder'>
      </div>
      <div class='form-group' style='flex:1;min-width:120px'>
        <label data-i18n='community'>Community</label>
        <input id='manual-community' type='text' value='public' data-i18n-placeholder='public_placeholder'>
      </div>
      <button class='btn' style='margin-top:1.45rem' onclick='addManual()' data-i18n='add'>Add</button>
      <button class='btn btn-secondary' style='margin-top:1.45rem' onclick='hideManual()' data-i18n='cancel'>Cancel</button>
    </div>
    <div id='manual-result' style='margin-top:.5rem;font-size:.85rem'></div>
  </div>

  <div id='scan-error'></div>

  <div id='topology-section'>
    <div class='topology-header'>
      <h2 style='margin:0' data-i18n='topology_map'>Topology Map</h2>
      <span id='edition-badge' class='topology-badge'></span>
      <div class='topology-toolbar'>
        <button class='btn btn-secondary btn-xs' onclick='topoZoom(1.2)' data-i18n-title='topology_zoom_in' title='Zoom in'>+</button>
        <button class='btn btn-secondary btn-xs' onclick='topoZoom(0.8)' data-i18n-title='topology_zoom_out' title='Zoom out'>&#8722;</button>
        <button class='btn btn-secondary btn-xs' onclick='topoFit()' data-i18n='topology_fit'>Fit</button>
        <button class='btn btn-secondary btn-xs' onclick='topoRelayout()' data-i18n='topology_auto_layout'>Auto layout</button>
        <button class='btn btn-secondary btn-xs' onclick='exportTopology(&quot;json&quot;)' data-i18n='topology_export_json'>Export JSON</button>
        <button class='btn btn-secondary btn-xs' onclick='exportTopology(&quot;csv&quot;)' data-i18n='topology_export_csv'>Export CSV</button>
      </div>
    </div>
    <div id='topology-canvas-wrap'>
      <svg id='topology-canvas'></svg>
      <div id='topology-empty' class='topology-empty'></div>
      <div id='topology-hidden' class='topology-hidden-indicator' style='display:none'></div>
      <aside id='topology-inspector' class='topology-inspector'>
        <div class='inspector-head'>
          <strong id='inspector-title'></strong>
          <button class='inspector-close' onclick='closeInspector()' data-i18n-title='topology_close' title='Close'>&#215;</button>
        </div>
        <div id='inspector-body' class='inspector-body'></div>
      </aside>
    </div>
    <div id='topology-result' class='topology-list'></div>
  </div>

  <div id='results-section'>
    <h2 style='margin:0 0 .75rem' id='results-title' data-i18n='scan_results'>Scan Results</h2>
    <table id='results-table'>
      <thead>
        <tr>
          <th style='width:2.5rem'><input type='checkbox' id='select-all' onchange='toggleAll(this)' data-i18n-title='select_all'></th>
          <th data-i18n='ip_address'>IP</th>
          <th data-i18n='hostname'>Hostname</th>
          <th data-i18n='status'>Status</th>
          <th data-i18n='registration'>Registration</th>
        </tr>
      </thead>
      <tbody id='results-body'></tbody>
    </table>
    <div id='register-action' style='display:none;margin-top:1rem;align-items:center;gap:1rem'>
      <button class='btn btn-register' id='register-btn' onclick='registerSelected()' data-i18n='register_selected'>&#10003; Register Selected</button>
      <span id='select-count' style='font-size:.85rem;color:#94a3b8'></span>
    </div>
    <div id='register-result'></div>
  </div>
</main>";

// SCAN_TIMEOUT_SECS: クライアント側のポーリングタイムアウト（秒）
// ホスト数 × 推定時間で動的に計算するが上限はこの値
// ─── Utilities ────────────────────────────────────────────────────────────────

fn parse_form(body: &str) -> std::collections::HashMap<String, String> {
    body.split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (url_decode(k), url_decode(v)))
        .collect()
}

fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.replace('+', " ").into_bytes().into_iter();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next().unwrap_or(b'0');
            let h2 = chars.next().unwrap_or(b'0');
            let hex = format!("{}{}", h1 as char, h2 as char);
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
                continue;
            }
        }
        result.push(b as char);
    }
    result
}

fn escape_json(s: &str) -> String {
    s.replace('\\', r"\\")
        .replace('"', "\\\"")
        .replace('\n', r"\n")
}

fn effective_interface_link_status(device_status: &str, sample_link_status: &str) -> String {
    if device_status.eq_ignore_ascii_case("offline") {
        "down".to_string()
    } else {
        sample_link_status.to_string()
    }
}

/// インターフェース状態テーブルの診断ステータスバッジを優先度順に判定する。
/// リンクダウン中はカウンタが陳腐化しているため最優先で "down" とする。
fn classify_interface_diagnostic(
    link_status: &str,
    in_errors_delta: u64,
    out_errors_delta: u64,
    in_discards_delta: u64,
    out_discards_delta: u64,
    late_collisions_delta: u64,
) -> &'static str {
    if !link_status.eq_ignore_ascii_case("up") {
        "down"
    } else if late_collisions_delta > 0 {
        "duplex_mismatch"
    } else if in_errors_delta > 0 || out_errors_delta > 0 {
        "l1_error"
    } else if in_discards_delta > 0 || out_discards_delta > 0 {
        "congestion"
    } else {
        "healthy"
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx + 1 < UNITS.len() {
        value /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{} {}", bytes, UNITS[idx])
    } else {
        format!("{:.1} {}", value, UNITS[idx])
    }
}

#[allow(dead_code)]
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Parse a JSON array of {ip, name, community} objects (minimal hand-rolled parser).
fn parse_register_body(body: &str) -> Vec<DeviceConfig> {
    let mut devices = Vec::new();
    for chunk in body.split('}') {
        let ip = extract_json_str(chunk, "ip");
        let name = extract_json_str(chunk, "name");
        let community = extract_json_str(chunk, "community");
        if let Some(ip) = ip {
            let name = name
                .unwrap_or_else(|| format!("device-{}", ip.split('.').next_back().unwrap_or("x")));
            let community = community.unwrap_or_else(|| "public".to_string());
            let mut d = DeviceConfig::new(ip.clone(), community);
            d.name = name;
            devices.push(d);
        }
    }
    devices
}

fn extract_json_str(chunk: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let pos = chunk.find(&needle)?;
    let rest = &chunk[pos + needle.len()..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    let inner = &after_colon[1..];
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

// ─── Background polling loop ──────────────────────────────────────────────────

/// `verbose` が false の間は進捗を標準出力へ書かない（TUI の描画を壊さないため）。
pub(crate) fn run_polling_loop(
    config: Arc<Mutex<AppConfig>>,
    repo: Arc<Mutex<Repository>>,
    alerts: crate::alert::AlertBroadcaster,
    verbose: bool,
) {
    run_polling_loop_with_observer(config, repo, alerts, None, verbose);
}

fn run_polling_loop_with_observer(
    config: Arc<Mutex<AppConfig>>,
    repo: Arc<Mutex<Repository>>,
    alerts: crate::alert::AlertBroadcaster,
    observer: Option<Arc<dyn WebExtensionProvider>>,
    verbose: bool,
) {
    use crate::db::models::{AlertEvent, DeviceMetrics, InterfaceSample};
    use crate::monitor::calculate_bandwidth_utilization_from_delta;
    use crate::snmp::SnmpClient;
    use std::time::Duration;

    if verbose {
        println!("Polling engine started");
    }
    const OFFLINE_GRACE_INTERVALS: i64 = 3;

    loop {
        let (interval, spike_threshold) = config
            .lock()
            .map(|c| (c.polling.interval_seconds, c.alert.spike_threshold))
            .unwrap_or((30, 10));
        let snmp_overrides = config.lock().map(|c| c.snmp.clone()).unwrap_or_default();
        std::thread::sleep(Duration::from_secs(interval));

        let devices = match repo.lock() {
            Ok(r) => r.list_devices().unwrap_or_default(),
            Err(_) => continue,
        };

        if devices.is_empty() {
            continue;
        }

        let now = chrono_now();

        // 機器ごとに並列スレッドで SNMP 収集し、結果を Vec に集める
        // （SNMP はネットワーク I/O なのでスレッド並列が有効）
        let handles: Vec<_> = devices
            .into_iter()
            .map(|device| {
                let now = now.clone();
                let snmp_overrides = snmp_overrides.clone();
                std::thread::spawn(move || -> PollResult {
                    let device_id = match device.id {
                        Some(id) => id,
                        None => return PollResult::default_for(device.ip.clone()),
                    };
                    let dev_config = DeviceConfig {
                        id: device.id,
                        name: device.name.clone(),
                        ip: device.ip.clone(),
                        community: device.community.clone(),
                        device_type: device.device_type.clone(),
                        status: device.status.clone(),
                        last_seen_at: device.last_seen_at.clone(),
                    };

                    let client =
                        SnmpClient::with_snmp_config(&dev_config.community, snmp_overrides);

                    // 死活確認
                    let is_online = client.probe_device(&dev_config).is_ok();
                    let status = if is_online {
                        "online"
                    } else {
                        let grace_secs =
                            interval.saturating_mul(OFFLINE_GRACE_INTERVALS as u64) as i64;
                        let last_seen_secs = device
                            .last_seen_at
                            .as_ref()
                            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
                            .map(|dt| {
                                (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_seconds()
                            })
                            .unwrap_or(i64::MAX);
                        if last_seen_secs <= grace_secs {
                            "warning"
                        } else {
                            "offline"
                        }
                    };
                    if verbose {
                        println!("polling: {} ({}) -> {}", device.name, device.ip, status);
                    }

                    let mut result = PollResult {
                        ip: device.ip.clone(),
                        name: device.name.clone(),
                        community: device.community.clone(),
                        device_id,
                        previous_status: device.status.clone(),
                        status: status.to_string(),
                        metrics: None,
                        samples: vec![],
                    };

                    if is_online {
                        // CPU/メモリ
                        if let Ok(info) = client.query_device(&dev_config) {
                            result.metrics = Some(DeviceMetrics {
                                id: None,
                                device_id,
                                cpu_usage: info.cpu_usage,
                                memory_usage: info.memory_usage,
                                memory_used_bytes: info.memory_used_bytes,
                                sampled_at: now.clone(),
                            });
                        }

                        // インターフェース
                        let indexes = client
                            .discover_interface_indexes(&dev_config)
                            .unwrap_or_else(|_| vec![1, 2, 3]);
                        if verbose {
                            println!(
                                "polling: {} ({}) if_indexes={:?}",
                                device.name, device.ip, indexes
                            );
                        }
                        for if_index in indexes.iter() {
                            match client.query_interface(&dev_config, *if_index, None) {
                                Ok(iface) => {
                                    result.samples.push(InterfaceSample {
                                        id: None,
                                        device_id,
                                        if_index: iface.if_index,
                                        if_name: iface.if_name.clone(),
                                        link_status: iface.link_status.clone(),
                                        in_errors: iface.in_errors,
                                        out_errors: iface.out_errors,
                                        in_packets: iface.in_packets,
                                        out_packets: iface.out_packets,
                                        in_discards: iface.in_discards,
                                        out_discards: iface.out_discards,
                                        late_collisions: iface.late_collisions,
                                        fcs_errors: iface.fcs_errors,
                                        alignment_errors: iface.alignment_errors,
                                        frame_too_longs: iface.frame_too_longs,
                                        internal_mac_receive_errors: iface
                                            .internal_mac_receive_errors,
                                        rx_optical_power_dbm: iface.rx_optical_power_dbm,
                                        in_octets: iface.in_octets,
                                        out_octets: iface.out_octets,
                                        bandwidth_utilization: iface.bandwidth_utilization,
                                        sampled_at: now.clone(),
                                    });
                                }
                                Err(e) => {
                                    if verbose {
                                        eprintln!(
                                            "polling: query_interface failed {} if_index={}: {}",
                                            device.ip, if_index, e
                                        );
                                    }
                                }
                            }
                        }
                    }
                    result
                })
            })
            .collect();

        // 全スレッドの完了を待ち、結果を DB に一括書き込み
        for handle in handles {
            let result = match handle.join() {
                Ok(r) => r,
                Err(_) => continue,
            };

            if let Ok(r) = repo.lock() {
                let _ = r.update_device_status(&result.ip, &result.status);

                let dev_config = DeviceConfig {
                    id: Some(result.device_id),
                    name: result.name.clone(),
                    ip: result.ip.clone(),
                    community: result.community.clone(),
                    device_type: "network".to_string(),
                    status: result.status.clone(),
                    last_seen_at: None,
                };

                // ダウン継続中に毎周期通知しないよう、遷移した瞬間だけ配信する。
                if result.status == "offline" && result.previous_status != "offline" {
                    alerts.publish(crate::alert::AlertEvent::device_offline(
                        &dev_config,
                        "SNMP probe failed",
                    ));
                }

                if let Some(metrics) = result.metrics {
                    let _ = r.save_device_metrics(&metrics);
                }

                for sample in &result.samples {
                    let mut sample = sample.clone();
                    if let Ok(Some(prev)) =
                        r.get_latest_interface_sample(sample.device_id, sample.if_index)
                    {
                        sample.bandwidth_utilization = calculate_bandwidth_utilization_from_delta(
                            prev.in_octets.saturating_add(prev.out_octets),
                            sample.in_octets.saturating_add(sample.out_octets),
                            interval,
                            1_000_000_000,
                        );
                    }
                    let _ = r.save_sample(&sample);
                    if let Some(observer) = observer.as_deref() {
                        let history = r
                            .get_recent_interface_samples(sample.device_id, sample.if_index, 3)
                            .unwrap_or_default();
                        observer.observe_interface_sample(&dev_config, &sample, &history, &alerts);
                    }
                }

                // スパイク検出（インターフェース単位）
                match r.check_interface_spikes(result.device_id, spike_threshold) {
                    Ok(spikes) => {
                        for spike in spikes {
                            if verbose {
                                println!(
                                    "polling: SPIKE on {} ({}) {} (if-{}) total_delta={} in_errors={} out_errors={} in_discards={} out_discards={}",
                                    result.name,
                                    result.ip,
                                    spike.if_name,
                                    spike.if_index,
                                    spike.total_delta,
                                    spike.in_errors_delta,
                                    spike.out_errors_delta,
                                    spike.in_discards_delta,
                                    spike.out_discards_delta
                                );
                            }
                            let alert = AlertEvent {
                                id: None,
                                device_id: result.device_id,
                                alert_type: "interface_error_spike".to_string(),
                                severity: "warning".to_string(),
                                details: format!(
                                    "Interface {} (if-{}) spiked by {}: in_errors +{}, out_errors +{}, in_discards +{}, out_discards +{} (threshold: {})",
                                    spike.if_name,
                                    spike.if_index,
                                    spike.total_delta,
                                    spike.in_errors_delta,
                                    spike.out_errors_delta,
                                    spike.in_discards_delta,
                                    spike.out_discards_delta,
                                    spike_threshold
                                ),
                                created_at: None,
                            };
                            let _ = r.save_alert(&alert);
                            alerts.publish(crate::alert::AlertEvent::from_interface_spike(
                                &dev_config,
                                &spike,
                                spike_threshold,
                            ));
                        }
                    }
                    Err(e) => {
                        if verbose {
                            eprintln!("spike check failed for {}: {}", result.ip, e);
                        }
                    }
                }
            }
        }
    }
}

// ポーリング結果を運ぶ型（スレッド間で Send 可能）
struct PollResult {
    ip: String,
    name: String,
    community: String,
    device_id: i64,
    previous_status: String,
    status: String,
    metrics: Option<crate::db::models::DeviceMetrics>,
    samples: Vec<crate::db::models::InterfaceSample>,
}

impl PollResult {
    fn default_for(ip: String) -> Self {
        Self {
            ip,
            name: String::new(),
            community: "public".to_string(),
            device_id: 0,
            previous_status: "offline".to_string(),
            status: "offline".to_string(),
            metrics: None,
            samples: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CDP_CACHE_DEVICE_ID_OID, COMMUNITY_MAX_DEVICES, WebLimits, WebTopologyEdge,
        WebTopologyInterface, WebTopologyReport, annotate_edge_health, api_flow_analytics,
        api_live_flow_analytics, classify_interface_diagnostic, effective_interface_link_status,
        evaluate_port_health, merge_duplicate_edges, page_dashboard, page_device_detail,
        page_discovery, parse_cdp_edges, persist_config,
    };
    use crate::config::AppConfig;
    use crate::db::models::{Device, FlowRecord, InterfacePortDelta, InterfaceSample};
    use crate::db::repository::Repository;
    use crate::db::sqlite::initialize_database;
    use rusqlite::Connection;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[test]
    fn flow_analytics_api_exposes_extended_contract() {
        let path =
            std::env::temp_dir().join(format!("tracepulse-flow-api-{}.db", std::process::id()));
        let connection = initialize_database(&path).expect("database should initialize");
        let repository = Repository::new(connection);
        repository
            .save_flow_record(&FlowRecord {
                exporter_ip: Some("192.0.2.10".to_string()),
                source_ip: "192.0.2.1".to_string(),
                destination_ip: "198.51.100.2".to_string(),
                source_port: 50000,
                destination_port: 443,
                protocol: "TCP".to_string(),
                bytes: 1000,
                packets: 10,
                ingress_if_index: Some(7),
                egress_if_index: Some(8),
                tcp_flags: 0x12,
                sampling_rate: 1,
                dscp: 0,
                bgp_next_hop: None,
                l7_hostname: None,
                observed_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            })
            .expect("flow should save");
        let repository = Arc::new(Mutex::new(repository));
        let body = api_flow_analytics(&repository, 60, 10);
        assert!(body.contains("\"summary\""));
        assert!(body.contains("\"protocols\""));
        assert!(body.contains("\"top_talkers\""));
        assert!(body.contains("\"packets\""));
        assert!(body.contains("\"tcp_flags\""));
        assert!(body.contains("\"ingress_if_index\":7"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn dashboard_has_no_extension_without_provider() {
        let path = std::env::temp_dir().join(format!(
            "tracepulse-dashboard-edition-{}.db",
            std::process::id()
        ));
        let repository =
            Repository::new(initialize_database(&path).expect("database should initialize"));
        let repository = Arc::new(Mutex::new(repository));
        let config = Arc::new(Mutex::new(AppConfig::default()));

        let community = page_dashboard(&repository, &config, None);
        assert!(!community.contains("Sankey Flow"));
        assert!(!community.contains("GeoIP / ASN"));
        assert!(!community.contains("BGP / QoS"));
        assert!(!community.contains("Threat Badges"));
        assert!(!community.contains("Flow Explorer"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn device_detail_hides_traffic_section_until_flow_data_arrives() {
        let path = std::env::temp_dir().join(format!(
            "tracepulse-device-traffic-section-{}.db",
            std::process::id()
        ));
        let repository =
            Repository::new(initialize_database(&path).expect("database should initialize"));
        repository
            .save_device(&Device {
                id: None,
                name: "router".to_string(),
                ip: "192.0.2.1".to_string(),
                community: "public".to_string(),
                device_type: "router".to_string(),
                status: "online".to_string(),
                last_seen_at: None,
                created_at: None,
                updated_at: None,
            })
            .expect("device should save");
        let html = page_device_detail("192.0.2.1", &Arc::new(Mutex::new(repository)));

        assert!(html.contains("id='traffic-protocols-section' style='display:none'"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn live_flow_analytics_exposes_all_sections() {
        let records = vec![FlowRecord {
            exporter_ip: Some("192.0.2.10".to_string()),
            source_ip: "192.0.2.1".to_string(),
            destination_ip: "198.51.100.2".to_string(),
            source_port: 50000,
            destination_port: 443,
            protocol: "TCP".to_string(),
            bytes: 2048,
            packets: 20,
            ingress_if_index: Some(7),
            egress_if_index: Some(8),
            tcp_flags: 0x12,
            sampling_rate: 1,
            dscp: 0,
            bgp_next_hop: None,
            l7_hostname: None,
            observed_at: "2026-09-12 12:00:00".to_string(),
        }];
        let body = api_live_flow_analytics(&records, 60, 10);
        for section in [
            "summary",
            "timeseries",
            "protocols",
            "applications",
            "top_sources",
            "top_destinations",
            "top_talkers",
        ] {
            assert!(
                body.contains(&format!("\"{section}\"")),
                "missing {section}: {body}"
            );
        }
        assert!(body.contains("HTTPS (TCP/443)"));
        assert!(body.contains("tcp_flags"));
    }

    #[test]
    fn long_window_rollup_preserves_flags_and_exporter_interface_names() {
        let path = std::env::temp_dir().join(format!(
            "tracepulse-flow-rollup-api-{}.db",
            std::process::id()
        ));
        let connection = initialize_database(&path).expect("database should initialize");
        let repository = Repository::new(connection);
        let device_id = repository
            .save_device(&Device {
                id: None,
                name: "exporter".to_string(),
                ip: "192.0.2.10".to_string(),
                community: "public".to_string(),
                device_type: "router".to_string(),
                status: "online".to_string(),
                last_seen_at: None,
                created_at: None,
                updated_at: None,
            })
            .expect("exporter should save");
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        repository
            .save_sample(&InterfaceSample {
                id: None,
                device_id,
                if_index: 7,
                if_name: "GigabitEthernet1".to_string(),
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
                sampled_at: now.clone(),
            })
            .expect("interface sample should save");
        repository
            .save_flow_record(&FlowRecord {
                exporter_ip: Some("192.0.2.10".to_string()),
                source_ip: "192.0.2.1".to_string(),
                destination_ip: "198.51.100.2".to_string(),
                source_port: 50000,
                destination_port: 443,
                protocol: "TCP".to_string(),
                bytes: 1000,
                packets: 10,
                ingress_if_index: Some(7),
                egress_if_index: Some(0),
                tcp_flags: 0x12,
                sampling_rate: 1,
                dscp: 0,
                bgp_next_hop: None,
                l7_hostname: None,
                observed_at: now,
            })
            .expect("flow should save");
        let body = api_flow_analytics(&Arc::new(Mutex::new(repository)), 3600, 10);
        assert!(body.contains("GigabitEthernet1"));
        assert!(body.contains("tcp_flags"));
        assert!(body.contains("SYN") || body.contains("ACK"));
        assert!(body.contains("Internal/Local"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn persist_config_preserves_unmanaged_toml_keys() {
        let path = std::env::temp_dir().join(format!(
            "tracepulse-config-merge-{}.toml",
            std::process::id()
        ));
        std::fs::write(
        &path,
        "[polling]\ninterval_seconds = 10\ncustom_polling_key = true\n\n[notifier]\nendpoint = 'https://example.invalid'\n",
      )
      .expect("test config should be written");

        let mut config = AppConfig::default();
        config.polling.interval_seconds = 45;
        persist_config(&config, &path).expect("config should be persisted");

        let content = std::fs::read_to_string(&path).expect("test config should be readable");
        let root: toml::Table = content
            .parse()
            .expect("persisted config should be valid TOML");
        let polling = root["polling"]
            .as_table()
            .expect("polling should be a table");
        let notifier = root["notifier"]
            .as_table()
            .expect("notifier should be preserved");

        assert_eq!(polling["interval_seconds"].as_integer(), Some(45));
        assert_eq!(polling["custom_polling_key"].as_bool(), Some(true));
        assert_eq!(
            notifier["endpoint"].as_str(),
            Some("https://example.invalid")
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn offline_device_forces_interface_status_down() {
        assert_eq!(effective_interface_link_status("offline", "up"), "down");
    }

    #[test]
    fn online_device_preserves_sample_interface_status() {
        assert_eq!(effective_interface_link_status("online", "up"), "up");
        assert_eq!(effective_interface_link_status("online", "down"), "down");
    }

    #[test]
    fn discovery_page_exposes_unrestricted_limits_to_browser_js() {
        let repo = Arc::new(Mutex::new(Repository::new(
            Connection::open_in_memory().expect("in-memory db should open"),
        )));

        let html = page_discovery(&repo, WebLimits::unrestricted(), None);

        assert!(
            html.contains("window.TRACEPULSE_DISCOVERY_LIMITS = {maxCidrs:null,nodeLimit:null};")
        );
        assert!(html.contains("/static/js/discovery.js"));
        assert!(html.contains("/static/js/topology.js"));
        assert!(html.contains("id='seed-ip'"));
    }

    #[test]
    fn discovery_page_exposes_default_limits_to_browser_js() {
        let repo = Arc::new(Mutex::new(Repository::new(
            Connection::open_in_memory().expect("in-memory db should open"),
        )));

        let html = page_discovery(&repo, WebLimits::community(), None);

        assert!(html.contains("window.TRACEPULSE_DISCOVERY_LIMITS = {maxCidrs:1,nodeLimit:25};"));
    }

    fn topology_report_with(node_count: usize) -> WebTopologyReport {
        let mut edges = Vec::new();
        for index in 1..node_count {
            edges.push(WebTopologyEdge {
                id: format!("e{index}"),
                protocol: "LLDP".to_string(),
                source: "10.0.0.1".to_string(),
                target: format!("10.0.0.{}", index + 1),
                local_ip: "10.0.0.1".to_string(),
                local_if_index: Some(index as i64),
                local_port: Some(format!("Gi1/0/{index}")),
                remote_ip: Some(format!("10.0.0.{}", index + 1)),
                remote_hostname: Some(format!("sw-{index}")),
                remote_port: Some("Gi1/0/23".to_string()),
                health_status: None,
            });
        }

        let mut report = WebTopologyReport {
            seed_ip: "10.0.0.1".to_string(),
            nodes: Vec::new(),
            edges,
            warnings: Vec::new(),
            total_nodes: 0,
            hidden_nodes: 0,
            node_limit: None,
        };
        report.rebuild_nodes(&BTreeMap::new());
        report
    }

    #[test]
    fn duplicate_lldp_and_cdp_edges_to_same_neighbor_are_merged() {
        let lldp = WebTopologyEdge {
            id: "e0".to_string(),
            protocol: "LLDP".to_string(),
            source: "10.0.0.1".to_string(),
            target: "10.0.0.2".to_string(),
            local_ip: "10.0.0.1".to_string(),
            local_if_index: Some(2),
            local_port: None,
            remote_ip: Some("10.0.0.2".to_string()),
            remote_hostname: Some("core-sw-02".to_string()),
            remote_port: Some("Gi1/0/23".to_string()),
            health_status: None,
        };
        let cdp = WebTopologyEdge {
            id: "e1".to_string(),
            protocol: "CDP".to_string(),
            source: "10.0.0.1".to_string(),
            target: "10.0.0.2".to_string(),
            local_ip: "10.0.0.1".to_string(),
            local_if_index: Some(3),
            local_port: Some("Gi1/0/2".to_string()),
            remote_ip: Some("10.0.0.2".to_string()),
            remote_hostname: Some("core-sw-02".to_string()),
            remote_port: Some("Gi1/0/23".to_string()),
            health_status: None,
        };

        let merged = merge_duplicate_edges(vec![lldp, cdp]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].protocol, "LLDP+CDP");
        assert_eq!(merged[0].local_if_index, Some(2));
        assert_eq!(merged[0].local_port, Some("Gi1/0/2".to_string()));
    }

    #[test]
    fn edges_with_distinct_remote_ports_are_kept_separate() {
        let a = WebTopologyEdge {
            id: "e0".to_string(),
            protocol: "LLDP".to_string(),
            source: "10.0.0.1".to_string(),
            target: "10.0.0.2".to_string(),
            local_ip: "10.0.0.1".to_string(),
            local_if_index: Some(2),
            local_port: Some("Gi1/0/2".to_string()),
            remote_ip: Some("10.0.0.2".to_string()),
            remote_hostname: Some("core-sw-02".to_string()),
            remote_port: Some("Gi1/0/23".to_string()),
            health_status: None,
        };
        let b = WebTopologyEdge {
            id: "e1".to_string(),
            protocol: "LLDP".to_string(),
            source: "10.0.0.1".to_string(),
            target: "10.0.0.2".to_string(),
            local_ip: "10.0.0.1".to_string(),
            local_if_index: Some(3),
            local_port: Some("Gi1/0/3".to_string()),
            remote_ip: Some("10.0.0.2".to_string()),
            remote_hostname: Some("core-sw-02".to_string()),
            remote_port: Some("Gi1/0/24".to_string()),
            health_status: None,
        };

        let merged = merge_duplicate_edges(vec![a, b]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn cdp_local_if_index_uses_ifindex_not_device_index() {
        let varbind = crate::snmp::SnmpVarBind {
            oid: [CDP_CACHE_DEVICE_ID_OID.as_slice(), &[23, 1]].concat(),
            value: crate::snmp::SnmpValue::OctetString(b"core-sw-02".to_vec()),
        };

        let edges = parse_cdp_edges("10.0.0.1", &[varbind]);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].local_if_index, Some(23));
    }

    #[test]
    fn topology_is_truncated_to_configured_node_limit() {
        let mut report = topology_report_with(30);
        report.apply_node_limit(WebLimits::community());

        assert_eq!(report.nodes.len(), COMMUNITY_MAX_DEVICES);
        assert_eq!(report.hidden_nodes, 30 - COMMUNITY_MAX_DEVICES);
        assert!(
            report
                .edges
                .iter()
                .all(|edge| report.nodes.iter().any(|node| node.id == edge.target))
        );
    }

    #[test]
    fn unrestricted_topology_keeps_every_node() {
        let mut report = topology_report_with(30);
        report.apply_node_limit(WebLimits::unrestricted());

        assert_eq!(report.nodes.len(), 30);
        assert_eq!(report.hidden_nodes, 0);
        assert!(report.to_json().contains(r#""node_limit":null"#));
    }

    #[test]
    fn topology_json_exposes_graph_schema() {
        let mut report = topology_report_with(3);
        report.apply_node_limit(WebLimits::community());
        let json = report.to_json();

        assert!(json.contains(r#""nodes":[{"id":"10.0.0.1""#));
        assert!(json.contains(r#""kind":"switch""#));
        assert!(json.contains(r#""source":"10.0.0.1","target":"10.0.0.2""#));
        assert!(json.contains("\"label\":\"Gi1/0/1 \u{2194} Gi1/0/23\""));
        assert!(json.contains(r#""hidden_nodes":0"#));
    }

    #[test]
    fn l1_physical_error_is_flagged_warning_below_threshold() {
        let delta = InterfacePortDelta {
            in_errors_delta: 5,
            ..Default::default()
        };
        let (status, alerts) = evaluate_port_health(&delta);
        assert_eq!(status, "Warning");
        assert_eq!(
            alerts,
            vec!["L1 Physical Error (CRC/Frame Error)".to_string()]
        );
    }

    #[test]
    fn l1_physical_error_escalates_to_critical_above_threshold() {
        let delta = InterfacePortDelta {
            in_errors_delta: 50,
            ..Default::default()
        };
        let (status, _) = evaluate_port_health(&delta);
        assert_eq!(status, "Critical");
    }

    #[test]
    fn late_collisions_are_flagged_as_duplex_mismatch() {
        let delta = InterfacePortDelta {
            late_collisions_delta: 1,
            ..Default::default()
        };
        let (status, alerts) = evaluate_port_health(&delta);
        assert_eq!(status, "Warning");
        assert_eq!(alerts, vec!["Duplex Mismatch / Late Collision".to_string()]);
    }

    #[test]
    fn discards_are_flagged_as_congestion() {
        let delta = InterfacePortDelta {
            out_discards_delta: 10,
            ..Default::default()
        };
        let (status, alerts) = evaluate_port_health(&delta);
        assert_eq!(status, "Warning");
        assert_eq!(
            alerts,
            vec!["Buffer Overflow / Congestion (Packet Discards)".to_string()]
        );
    }

    #[test]
    fn healthy_port_has_no_alerts() {
        let (status, alerts) = evaluate_port_health(&InterfacePortDelta::default());
        assert_eq!(status, "Healthy");
        assert!(alerts.is_empty());
    }

    #[test]
    fn edge_inherits_local_port_health_from_source_node() {
        let mut report = topology_report_with(2);
        let unhealthy_iface = WebTopologyInterface {
            if_index: Some(1),
            if_name: "Gi1/0/1".to_string(),
            link_status: "up".to_string(),
            metrics: InterfacePortDelta {
                in_errors_delta: 5,
                ..Default::default()
            },
            health_status: "Warning".to_string(),
            alerts: vec!["L1 Physical Error (CRC/Frame Error)".to_string()],
        };
        report.nodes[0].interfaces = vec![unhealthy_iface];

        annotate_edge_health(&mut report);

        assert_eq!(report.edges[0].health_status, Some("Warning".to_string()));
    }

    #[test]
    fn diagnostic_status_prioritizes_down_over_counters() {
        assert_eq!(classify_interface_diagnostic("down", 5, 0, 0, 0, 5), "down");
    }

    #[test]
    fn diagnostic_status_prioritizes_duplex_mismatch_over_l1_error() {
        assert_eq!(
            classify_interface_diagnostic("up", 5, 0, 0, 0, 1),
            "duplex_mismatch"
        );
    }

    #[test]
    fn diagnostic_status_flags_l1_error_over_congestion() {
        assert_eq!(
            classify_interface_diagnostic("up", 1, 0, 3, 0, 0),
            "l1_error"
        );
    }

    #[test]
    fn diagnostic_status_flags_congestion_when_only_discards_present() {
        assert_eq!(
            classify_interface_diagnostic("up", 0, 0, 0, 4, 0),
            "congestion"
        );
    }

    #[test]
    fn diagnostic_status_is_healthy_when_no_deltas() {
        assert_eq!(
            classify_interface_diagnostic("up", 0, 0, 0, 0, 0),
            "healthy"
        );
    }
}

/// UTC 現在時刻を ISO 8601 文字列で返す（chrono 非依存）
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, mi, s)
}

fn epoch_to_ymd_hms(mut secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    secs /= 60;
    let mi = secs % 60;
    secs /= 60;
    let h = secs % 24;
    secs /= 24;
    // days since 1970-01-01
    let mut days = secs;
    let mut y = 1970u64;
    loop {
        let dy = if (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400) {
            366
        } else {
            365
        };
        if days < dy {
            break;
        }
        days -= dy;
        y += 1;
    }
    let leap = (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400);
    let months = [
        31u64,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut mo = 1u64;
    for dm in &months {
        if days < *dm {
            break;
        }
        days -= dm;
        mo += 1;
    }
    (y, mo, days + 1, h, mi, s)
}
