use crate::config::AppConfig;
use crate::db::repository::Repository;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::snmp::SnmpClient;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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

pub struct WebServer {
    pub host: String,
    pub port: u16,
    config: Arc<Mutex<AppConfig>>,
    config_path: std::path::PathBuf,
    repository: Arc<Mutex<Repository>>,
    jobs: JobStore,
}

impl WebServer {
    pub fn new(host: impl Into<String>, port: u16, config: AppConfig, repository: Repository) -> Self {
        let config_path = crate::config_path();
        Self {
            host: host.into(),
            port,
            config: Arc::new(Mutex::new(config)),
            config_path,
            repository: Arc::new(Mutex::new(repository)),
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self) -> Result<(), AppError> {
        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr).map_err(|err| AppError::Io(err.to_string()))?;

        println!("Web dashboard started on http://{}/", addr);

        // バックグラウンドポーリングスレッドを起動
        {
            let repo = Arc::clone(&self.repository);
            let cfg = Arc::clone(&self.config);
            std::thread::spawn(move || {
                run_polling_loop(cfg, repo);
            });
        }

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let repo = Arc::clone(&self.repository);
                    let jobs = Arc::clone(&self.jobs);
                    let cfg = Arc::clone(&self.config);
                    let cfg_path = self.config_path.clone();
                    if let Err(err) = handle_connection(stream, repo, jobs, cfg, cfg_path) {
                        eprintln!("web request failed: {err}");
                    }
                }
                Err(err) => {
                    eprintln!("accept failed: {err}");
                }
            }
        }

        Ok(())
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

fn respond(mut stream: TcpStream, status: &str, content_type: &str, body: String) -> Result<(), AppError> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
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

// ─── Router ──────────────────────────────────────────────────────────────────

fn handle_connection(
    stream: TcpStream,
    repo: Arc<Mutex<Repository>>,
    jobs: JobStore,
    cfg: Arc<Mutex<AppConfig>>,
    cfg_path: std::path::PathBuf,
) -> Result<(), AppError> {
    let req = parse_request(&stream)?;

    // Route: GET /api/discovery/scan/<job_id>
    if req.method == "GET" && req.path.starts_with("/api/discovery/scan/") {
        let job_id = req.path.trim_start_matches("/api/discovery/scan/").to_string();
        let body = api_scan_status(&job_id, &jobs);
        return respond_json(stream, "200 OK", body);
    }

    // Route: GET /api/device/<ip>  (IP に . を含むためパスマッチで処理)
    if req.method == "GET" && req.path.starts_with("/api/device/") {
        let ip = req.path.trim_start_matches("/api/device/").to_string();
        let body = api_device_detail(&ip, &repo, &cfg);
        return respond_json(stream, "200 OK", body);
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
        return respond_html(stream, page_diagnostics(&req.path, &cfg));
    }

    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") | ("GET", "/dashboard") => {
            respond_html(stream, page_dashboard(&repo, &cfg))
        }
        ("GET", "/discovery") => {
            respond_html(stream, page_discovery(&repo))
        }
        ("GET", "/diagnostics") => {
            respond_html(stream, page_diagnostics("/diagnostics", &cfg))
        }
        ("GET", "/settings") => {
            respond_html(stream, page_settings(&cfg))
        }
        ("GET", "/api/summary") => {
            respond_json(stream, "200 OK", api_summary(&repo))
        }
        ("GET", "/api/devices") => {
            respond_json(stream, "200 OK", api_devices(&repo, &cfg))
        }
        ("GET", "/api/settings") => {
            respond_json(stream, "200 OK", api_get_settings(&cfg))
        }
        ("POST", "/api/settings") => {
            respond_json(stream, "200 OK", api_post_settings(&req.body, &cfg, &cfg_path))
        }
        ("POST", "/api/diagnostics/oids") => {
            respond_json(stream, "200 OK", api_post_oid_overrides(&req.body, &cfg, &cfg_path))
        }
        ("POST", "/api/discovery/scan") => {
            respond_json(stream, "200 OK", api_scan_start(&req.body, jobs))
        }
        ("POST", "/api/discovery/register") => {
            respond_json(stream, "200 OK", api_register(&req.body, &repo))
        }
        _ => {
            respond_json(stream, "404 Not Found", r#"{"error":"not found"}"#.to_string())
        }
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
        return ("500 Internal Server Error".to_string(), r#"{"error":"repository lock failed"}"#.to_string());
    };

    match repo.remove_device_by_ip(ip) {
        Ok(()) => ("200 OK".to_string(), r#"{"ok":true}"#.to_string()),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("was not found") {
                ("404 Not Found".to_string(), format!(r#"{{"error":"{}"}}"#, escape_json(&msg)))
            } else {
                ("400 Bad Request".to_string(), format!(r#"{{"error":"{}"}}"#, escape_json(&msg)))
            }
        }
    }
}

fn api_device_detail(ip: &str, repo: &Arc<Mutex<Repository>>, _cfg: &Arc<Mutex<AppConfig>>) -> String {
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

    let interface_spikes = r.get_recent_interface_spikes(device_id, 30).unwrap_or_default();
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
        let recent = r.get_recent_interface_samples(device_id, s.if_index, 2).unwrap_or_default();
        let prev = recent.get(1);
        let in_errors_delta = prev.map(|p| s.in_errors.saturating_sub(p.in_errors)).unwrap_or(0);
        let out_errors_delta = prev.map(|p| s.out_errors.saturating_sub(p.out_errors)).unwrap_or(0);
        let in_discards_delta = prev.map(|p| s.in_discards.saturating_sub(p.in_discards)).unwrap_or(0);
        let out_discards_delta = prev.map(|p| s.out_discards.saturating_sub(p.out_discards)).unwrap_or(0);
        format!(
            r#"{{"if_index":{},"if_name":"{}","link_status":"{}","in_errors":{},"out_errors":{},"in_discards":{},"out_discards":{},"in_errors_delta":{},"out_errors_delta":{},"in_discards_delta":{},"out_discards_delta":{},"bandwidth":{:.2},"sampled_at":"{}"}}"#,
            s.if_index, escape_json(&s.if_name), escape_json(&s.link_status),
            s.in_errors, s.out_errors, s.in_discards, s.out_discards,
            in_errors_delta, out_errors_delta, in_discards_delta, out_discards_delta,
            s.bandwidth_utilization, escape_json(&s.sampled_at),
        )
    }).collect();

    // インターフェース時系列（直近 48 件 = 約24時間 / 30秒ポーリング → 実際は1440件/24hだが表示は間引き）
    let if_history = r.get_interface_history(device_id, 288).unwrap_or_default();
    // if_index ごとに分けてグラフデータにする
    let mut if_series: std::collections::HashMap<i32, Vec<String>> = std::collections::HashMap::new();
    for s in &if_history {
        let entry = if_series.entry(s.if_index).or_default();
        entry.push(format!(
            r#"{{"t":"{}","in_err":{},"out_err":{},"in_dis":{},"out_dis":{},"bw":{:.2}}}"#,
            escape_json(&s.sampled_at), s.in_errors, s.out_errors, s.in_discards, s.out_discards, s.bandwidth_utilization
        ));
    }
    let mut if_series_json: Vec<(i32, String)> = if_series.iter().map(|(idx, pts)| {
        (*idx, format!(r#"{{"if_index":{},"points":[{}]}}"#, idx, pts.join(",")))
    }).collect();
    if_series_json.sort_by_key(|(idx, _)| *idx);

    // CPU履歴（直近 288 件 = 約2.4時間 @ 30s）
    let metrics = r.get_device_metrics_history(device_id, 288).unwrap_or_default();
    let metrics_json: Vec<String> = metrics.iter().map(|m| {
        let cpu = m.cpu_usage.filter(|v| *v <= 100).map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        format!(r#"{{"t":"{}","cpu":{}}}"#, escape_json(&m.sampled_at), cpu)
    }).collect();

    // アラート履歴（直近 30 件）
    let alerts = r.get_alert_history(device_id, 30).unwrap_or_default();
    let alerts_json: Vec<String> = alerts.iter().map(|a| {
        let interface = extract_interface_from_alert(&a.alert_type, &a.details);
        format!(
            r#"{{"type":"{}","severity":"{}","details":"{}","interface":"{}","at":"{}"}}"#,
            escape_json(&a.alert_type),
            escape_json(&a.severity),
            escape_json(&a.details),
            escape_json(&interface),
            escape_json(a.created_at.as_deref().unwrap_or("")),
        )
    }).collect();

    let spikes_json: Vec<String> = interface_spikes.iter().map(|s| {
        format!(
            r#"{{"if_index":{},"if_name":"{}","link_status":"{}","in_errors_delta":{},"out_errors_delta":{},"in_discards_delta":{},"out_discards_delta":{},"total_delta":{},"latest_sampled_at":"{}","previous_sampled_at":"{}"}}"#,
            s.if_index,
            escape_json(&s.if_name),
            escape_json(&s.link_status),
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
            r#"{{"index":{},"name":"{}","sensor_type":"{}","source":"{}","oid":"{}","value":{},"unit":"{}","status":{},"is_alarm":{}}}"#,
            s.index,
            escape_json(&s.name),
            escape_json(&s.sensor_type),
            escape_json(&s.source),
            escape_json(&s.oid),
            value,
            escape_json(unit),
            status,
            s.is_alarm,
        )
    }).collect();

    format!(
        r#"{{"ip":"{}","name":"{}","status":"{}","community":"{}","last_seen":"{}","interfaces":[{}],"if_series":[{}],"metrics":[{}],"alerts":[{}],"spikes":[{}],"hardware_sensors":[{}]}}"#,
        escape_json(&device.ip), escape_json(&device.name), escape_json(&device.status),
        escape_json(&device.community), escape_json(device.last_seen_at.as_deref().unwrap_or("")),
        ifaces_json.join(","), if_series_json.into_iter().map(|(_, s)| s).collect::<Vec<_>>().join(","), metrics_json.join(","), alerts_json.join(","), spikes_json.join(","), sensors_json.join(","),
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

fn api_post_settings(body: &str, cfg: &Arc<Mutex<AppConfig>>, cfg_path: &std::path::Path) -> String {
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
        if v < 5 || v > 600 {
            return r#"{"error":"interval_seconds must be 5-600"}"#.to_string();
        }
        c.polling.interval_seconds = v;
    }
    if let Some(v) = parse_str_field(body, "default_community") {
        if !v.is_empty() { c.snmp.default_community = v; }
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
        return r#"{"error":"warning threshold must be greater than critical threshold"}"#.to_string();
    }

    if let Err(e) = persist_config(&c, cfg_path) {
        return format!(r#"{{"error":"failed to write config: {}"}}"#, escape_json(&e));
    }

    r#"{"ok":true}"#.to_string()
}

fn api_post_oid_overrides(body: &str, cfg: &Arc<Mutex<AppConfig>>, cfg_path: &std::path::Path) -> String {
    fn parse_str_field(body: &str, key: &str) -> Option<String> {
        let pat = format!("\"{}\":\"", key);
        let start = body.find(&pat)? + pat.len();
        let end = body[start..].find('"')?;
        Some(body[start..start + end].to_string())
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

    if let Err(e) = persist_config(&c, cfg_path) {
        return format!(r#"{{"error":"failed to write config: {}"}}"#, escape_json(&e));
    }

    r#"{"ok":true}"#.to_string()
}

fn persist_config(c: &AppConfig, cfg_path: &std::path::Path) -> Result<(), String> {
    let toml_content = format!(
        "[polling]\ninterval_seconds = {}\n\n[snmp]\ndefault_community = \"{}\"\ncpu_oid_override = \"{}\"\nmemory_oid_override = \"{}\"\n\n[display]\ntimezone = \"{}\"\n\n[alert]\nerror_rate_threshold = {}\nspike_threshold = {}\nhealth_warning_threshold = {}\nhealth_critical_threshold = {}\n\n[retention]\nhistory_days = {}\n",
        c.polling.interval_seconds,
        c.snmp.default_community,
        c.snmp.cpu_oid_override,
        c.snmp.memory_oid_override,
        c.display.timezone,
        c.alert.error_rate_threshold,
        c.alert.spike_threshold,
        c.alert.health_warning_threshold,
        c.alert.health_critical_threshold,
        c.retention.history_days,
    );

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
fn api_scan_start(body: &str, jobs: JobStore) -> String {
    let params = parse_form(body);
    let cidr = params.get("cidr").cloned().unwrap_or_default();
    let community = params.get("community").cloned().unwrap_or_else(|| "public".to_string());
    // max_hosts はクライアントが CIDR から計算して送る実際のホスト数。上限なし（/8 チェックはクライアント側で実施）
    let max_hosts: usize = params.get("max_hosts").and_then(|s| s.parse().ok()).unwrap_or(65534);

    if cidr.is_empty() {
        return r#"{"error":"cidr is required"}"#.to_string();
    }

    // Pre-validate and count hosts (サーバー側は max_hosts をそのまま使う)
    let host_count = match crate::device::discovery::enumerate_cidr_hosts(&cidr) {
        Ok(hosts) => hosts.len().min(max_hosts),
        Err(err) => return format!(r#"{{"error":"{}"}}"#, escape_json(&err.to_string())),
    };

    let job_id = format!("{:x}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos());

    // Register job as Running
    {
        let mut store = jobs.lock().unwrap();
        store.insert(job_id.clone(), ScanJob::new(host_count));
        // Evict old jobs (keep at most 16)
        if store.len() > 16 {
            if let Some(oldest) = store.keys()
                .min_by_key(|k| store[*k].started_at)
                .cloned()
            {
                store.remove(&oldest);
            }
        }
    }

    // Spawn background scan thread
    let job_id_thread = job_id.clone();
    std::thread::spawn(move || {
        run_scan_job(job_id_thread, cidr, community, max_hosts, jobs);
    });

    format!(r#"{{"job_id":"{job_id}","total":{host_count}}}"#)
}

fn run_scan_job(job_id: String, cidr: String, community: String, max_hosts: usize, jobs: JobStore) {
    use crate::device::discovery::enumerate_cidr_hosts;
    use crate::snmp::SnmpClient;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const CONCURRENCY: usize = 256;

    let hosts = match enumerate_cidr_hosts(&cidr) {
        Ok(h) => h,
        Err(err) => {
            if let Ok(mut store) = jobs.lock() {
                if let Some(job) = store.get_mut(&job_id) {
                    job.state = ScanState::Error(err.to_string());
                }
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
                    if let Ok(mut store) = jobs2.lock() {
                        if let Some(job) = store.get_mut(&job_id2) {
                            job.found.push(device);
                        }
                    }
                }
                let done = counter2.fetch_add(1, Ordering::Relaxed) + 1;
                if let Ok(mut store) = jobs2.lock() {
                    if let Some(job) = store.get_mut(&job_id2) {
                        job.scanned = done;
                    }
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
    if let Ok(mut store) = jobs.lock() {
        if let Some(job) = store.get_mut(&job_id) {
            job.scanned = total;
            job.state = ScanState::Done;
        }
    }
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
            let items: Vec<String> = job.found.iter().map(|d| {
                format!(
                    r#"{{"ip":"{}","name":"{}","status":"{}","community":"{}"}}"#,
                    escape_json(&d.ip),
                    escape_json(&d.name),
                    escape_json(&d.status),
                    escape_json(&d.community),
                )
            }).collect();
            format!(
                r#"{{"status":"done","scanned":{},"total":{},"elapsed":{elapsed},"devices":[{}]}}"#,
                job.scanned, job.total, items.join(",")
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

/// POST /api/discovery/register
fn api_register(body: &str, repo: &Arc<Mutex<Repository>>) -> String {
    let devices = parse_register_body(body);
    if devices.is_empty() {
        return r#"{"error":"no devices provided"}"#.to_string();
    }

    let Ok(repo) = repo.lock() else {
        return r#"{"error":"repository lock failed"}"#.to_string();
    };

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
                match repo.save_device_config(&device) {
                    Ok(_) => registered.push(device.ip.clone()),
                    Err(err) => errors.push(format!("{}: {}", device.ip, err)),
                }
            }
            Err(err) => errors.push(format!("{}: {}", device.ip, err)),
        }
    }

    let reg_json: Vec<String> = registered.iter().map(|s| format!("\"{}\"", escape_json(s))).collect();
    let skip_json: Vec<String> = skipped.iter().map(|s| format!("\"{}\"", escape_json(s))).collect();
    let err_json: Vec<String> = errors.iter().map(|s| format!("\"{}\"", escape_json(s))).collect();

    format!(
        r#"{{"registered":[{}],"skipped":[{}],"errors":[{}]}}"#,
        reg_json.join(","),
        skip_json.join(","),
        err_json.join(","),
    )
}

// ─── Pages ───────────────────────────────────────────────────────────────────

fn page_dashboard(repo: &Arc<Mutex<Repository>>, cfg: &Arc<Mutex<AppConfig>>) -> String {
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
    let spike_card_cls = if spikes_count > 0 { "card card-spike" } else { "card" };

    let mut html = String::new();
    html.push_str(HTML_DOCTYPE);
    html.push_str("<html lang='en'><head>");
    html.push_str("<meta charset='utf-8'>");
    html.push_str("<meta name='viewport' content='width=device-width,initial-scale=1'>");
    html.push_str(THEME_BOOTSTRAP_JS);
    html.push_str("<title>TracePulse \u{2013} Dashboard</title>");
    html.push_str(COMMON_CSS);
    html.push_str(DASHBOARD_CSS);
    html.push_str("</head><body data-title-key='dashboard_title'>");
    html.push_str(NAV_HTML);
    html.push_str("<main><h1 data-i18n='dashboard_title'>Dashboard</h1><div id='summary-cards' class='summary-cards'>");
    html.push_str(&format!("<div class='card card-online'><div class='card-num'>{healthy}</div><div class='card-label' data-i18n='online'>Online</div></div>"));
    html.push_str(&format!("<div class='card card-warning'><div class='card-num'>{warning}</div><div class='card-label' data-i18n='warning'>Warning</div></div>"));
    html.push_str(&format!("<div class='card card-offline'><div class='card-num'>{offline}</div><div class='card-label' data-i18n='offline'>Offline</div></div>"));
    html.push_str(&format!("<div class='card'><div class='card-num'>{total}</div><div class='card-label' data-i18n='total'>Total</div></div>"));
    html.push_str(&format!("<div class='{spike_card_cls}'><div class='card-num'>{spikes_count}</div><div class='card-label' data-i18n='error_spikes'>Error Spikes</div></div>"));
    html.push_str("</div>");
    html.push_str("<p id='last-refreshed' style='font-size:.8rem;color:#64748b;margin-bottom:.75rem;text-align:right'></p>");
    html.push_str("<table id='dash-table'><thead><tr>");
    html.push_str("<th id='th-ip' onclick='sortBy(\"ip\")' data-i18n='ip_address'>IP <span class='sort-icon' id='sort-ip'></span></th>");
    html.push_str("<th id='th-name' onclick='sortBy(\"name\")' data-i18n='hostname'>Name <span class='sort-icon' id='sort-name'></span></th>");
    html.push_str("<th id='th-status' onclick='sortBy(\"status\")' data-i18n='status'>Status <span class='sort-icon' id='sort-status'></span></th>");
    html.push_str("<th id='th-community' onclick='sortBy(\"community\")' data-i18n='community'>Community <span class='sort-icon' id='sort-community'></span></th>");
    html.push_str("<th id='th-last_seen' onclick='sortBy(\"last_seen\")' data-i18n='time'>Last Seen <span class='sort-icon' id='sort-last_seen'></span></th>");
    html.push_str("<th data-i18n='actions'>Actions</th>");
    html.push_str("</tr></thead><tbody id='dash-tbody'></tbody></table>");
    html.push_str("<section class='detail-section' style='margin-top:1.25rem'>");
    html.push_str("<h2>Recent Alerts</h2>");
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
                "<tr><td>{}</td><td><a href='/device/{}'>{}</a></td><td>{}</td><td><span class='{}'>{}</span></td><td>{}</td></tr>",
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
    html.push_str(I18N_JS);
    html.push_str(DASHBOARD_JS);
    html.push_str("</script></body></html>");
    html
}

fn page_settings(cfg: &Arc<Mutex<AppConfig>>) -> String {
    let mut html = String::new();
    html.push_str(HTML_DOCTYPE);
    html.push_str("<html lang='en'><head>");
    html.push_str("<meta charset='utf-8'>");
    html.push_str("<meta name='viewport' content='width=device-width,initial-scale=1'>");
    html.push_str(THEME_BOOTSTRAP_JS);
    html.push_str("<title>TracePulse \u{2013} Settings</title>");
    html.push_str(COMMON_CSS);
    html.push_str(SETTINGS_CSS);
    html.push_str("</head><body data-title-key='settings_title'>");
    html.push_str(NAV_HTML);
    html.push_str(SETTINGS_HTML);
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
    html.push_str(I18N_JS);
    html.push_str(SETTINGS_JS);
    html.push_str("</script></body></html>");
    html
}

fn page_device_detail(ip: &str, repo: &Arc<Mutex<Repository>>) -> String {
    let mut html = String::new();
    html.push_str(HTML_DOCTYPE);
    html.push_str("<html lang='en'><head>");
    html.push_str("<meta charset='utf-8'>");
    html.push_str("<meta name='viewport' content='width=device-width,initial-scale=1'>");
    html.push_str(THEME_BOOTSTRAP_JS);
    html.push_str(&format!("<title>TracePulse \u{2013} {}</title>", escape_json(ip)));
    html.push_str(COMMON_CSS);
    html.push_str(DEVICE_DETAIL_CSS);
    html.push_str("</head><body data-title-key='device_title_prefix'>");
    html.push_str(NAV_HTML);

    // 404 check
    let exists = repo.lock().ok()
        .and_then(|r| r.find_device_by_ip(ip).ok())
        .flatten()
        .is_some();

    if !exists {
        html.push_str("<main><p style='color:#f87171;margin-top:2rem'>Device not found.</p></main>");
        html.push_str("</body></html>");
        return html;
    }

    html.push_str(&format!(
        "<main><div style='display:flex;align-items:center;gap:1rem;margin-bottom:1rem;flex-wrap:wrap'>\
         <a href='/' style='color:#94a3b8;font-size:.85rem;text-decoration:none' data-i18n='nav_dashboard'>&#8592; Dashboard</a>\
         <h1 id='dev-title' style='margin:0'>Loading...</h1>\
         <span id='dev-status' class='status-unknown'>unknown</span>\
         <span id='device-refresh' style='font-size:.8rem;color:#64748b;margin-left:auto'>Auto refresh: 30s</span>\
         </div>",
    ));

    // セクション: インターフェース一覧
    html.push_str("<section class='detail-section'>");
    html.push_str("<h2 data-i18n='interface_status'>Interface Status</h2>");
    html.push_str("<div class='table-scroll'><table id='if-table'><thead><tr>\
        <th>#</th><th data-i18n='interface'>Interface</th><th data-i18n='status'>Link</th>\
        <th data-i18n='in_errors'>In Errors</th><th data-i18n='out_errors'>Out Errors</th>\
        <th data-i18n='in_discards'>In Discards</th><th data-i18n='out_discards'>Out Discards</th>\
        <th><span data-i18n='bandwidth_line1'>Bandwidth</span><br><span data-i18n='bandwidth_line2'>Utilization (%)</span></th><th data-i18n='time'>Sampled At</th>\
        </tr></thead><tbody id='if-tbody'><tr><td colspan=9>Loading...</td></tr></tbody></table></div>");
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
    html.push_str(&format!("<script>\nvar DEVICE_IP = '{}';\n", escape_json(ip)));
    html.push_str(I18N_JS);
    html.push_str(DEVICE_DETAIL_JS);
    html.push_str("</script></body></html>");
    html
}

fn page_discovery(repo: &Arc<Mutex<Repository>>) -> String {
    let existing_ips: Vec<String> = repo.lock().ok()
        .and_then(|r| r.list_devices().ok())
        .unwrap_or_default()
        .into_iter()
        .map(|d| d.ip)
        .collect();

    let existing_json: Vec<String> = existing_ips.iter()
        .map(|ip| format!("\"{}\"", escape_json(ip)))
        .collect();

    let existing_set = format!("[{}]", existing_json.join(","));

    let mut html = String::new();
    html.push_str(HTML_DOCTYPE);
    html.push_str("<html lang='en'><head>");
    html.push_str("<meta charset='utf-8'>");
    html.push_str("<meta name='viewport' content='width=device-width,initial-scale=1'>");
    html.push_str(THEME_BOOTSTRAP_JS);
    html.push_str("<title>TracePulse \u{2013} Discovery</title>");
    html.push_str(COMMON_CSS);
    html.push_str(DISCOVERY_CSS);
    html.push_str("</head><body data-title-key='discovery_title'>");
    html.push_str(NAV_HTML);
    html.push_str(DISCOVERY_HTML);
    html.push_str("<script>\nconst EXISTING = new Set(");
    html.push_str(&existing_set);
    html.push_str(");\n");
    html.push_str(I18N_JS);
    html.push_str(DISCOVERY_JS);
    html.push_str("</script></body></html>");
    html
}

fn page_diagnostics(path: &str, cfg: &Arc<Mutex<AppConfig>>) -> String {
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
    let ip = params.get("ip").cloned().unwrap_or_default();
    let community = params.get("community").cloned().unwrap_or_else(|| default_community.clone());

    fn fmt_opt(value: Option<u32>) -> String {
        value.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string())
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

    fn render_hardware_sensors(sensors: &[crate::snmp::SnmpHardwareSensor]) -> String {
        if sensors.is_empty() {
            return "<tr><td colspan='6' class='diag-empty'>No Sensors Detected</td></tr>".to_string();
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
            return "<tr><td colspan='4' class='diag-empty'>No hardware candidates</td></tr>".to_string();
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
                let vendor = report.vendor_name.clone().unwrap_or_else(|| "Unknown".to_string());
                let enterprise = report.enterprise_id.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string());
                let sys_object_id = report.sys_object_id.clone().unwrap_or_else(|| "N/A".to_string());

                result_html.push_str("<section class='settings-section'>");
                result_html.push_str("<h2 data-i18n='diagnostics_title'>SNMP Diagnostics</h2>");
                result_html.push_str("<div class='diag-grid'>");
                result_html.push_str("<div class='diag-card'>");
                result_html.push_str("<h3 data-i18n='target_device'>Target Device</h3>");
                result_html.push_str("<dl class='diag-kv'>");
                result_html.push_str(&format!("<dt data-i18n='ip_address'>IP</dt><dd>{}</dd>", escape_html(&ip)));
                result_html.push_str(&format!("<dt data-i18n='hostname'>sysName</dt><dd>{}</dd>", escape_html(&report.sys_name)));
                result_html.push_str(&format!("<dt>sysDescr</dt><dd>{}</dd>", escape_html(&report.sys_descr)));
                result_html.push_str(&format!("<dt data-i18n='sys_object_id'>sysObjectID</dt><dd><code>{}</code></dd>", escape_html(&sys_object_id)));
                result_html.push_str(&format!("<dt data-i18n='enterprise_id'>Enterprise ID</dt><dd>{}</dd>", escape_html(&enterprise)));
                result_html.push_str(&format!("<dt data-i18n='vendor'>Vendor</dt><dd>{}</dd>", escape_html(&vendor)));
                result_html.push_str("</dl>");
                result_html.push_str("</div>");
                result_html.push_str("<div class='diag-card'>");
                result_html.push_str("<h3>Summary</h3>");
                result_html.push_str("<dl class='diag-kv'>");
                result_html.push_str(&format!("<dt data-i18n='cpu_usage_title'>CPU Usage (%)</dt><dd>{}</dd>", cpu_value));
                result_html.push_str(&format!("<dt data-i18n='if_index_list'>ifIndex Count</dt><dd>{}</dd>", report.interfaces.len()));
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
                result_html.push_str("<h3 data-i18n='hardware_candidates'>Hardware Candidates</h3>");
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
    html.push_str(THEME_BOOTSTRAP_JS);
    html.push_str("<title>TracePulse \u{2013} Diagnostics</title>");
    html.push_str(COMMON_CSS);
    html.push_str(SETTINGS_CSS);
    html.push_str(DIAGNOSTICS_CSS);
    html.push_str("</head><body data-title-key='diagnostics_title'>");
    html.push_str(NAV_HTML);
    html.push_str("<main>");
    html.push_str("<h1 data-i18n='diagnostics_title'>SNMP Diagnostics</h1>");
    html.push_str("<div class='settings-section'>");
    html.push_str("<h2 data-i18n='target_device'>Target Device</h2>");
    html.push_str("<form method='GET' action='/diagnostics' onsubmit='return startDiagnostics(event)'>");
    html.push_str("<div class='field-row'>");
    html.push_str("<div class='field-group'>");
    html.push_str("<label data-i18n='ip_address'>IP Address</label>");
    html.push_str(&format!("<input type='text' name='ip' value='{}' placeholder='192.168.1.1'>", escape_html(&ip)));
    html.push_str("</div>");
    html.push_str("<div class='field-group'>");
    html.push_str("<label data-i18n='community'>Community</label>");
    html.push_str(&format!("<input type='text' name='community' value='{}' placeholder='public'>", escape_html(&community)));
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
    html.push_str(&format!("<input type='text' id='cpu-oid-override' value='{}' placeholder='1.3.6.1.4.1....'>", escape_html(&saved_cpu_oid)));
    html.push_str("</div>");
    html.push_str("</div>");
    html.push_str("<div class='actions'>");
    html.push_str("<button class='btn btn-primary' type='button' onclick='saveOidOverrides()' data-i18n='save_oid_overrides'>OID の手動保存</button>");
    html.push_str("<span id='oid-save-result' class='diag-note'></span>");
    html.push_str("</div>");
    html.push_str("</div>");
    html.push_str(&result_html);
    html.push_str("</main>");
    html.push_str("<script>\n");
    html.push_str(I18N_JS);
    html.push_str(DIAGNOSTICS_JS);
    html.push_str("</script></body></html>");
    html
}

// ─── Shared HTML components ───────────────────────────────────────────────────

const HTML_DOCTYPE: &str = "<!doctype html>";
const THEME_BOOTSTRAP_JS: &str = "<script>try{document.documentElement.dataset.theme=localStorage.getItem('tracepulse-theme')||'dark';}catch(e){document.documentElement.dataset.theme='dark';}</script>";

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
</style>";

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

const I18N_JS: &str = r"
var I18N = {
  en: {
    nav_dashboard: 'Dashboard',
    nav_discovery: 'Discovery',
    nav_diagnostics: 'Diagnostics',
    nav_settings: 'Settings',
    theme_label: 'Theme',
    theme_dark: 'Dark',
    theme_light: 'Light',
    language_label: 'Language',
    english: 'English',
    japanese: 'Japanese',
    dashboard_title: 'Dashboard',
    discovery_title: 'Device Discovery',
    diagnostics_title: 'SNMP Diagnostics',
    settings_title: 'Settings',
    device_title_prefix: 'Device',
    interface_status: 'Interface Status',
    interface_filter_title: 'Interface Selection',
    select_all_interfaces: 'Select All',
    clear_all_interfaces: 'Clear All',
    bandwidth_title: 'Bandwidth Utilization (%)',
    bandwidth_line1: 'Bandwidth',
    bandwidth_line2: 'Utilization (%)',
    error_discard_title: 'Error & Discard Counters',
    cpu_memory_title: 'CPU & Memory Usage (%)',
    cpu_usage_title: 'CPU Usage (%)',
    memory_usage_title: 'Memory Usage (%)',
    recent_spikes_title: 'Recent Interface Spikes',
    alert_history: 'Alert History',
    polling: 'Polling',
    snmp: 'SNMP',
    display: 'Display',
    time_zone: 'Time Zone',
    time_zone_hint: 'Controls how timestamps are shown in the web UI',
    timezone_utc: 'UTC',
    timezone_jst: 'JST',
    alert_thresholds: 'Alert Thresholds',
    data_retention: 'Data Retention',
    polling_interval_label: 'Polling Interval (seconds)',
    polling_interval_hint: 'Range: 5 – 600 seconds (default: 30)',
    default_community: 'Default Community String',
    public_placeholder: 'public',
    default_community_hint: 'Used when no community is specified per device',
    error_rate_threshold: 'Error Rate Threshold (0.0 – 1.0)',
    error_rate_hint: 'e.g. 0.05 = alert when error rate exceeds 5%',
    spike_threshold: 'Spike Threshold (error count delta)',
    spike_threshold_hint: 'Alert when error count jumps by this amount between polls',
    warning_threshold: 'Health Score — Warning threshold (0 – 100)',
    warning_threshold_hint: 'Score below this value turns status yellow (Warning)',
    critical_threshold: 'Health Score — Critical threshold (0 – 100)',
    critical_threshold_hint: 'Score below this value turns status red (Critical)',
    score_preview: 'Score preview',
    score_formula: 'Health score (0–100) = 100 − error_rate×1.5 − bandwidth×0.3 − cpu/3 − memory/3',
    history_retention: 'History Retention (days)',
    history_retention_hint: 'Polling history older than this is purged automatically',
    save_settings: 'Save Settings',
    reset_defaults: 'Reset to Defaults',
    settings_saved: 'Settings saved',
    device_discovery: 'Device Discovery',
    cidr_range: 'CIDR Range',
    cidr_range_placeholder: '192.168.1.0/24',
    community: 'Community',
    public_placeholder: 'public',
    scan: 'Scan',
    cancel: 'Cancel',
    add_manually: '+ Add manually',
    add_device_manually: 'Add Device Manually',
    ip_address: 'IP Address',
    ip_address_placeholder: '192.168.1.1',
    name: 'Name',
    name_placeholder: 'router-01',
    add: 'Add',
    registration: 'Registration',
    hostname: 'Hostname',
    status: 'Status',
    in_errors: 'In Errors',
    out_errors: 'Out Errors',
    in_discards: 'In Discards',
    out_discards: 'Out Discards',
    registered: 'Registered',
    select_all: 'Select all',
    scanning: 'Scanning…',
    elapsed: 'Elapsed',
    found: 'found',
    scan_results_label: 'Scan Results',
    register: 'Register',
    selected: 'selected',
    time: 'Time',
    type: 'Type',
    severity: 'Severity',
    details: 'Details',
    interface: 'Interface',
    online: 'Online',
    warning: 'Warning',
    offline: 'Offline',
    total: 'Total',
    error_spikes: 'Error Spikes',
    updated: 'Updated',
    no_data: 'No data yet',
    no_devices: 'No devices registered yet.',
    no_interface_data: 'No interface data collected yet',
    no_interface_spikes: 'No interface spikes recorded',
    no_alerts: 'No alerts recorded',
    actions: 'Actions',
    scan_results: 'Scan Results',
    run_diagnostics: 'Run Diagnostics',
    diagnostics_running: 'Diagnosing…',
    diagnostics_help: 'Check sysObjectID, vendor presets, CPU / memory candidates, and ifIndex mappings.',
    target_device: 'Target Device',
    oid_overrides: 'OID Overrides',
    cpu_oid_override: 'CPU OID Override',
    memory_oid_override: 'Memory OID Override',
    save_oid_overrides: 'Save OID Overrides',
    unregister: 'Unregister',
    unregistering: 'Unregistering…',
    unregister_confirm: 'Remove this device from TracePulse?',
    unregister_success: 'Device unregistered',
    unregister_failed: 'Failed to unregister device',
    cpu_candidates: 'CPU Candidates',
    hardware_candidates: 'Hardware Candidates',
    hardware_sensors: 'Hardware Sensors',
    hardware_status_title: 'Hardware Status',
    memory_candidates: 'Memory Candidates',
    if_index_list: 'ifIndex List',
    sys_object_id: 'sysObjectID',
    enterprise_id: 'Enterprise ID',
    vendor: 'Vendor',
    probe: 'Probe',
    value: 'Value',
    link_status: 'Link',
    no_snmp_devices_found: 'No SNMP-responding devices found',
    registering: 'Registering…',
    select_at_least_one_device: 'Select at least one device.',
    request_failed: 'Request failed',
    register_selected: 'Register Selected'
  },
  ja: {
    nav_dashboard: 'ダッシュボード',
    nav_discovery: '探索',
    nav_diagnostics: '診断',
    nav_settings: '設定',
    theme_label: 'テーマ',
    theme_dark: 'ダーク',
    theme_light: 'ライト',
    language_label: 'Language',
    english: '英語',
    japanese: '日本語',
    dashboard_title: 'ダッシュボード',
    discovery_title: 'デバイス探索',
    diagnostics_title: 'SNMP 診断',
    settings_title: '設定',
    device_title_prefix: '機器',
    interface_status: 'インターフェース状態',
    interface_filter_title: 'インターフェース選択',
    select_all_interfaces: '全選択',
    clear_all_interfaces: '全解除',
    bandwidth_title: '帯域利用率 (%)',
    bandwidth_line1: '帯域',
    bandwidth_line2: '利用率 (%)',
    error_discard_title: 'エラー / ディスカード',
    cpu_memory_title: 'CPU / メモリ使用率 (%)',
    cpu_usage_title: 'CPU 使用率 (%)',
    memory_usage_title: 'メモリ使用率 (%)',
    recent_spikes_title: '最近のインターフェーススパイク',
    alert_history: 'アラート履歴',
    polling: 'ポーリング',
    snmp: 'SNMP',
    display: '表示',
    time_zone: '時刻設定',
    time_zone_hint: 'Web UI 上の時刻表示を切り替えます',
    timezone_utc: 'UTC',
    timezone_jst: 'JST',
    alert_thresholds: 'アラート閾値',
    data_retention: 'データ保持',
    polling_interval_label: 'ポーリング間隔（秒）',
    polling_interval_hint: '範囲: 5 ～ 600 秒（デフォルト: 30）',
    default_community: 'デフォルトコミュニティ文字列',
    public_placeholder: 'public',
    default_community_hint: '各デバイスで community 未指定時に使用',
    error_rate_threshold: 'エラー率閾値 (0.0 – 1.0)',
    error_rate_hint: '例: 0.05 = エラー率が 5% を超えたら通知',
    spike_threshold: 'スパイク閾値（エラーカウント差分）',
    spike_threshold_hint: 'ポーリング間のエラーカウント増分がこの値を超えると通知',
    warning_threshold: 'ヘルススコア — Warning 閾値（0 – 100）',
    warning_threshold_hint: 'この値を下回ると黄色 (Warning)',
    critical_threshold: 'ヘルススコア — Critical 閾値（0 – 100）',
    critical_threshold_hint: 'この値を下回ると赤色 (Critical)',
    score_preview: 'スコア表示',
    score_formula: 'ヘルススコア (0–100) = 100 − error_rate×1.5 − bandwidth×0.3 − cpu/3 − memory/3',
    history_retention: '履歴保持期間（日）',
    history_retention_hint: '指定日数より古いポーリング履歴は自動削除',
    save_settings: '設定を保存',
    reset_defaults: 'デフォルトに戻す',
    settings_saved: '設定を保存しました',
    device_discovery: 'デバイス探索',
    cidr_range: 'CIDR 範囲',
    cidr_range_placeholder: '192.168.1.0/24',
    community: 'コミュニティ',
    public_placeholder: 'public',
    scan: 'スキャン',
    cancel: 'キャンセル',
    add_manually: '+ 手動追加',
    add_device_manually: 'デバイスを手動追加',
    ip_address: 'IP アドレス',
    ip_address_placeholder: '192.168.1.1',
    name: '名前',
    name_placeholder: 'router-01',
    add: '追加',
    registration: '登録',
    hostname: 'ホスト名',
    status: '状態',
    in_errors: '入力エラー',
    out_errors: '出力エラー',
    in_discards: '入力ディスカード',
    out_discards: '出力ディスカード',
    registered: '登録済み',
    select_all: '全選択',
    scanning: 'スキャン中…',
    elapsed: '経過',
    found: '件検出',
    scan_results_label: 'スキャン結果',
    register: '登録',
    selected: '選択',
    time: '時刻',
    type: '種別',
    severity: '重要度',
    details: '詳細',
    interface: 'インターフェース',
    online: 'Online',
    warning: 'Warning',
    offline: 'Offline',
    total: '合計',
    error_spikes: 'エラー急増',
    updated: '更新',
    no_data: 'データはまだありません',
    no_devices: 'まだデバイスが登録されていません。',
    no_interface_data: 'インターフェースデータはまだ収集されていません',
    no_interface_spikes: 'インターフェーススパイクはまだ記録されていません',
    no_alerts: 'アラートはまだありません',
    actions: '操作',
    scan_results: 'スキャン結果',
    run_diagnostics: '診断を実行',
    diagnostics_running: '診断中…',
    diagnostics_help: 'sysObjectID、ベンダー候補、CPU / メモリ候補、ifIndex マッピングを確認します。',
    target_device: '対象デバイス',
    oid_overrides: 'OID 上書き',
    cpu_oid_override: 'CPU OID 上書き',
    memory_oid_override: 'メモリ OID 上書き',
    save_oid_overrides: 'OID の手動保存',
    unregister: '登録解除',
    unregistering: '解除中…',
    unregister_confirm: 'このデバイスを TracePulse から削除しますか？',
    unregister_success: 'デバイスを削除しました',
    unregister_failed: 'デバイスの削除に失敗しました',
    cpu_candidates: 'CPU 候補',
    hardware_candidates: 'Hardware 候補',
    hardware_sensors: 'Hardware センサー',
    hardware_status_title: 'Hardware ステータス',
    memory_candidates: 'メモリ候補',
    if_index_list: 'ifIndex 一覧',
    sys_object_id: 'sysObjectID',
    enterprise_id: 'Enterprise ID',
    vendor: 'ベンダー',
    probe: '候補',
    value: '値',
    link_status: 'リンク',
    no_snmp_devices_found: 'SNMP 応答のあるデバイスは見つかりませんでした',
    registering: '登録中…',
    select_at_least_one_device: '少なくとも1台の機器を選択してください。',
    request_failed: 'リクエストに失敗しました',
    register_selected: '選択した機器を登録'
  }
};

function t(key, lang) {
  lang = lang || currentLanguage();
  return (I18N[lang] && I18N[lang][key]) || (I18N.en && I18N.en[key]) || key;
}

function currentLanguage() {
  return localStorage.getItem('tracepulse-lang') || 'en';
}

function setLanguage(lang) {
  localStorage.setItem('tracepulse-lang', lang);
  applyLanguage(lang);
}

function currentTheme() {
  try { return localStorage.getItem('tracepulse-theme') || 'dark'; } catch (e) { return 'dark'; }
}

function setTheme(theme) {
  localStorage.setItem('tracepulse-theme', theme);
  applyTheme(theme);
}

function applyTheme(theme) {
  theme = theme || currentTheme();
  document.documentElement.dataset.theme = theme;
  var select = document.getElementById('theme-select');
  if (select && select.value !== theme) select.value = theme;
}

function applyLanguage(lang) {
  lang = lang || currentLanguage();
  document.documentElement.lang = lang;
  var select = document.getElementById('lang-select');
  if (select && select.value !== lang) select.value = lang;

  document.querySelectorAll('[data-i18n]').forEach(function(el) {
    el.textContent = t(el.dataset.i18n, lang);
  });

  document.querySelectorAll('[data-i18n-placeholder]').forEach(function(el) {
    el.placeholder = t(el.dataset.i18nPlaceholder, lang);
  });

  document.querySelectorAll('[data-i18n-title]').forEach(function(el) {
    el.title = t(el.dataset.i18nTitle, lang);
  });

  var titleKey = document.body && document.body.dataset.titleKey;
  if (titleKey) {
    document.title = 'TracePulse \u2013 ' + t(titleKey, lang);
  }

  if (typeof applyPageLanguage === 'function') {
    applyPageLanguage(lang);
  }
}

function currentTimeZoneSetting() {
  try { return localStorage.getItem('tracepulse-timezone') || 'utc'; } catch (e) { return 'utc'; }
}

function tracePulseTimeZone() {
  return currentTimeZoneSetting() === 'jst' ? 'Asia/Tokyo' : 'UTC';
}

function parseTracePulseTime(iso) {
  if (!iso) return new Date(NaN);
  if (/Z$|[+-]\d\d:\d\d$/.test(iso)) return new Date(iso);
  return new Date(iso.replace(' ', 'T') + 'Z');
}

function formatTracePulseTimestamp(value) {
  var d = value instanceof Date ? value : parseTracePulseTime(value);
  if (isNaN(d)) return value || '-';
  var parts = new Intl.DateTimeFormat('en-US', {
    timeZone: tracePulseTimeZone(),
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  }).formatToParts(d);
  var map = {};
  parts.forEach(function(p) { if (p.type !== 'literal') map[p.type] = p.value; });
  return map.month + '/' + map.day + ' ' + map.hour + ':' + map.minute + ':' + map.second;
}

function formatTracePulseClock(value) {
  var d = value instanceof Date ? value : parseTracePulseTime(value);
  if (isNaN(d)) return '-';
  var parts = new Intl.DateTimeFormat('en-US', {
    timeZone: tracePulseTimeZone(),
    hour: '2-digit',
    minute: '2-digit',
    hour12: false
  }).formatToParts(d);
  var map = {};
  parts.forEach(function(p) { if (p.type !== 'literal') map[p.type] = p.value; });
  return map.hour + ':' + map.minute;
}

document.addEventListener('DOMContentLoaded', function() {
  applyTheme(currentTheme());
  applyLanguage(currentLanguage());
});
";

const SETTINGS_CSS: &str = "<style>
  .settings-section { background:#1e293b; border:1px solid #334155; border-radius:.5rem; padding:1.5rem; margin-bottom:1.5rem; }
  .settings-section h2 { margin-bottom:1rem; font-size:1rem; color:#38bdf8; border-bottom:1px solid #334155; padding-bottom:.5rem; }
  .field-row { display:grid; grid-template-columns:1fr 1fr; gap:1rem; margin-bottom:1rem; }
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

<div class='actions'>
  <button class='btn btn-primary' onclick='saveSettings()' data-i18n='save_settings'>Save Settings</button>
  <button class='btn' onclick='resetDefaults()' style='background:#334155' data-i18n='reset_defaults'>Reset to Defaults</button>
  <span class='toast toast-ok' id='toast-ok' data-i18n='settings_saved'>&#10003; Settings saved</span>
  <span class='toast toast-err' id='toast-err'></span>
</div>
</main>";

const SETTINGS_JS: &str = r"
function populateForm(d) {
  document.getElementById('interval').value    = d.interval;
  document.getElementById('community').value   = d.community;
  document.getElementById('timezone').value    = d.timezone || 'utc';
  try { localStorage.setItem('tracepulse-timezone', document.getElementById('timezone').value); } catch (e) {}
  document.getElementById('error_rate').value  = d.error_rate;
  document.getElementById('spike').value       = d.spike;
  document.getElementById('warn_t').value      = d.warn;
  document.getElementById('crit_t').value      = d.crit;
  document.getElementById('days').value        = d.days;
  updateBar();
}

function updateBar() {
  var w = parseInt(document.getElementById('warn_t').value) || 80;
  var c = parseInt(document.getElementById('crit_t').value) || 60;
  var bar = document.getElementById('tbar');
  var lbl = document.getElementById('bar-label');
  bar.style.setProperty('--warn', w + '%');
  bar.style.setProperty('--crit', c + '%');
  lbl.textContent = 'crit < ' + c + ' \u2264 warn < ' + w + ' \u2264 online';
}

function showToast(ok, msg) {
  var tok = document.getElementById('toast-ok');
  var terr = document.getElementById('toast-err');
  tok.style.display = 'none';
  terr.style.display = 'none';
  if (ok) {
    tok.style.display = 'inline-block';
    setTimeout(function(){ tok.style.display='none'; }, 3000);
  } else {
    terr.textContent = '\u2717 ' + msg;
    terr.style.display = 'inline-block';
    setTimeout(function(){ terr.style.display='none'; }, 5000);
  }
}

function saveSettings() {
  var timezone = document.getElementById('timezone').value;
  var body = JSON.stringify({
    interval_seconds: parseInt(document.getElementById('interval').value),
    default_community: document.getElementById('community').value,
    timezone: timezone,
    error_rate_threshold: parseFloat(document.getElementById('error_rate').value),
    spike_threshold: parseInt(document.getElementById('spike').value),
    health_warning_threshold: parseInt(document.getElementById('warn_t').value),
    health_critical_threshold: parseInt(document.getElementById('crit_t').value),
    history_days: parseInt(document.getElementById('days').value)
  });
  fetch('/api/settings', {method:'POST', headers:{'Content-Type':'application/json'}, body:body})
    .then(function(r){ return r.json(); })
    .then(function(d){
      if (d.ok) {
        try { localStorage.setItem('tracepulse-timezone', timezone); } catch (e) {}
        showToast(true);
      } else { showToast(false, d.error || 'Unknown error'); }
    })
    .catch(function(e){ showToast(false, e.toString()); });
}

function resetDefaults() {
  populateForm({interval:30, community:'public', timezone:'utc', error_rate:0.05, spike:10, warn:80, crit:60, days:7});
}

// 初期値をサーバーから埋め込まれた INIT 変数で設定
populateForm(INIT);
";

const DIAGNOSTICS_JS: &str = r"
function startDiagnostics(event) {
  event.preventDefault();
  var form = event.target;
  var button = document.getElementById('diag-run-btn');
  var progress = document.getElementById('diag-progress');
  if (button) button.disabled = true;
  if (progress) progress.textContent = t('diagnostics_running');

  var url = form.action + '?' + new URLSearchParams(new FormData(form)).toString();
  fetch(url, { cache: 'no-store' })
    .then(function(r) { return r.text(); })
    .then(function(html) {
      document.open();
      document.write(html);
      document.close();
    })
    .catch(function(e) {
      if (progress) progress.textContent = e.toString();
      if (button) button.disabled = false;
    });
  return false;
}

function saveOidOverrides() {
  var body = JSON.stringify({
    cpu_oid_override: document.getElementById('cpu-oid-override').value || ''
  });
  fetch('/api/diagnostics/oids', {
    method: 'POST',
    headers: {'Content-Type':'application/json'},
    body: body
  })
  .then(function(r) { return r.json(); })
  .then(function(d) {
    var el = document.getElementById('oid-save-result');
    if (!el) return;
    if (d.ok) {
      el.className = 'diag-note';
      el.textContent = t('settings_saved');
      setTimeout(function() { window.location.reload(); }, 300);
    } else {
      el.textContent = d.error || 'Save failed';
      el.className = 'diag-note diag-error';
    }
  })
  .catch(function(e) {
    var el = document.getElementById('oid-save-result');
    if (el) {
      el.textContent = e.toString();
      el.className = 'diag-note diag-error';
    }
  });
}
";

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
  .row-warn td { background: rgba(245, 158, 11, .08); }
  .row-crit td { background: rgba(248, 113, 113, .10); }
  .link-up { color:#4ade80; font-weight:600; }
  .link-down { color:#f87171; font-weight:600; }
  .sev-warning { color:#f59e0b; }
  .sev-critical { color:#f87171; }
  .sev-info { color:#64748b; }
  .no-data { color:#475569; font-style:italic; padding:.5rem 0; }
  #dev-title { font-size:1.4rem; }
  #spike-table th { white-space:nowrap; }
</style>";

const DEVICE_DETAIL_JS: &str = r"
var COLORS = ['#38bdf8','#4ade80','#f59e0b','#f87171','#a78bfa','#34d399','#fb923c','#e879f9'];
var REFRESH_MS = 30000;
var TIME_ZONE = (function() { try { return localStorage.getItem('tracepulse-timezone') || 'utc'; } catch (e) { return 'utc'; } })();
var IFACE_LABELS = {};
var IFACE_SELECTED = new Set();
var LAST_DEVICE_DETAIL = null;
var IFACE_SELECTION_READY = false;

function ifaceSelectionKey() {
  return 'tracepulse-iface-selection:' + DEVICE_IP;
}

function syncSelectedInterfaces(ifaces) {
  var known = new Set((ifaces || []).map(function(f) { return String(f.if_index); }));
  var stored = null;
  if (!IFACE_SELECTION_READY) {
    try {
      stored = JSON.parse(localStorage.getItem(ifaceSelectionKey()) || 'null');
    } catch (e) {
      stored = null;
    }

    IFACE_SELECTED = new Set();
    if (Array.isArray(stored) && stored.length > 0) {
      stored.forEach(function(v) {
        var key = String(v);
        if (known.has(key)) IFACE_SELECTED.add(key);
      });
    }

    if (IFACE_SELECTED.size === 0) {
      known.forEach(function(v) { IFACE_SELECTED.add(v); });
    }
    IFACE_SELECTION_READY = true;
  } else {
    Array.from(IFACE_SELECTED).forEach(function(v) {
      if (!known.has(String(v))) IFACE_SELECTED.delete(v);
    });
  }

  saveSelectedInterfaces();
}

function saveSelectedInterfaces() {
  try {
    localStorage.setItem(ifaceSelectionKey(), JSON.stringify(Array.from(IFACE_SELECTED)));
  } catch (e) {}
}

function isInterfaceSelected(ifIndex) {
  return IFACE_SELECTED.has(String(ifIndex));
}

function selectAllInterfaces(checked) {
  var ifaces = (LAST_DEVICE_DETAIL && LAST_DEVICE_DETAIL.interfaces) || [];
  IFACE_SELECTED = new Set();
  if (checked) {
    ifaces.forEach(function(f) { IFACE_SELECTED.add(String(f.if_index)); });
  }
  saveSelectedInterfaces();
  if (LAST_DEVICE_DETAIL) {
    renderDetail(LAST_DEVICE_DETAIL);
  }
}

function toggleInterfaceSelection(ifIndex, checked) {
  ifIndex = String(ifIndex);
  if (checked) {
    IFACE_SELECTED.add(ifIndex);
  } else {
    IFACE_SELECTED.delete(ifIndex);
  }
  saveSelectedInterfaces();
  if (LAST_DEVICE_DETAIL) {
    renderDetail(LAST_DEVICE_DETAIL);
  }
}

function renderInterfaceSelection(ifaces) {
  var box = document.getElementById('iface-filter');
  var note = document.getElementById('iface-filter-note');
  if (!box) return;
  ifaces = ifaces || [];

  box.innerHTML = ifaces.map(function(f) {
    var id = 'iface-select-' + f.if_index;
    var label = f.if_name || ('if-' + f.if_index);
    var checked = isInterfaceSelected(f.if_index) ? ' checked' : '';
    return '<label class=\'iface-filter-item\' for=\'' + id + '\'><input id=\'' + id + '\' type=\'checkbox\' value=\'' + f.if_index + '\'' + checked + ' onchange=\'toggleInterfaceSelection(this.value,this.checked)\'><span>' + label + ' (if-' + f.if_index + ')</span></label>';
  }).join('');

  if (note) {
    note.textContent = ifaces.length ? (Array.from(IFACE_SELECTED).length + ' / ' + ifaces.length + ' ' + t('selected')) : ('0 / 0 ' + t('selected'));
  }
}

function selectedSeries(series, getPoints) {
  return (series || []).filter(function(s) { return isInterfaceSelected(s.if_index); }).map(getPoints);
}

function fmtTime(iso) {
  if (!iso) return '-';
  var d = parseTracePulseTime(iso);
  return isNaN(d) ? iso : formatTracePulseTime(d);
}
function pad2(n) { return n < 10 ? '0'+n : ''+n; }
function parseTracePulseTime(iso) {
  if (!iso) return new Date(NaN);
  if (/Z$|[+-]\d\d:\d\d$/.test(iso)) return new Date(iso);
  return new Date(iso.replace(' ', 'T') + 'Z');
}
function tracePulseTimeZone() {
  return TIME_ZONE === 'jst' ? 'Asia/Tokyo' : 'UTC';
}
function formatTracePulseTime(d) {
  var parts = new Intl.DateTimeFormat('en-US', {
    timeZone: tracePulseTimeZone(),
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  }).formatToParts(d);
  var map = {};
  parts.forEach(function(p) { if (p.type !== 'literal') map[p.type] = p.value; });
  return map.month + '/' + map.day + ' ' + map.hour + ':' + map.minute + ':' + map.second;
}
function formatTracePulseClock(d) {
  var parts = new Intl.DateTimeFormat('en-US', {
    timeZone: tracePulseTimeZone(),
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  }).formatToParts(d);
  var map = {};
  parts.forEach(function(p) { if (p.type !== 'literal') map[p.type] = p.value; });
  return map.hour + ':' + map.minute + ':' + map.second;
}
function fmtNum(v) {
  return new Intl.NumberFormat('en-US').format(v || 0);
}
function counterClass(delta, spikeThreshold) {
  if (delta >= spikeThreshold) return 'crit';
  if (delta > 0) return 'warn';
  return 'none';
}
function formatCounter(total, delta) {
  var totalText = '<span class=\'counter-total\'>' + fmtNum(total) + '</span>';
  if (!delta) return totalText;
  return totalText + '<span class=\'counter-delta ' + counterClass(delta, (window.TRACEPULSE_SPIKE_THRESHOLD || 10)) + '\'> (+' + fmtNum(delta) + ')</span>';
}

// ── SVG sparkline ────────────────────────────────────────────────────────────
function sparkline(svgId, series, yLabel) {
  // series: [{label, color, points:[{t,v}]}]
  var svg = document.getElementById(svgId);
  if (!svg) return;
  var W = 800, H = 140, PL = 40, PR = 8, PT = 8, PB = 20;
  var cW = W - PL - PR, cH = H - PT - PB;

  // flatten all points to find x/y range
  var allPts = [];
  series.forEach(function(s) {
    (s.points || []).forEach(function(p) {
      if (p && p.v !== null && p.v !== undefined && !isNaN(p.v)) {
        allPts.push(p);
      }
    });
  });
  if (allPts.length === 0) {
    svg.classList.add('chart-empty');
    svg.innerHTML = '<text x=\'400\' y=\'80\' text-anchor=\'middle\' fill=\'#64748b\' font-size=\'13\'>N/A</text>';
    return;
  }
  svg.classList.remove('chart-empty');

  var times = allPts.map(function(p) { return parseTracePulseTime(p.t).getTime(); });
  var vals  = allPts.map(function(p) { return p.v; });
  var tMin = Math.min.apply(null, times), tMax = Math.max.apply(null, times);
  var vMin = 0, vMax = (yLabel === '%') ? 100 : (Math.max.apply(null, vals) * 1.15 || 1);
  if (tMin === tMax) tMax = tMin + 1;

  function tx(t) { return PL + (parseTracePulseTime(t).getTime() - tMin) / (tMax - tMin) * cW; }
  function ty(v) { return PT + cH - (v - vMin) / (vMax - vMin) * cH; }
  function formatAxisValue(v) {
    if (yLabel === '%') {
      if (v < 1) return v.toFixed(2);
      if (v < 10) return v.toFixed(1);
    }
    return Math.round(v).toString();
  }

  var out = '';
  // grid lines
  for (var gi = 0; gi <= 4; gi++) {
    var gy = PT + gi * cH / 4;
    var gv = (vMax - vMin) * (1 - gi/4) + vMin;
    out += '<line x1=\''+PL+'\' y1=\''+gy+'\' x2=\''+(W-PR)+'\' y2=\''+gy+'\' stroke=\'#1e293b\' stroke-width=\'1\'/>';
    out += '<text x=\''+(PL-4)+'\' y=\''+(gy+4)+'\' text-anchor=\'end\' fill=\'#64748b\' font-size=\'9\'>'+formatAxisValue(gv)+'</text>';
  }
  // x-axis labels (4 points)
  for (var xi = 0; xi <= 3; xi++) {
    var xt = tMin + (tMax - tMin) * xi / 3;
    var xp = PL + (xt - tMin) / (tMax - tMin) * cW;
    var xl = formatTracePulseClock(new Date(xt));
    var anchor = 'middle';
    var labelX = xp;
    if (xi === 0) {
      anchor = 'start';
      labelX = PL + 2;
    } else if (xi === 3) {
      anchor = 'end';
      labelX = W - PR - 2;
    }
    out += '<text x=\''+labelX+'\' y=\''+(H-4)+'\' text-anchor=\''+anchor+'\' fill=\'#475569\' font-size=\'9\'>'+xl+'</text>';
  }
  // series lines
  series.forEach(function(s) {
    if (s.points.length === 0) return;
    var d = s.points.map(function(p, i) {
      return (i === 0 ? 'M' : 'L') + tx(p.t).toFixed(1) + ' ' + ty(p.v).toFixed(1);
    }).join(' ');
    out += '<path d=\''+d+'\' fill=\'none\' stroke=\''+s.color+'\' stroke-width=\'1.5\' stroke-linejoin=\'round\'/>';
  });
  svg.innerHTML = out;
}

// ── Render functions ──────────────────────────────────────────────────────────
function renderInterfaces(ifaces) {
  var tbody = document.getElementById('if-tbody');
  if (!tbody) return;
  if (ifaces.length === 0) {
    tbody.innerHTML = '<tr><td colspan=9 class=no-data>' + t('no_interface_data') + '</td></tr>';
    return;
  }
  var spikeThreshold = (window.TRACEPULSE_SPIKE_THRESHOLD || 10);
  tbody.innerHTML = ifaces.map(function(f) {
    var lc = f.link_status === 'up' ? 'link-up' : 'link-down';
    var errorDelta = Math.max(f.in_errors_delta || 0, f.out_errors_delta || 0);
    var discardDelta = Math.max(f.in_discards_delta || 0, f.out_discards_delta || 0);
    var rowClass = errorDelta >= spikeThreshold || discardDelta >= spikeThreshold ? ' row-crit' : (errorDelta > 0 || discardDelta > 0 ? ' row-warn' : '');
    return '<tr class=\'' + rowClass.trim() + '\'>' +
      '<td>'+f.if_index+'</td>' +
      '<td>'+f.if_name+'</td>' +
      '<td><span class='+lc+'>'+f.link_status+'</span></td>' +
      '<td>'+formatCounter(f.in_errors, f.in_errors_delta)+'</td>' +
      '<td>'+formatCounter(f.out_errors, f.out_errors_delta)+'</td>' +
      '<td>'+formatCounter(f.in_discards, f.in_discards_delta)+'</td>' +
      '<td>'+formatCounter(f.out_discards, f.out_discards_delta)+'</td>' +
      '<td>'+(f.bandwidth*100).toFixed(1)+'%</td>' +
      '<td>'+fmtTime(f.sampled_at)+'</td>' +
      '</tr>';
  }).join('');
}

function renderHardwareStatus(sensors) {
  var box = document.getElementById('hardware-status');
  if (!box) return;
  var visible = (sensors || []).filter(function(s) {
    var type = (s.sensor_type || '').toLowerCase();
    return type === 'temperature' || type === 'power' || type === 'fan';
  });
  if (visible.length === 0) {
    box.innerHTML = '<span class=no-data>No Sensors Detected</span>';
    return;
  }
  box.innerHTML = visible.map(function(s) {
    var cls = s.is_alarm ? 'hardware-card crit' : (s.status !== null && s.status !== undefined && s.status !== 0 ? 'hardware-card warn' : 'hardware-card');
    var label = s.name || (s.source === 'entity-physical' ? ('component ' + s.index) : ('sensor ' + s.index));
    var value = (s.value !== null && s.value !== undefined) ? s.value : ((s.status !== null && s.status !== undefined) ? s.status : 'N/A');
    var unit = s.unit ? (' ' + s.unit) : '';
    return '<div class=\'' + cls + '\'>' +
      '<div class=\'label\'>' + label + '</div>' +
      '<div class=\'value\'>' + value + unit + '</div>' +
      '</div>';
  }).join('');
}

function renderBwChart(ifSeries) {
  var legend = document.getElementById('bw-legend');
  if (legend) legend.innerHTML = '';
  var ordered = ifSeries.slice().sort(function(a, b) { return a.if_index - b.if_index; });
  var filtered = ordered.filter(function(s) { return isInterfaceSelected(s.if_index); });
  var series = filtered.map(function(s, i) {
    var color = COLORS[i % COLORS.length];
    var label = IFACE_LABELS[String(s.if_index)] || ('if-' + s.if_index);
    if (legend) legend.innerHTML += '<span style=\'color:'+color+';margin-right:.5rem\'>\u{25a0} '+label+'</span>';
    return { label: label, color: color, points: s.points.map(function(p) { return {t: p.t, v: p.bw}; }) };
  });
  sparkline('bw-chart', series, '%');
}

function renderErrChart(ifSeries) {
  var legend = document.getElementById('err-legend');
  if (legend) legend.innerHTML = '';
  var ordered = ifSeries.slice().sort(function(a, b) { return a.if_index - b.if_index; });
  var series = [];
  ordered.filter(function(s) { return isInterfaceSelected(s.if_index); }).forEach(function(s, i) {
    var color = COLORS[i % COLORS.length];
    var label = IFACE_LABELS[String(s.if_index)] || ('if-' + s.if_index);
    if (legend && i < 4) legend.innerHTML += '<span style=\'color:'+color+';margin-right:.5rem\'>\u{25a0} '+label+'</span>';
    series.push({ label: label+' in_err', color: color,
      points: s.points.map(function(p) { return {t: p.t, v: p.in_err + p.in_dis}; }) });
  });
  sparkline('err-chart', series, 'count');
}

function renderSysChart(metrics) {
  renderCpuChart(metrics);
}

function renderDetail(d) {
  LAST_DEVICE_DETAIL = d;
  renderHeader(d);
  var ifaces = (d.interfaces || []).map(function(f) { f.bandwidth = f.bandwidth || f.bandwidth_utilization || 0; return f; });
  IFACE_LABELS = {};
  ifaces.forEach(function(f) { IFACE_LABELS[String(f.if_index)] = f.if_name || ('if-' + f.if_index); });
  syncSelectedInterfaces(ifaces);
  renderInterfaceSelection(ifaces);
  renderInterfaces(ifaces);
  renderBwChart(d.if_series || []);
  renderErrChart(d.if_series || []);
  renderSysChart(d.metrics || []);
  renderHardwareStatus(d.hardware_sensors || []);
  renderSpikes(d.spikes || []);
  renderAlerts(d.alerts || []);
  updateLastRefreshed();
}

function renderCpuChart(metrics) {
  var cpuSeries = {
    label:'CPU %',
    color:'#38bdf8',
    points: metrics.filter(function(m) { return m.cpu !== null && m.cpu !== undefined && m.cpu > 0; }).map(function(m) { return {t:m.t, v:m.cpu}; })
  };
  sparkline('cpu-chart', [cpuSeries], '%');
}

function renderSpikes(spikes) {
  var tbody = document.getElementById('spike-tbody');
  if (!tbody) return;
  if (!spikes || spikes.length === 0) {
    tbody.innerHTML = '<tr><td colspan=8 class=no-data>' + t('no_interface_spikes') + '</td></tr>';
    return;
  }
  tbody.innerHTML = spikes.map(function(s) {
    var lc = s.link_status === 'up' ? 'link-up' : 'link-down';
    return '<tr>' +
      '<td>'+fmtTime(s.latest_sampled_at)+'</td>' +
      '<td>'+s.if_name+' (if-'+s.if_index+')</td>' +
      '<td><span class='+lc+'>'+s.link_status+'</span></td>' +
      '<td>'+fmtNum(s.in_errors_delta)+'</td>' +
      '<td>'+fmtNum(s.out_errors_delta)+'</td>' +
      '<td>'+fmtNum(s.in_discards_delta)+'</td>' +
      '<td>'+fmtNum(s.out_discards_delta)+'</td>' +
      '<td>'+fmtNum(s.total_delta)+'</td>' +
      '</tr>';
  }).join('');
}

function renderAlerts(alerts) {
  var tbody = document.getElementById('alert-tbody');
  if (!tbody) return;
  if (alerts.length === 0) {
    tbody.innerHTML = '<tr><td colspan=5 class=no-data>' + t('no_alerts') + '</td></tr>';
    return;
  }
  tbody.innerHTML = alerts.map(function(a) {
    var sc = {warning:'sev-warning', critical:'sev-critical', info:'sev-info'}[a.severity] || '';
    var iface = a.interface || '-';
    return '<tr><td>'+fmtTime(a.at)+'</td><td>'+iface+'</td><td>'+a.type+'</td><td><span class='+sc+'>'+a.severity+'</span></td><td>'+a.details+'</td></tr>';
  }).join('');
}

function renderHeader(d) {
  var el = document.getElementById('dev-title');
  if (el) el.textContent = d.name + ' (' + d.ip + ')';
  var st = document.getElementById('dev-status');
  if (st) {
    var clsMap = {online:'status-online', offline:'status-offline', warning:'status-warning', critical:'status-critical'};
    st.className = clsMap[d.status] || 'status-unknown';
    st.textContent = d.status;
  }
  document.title = 'TracePulse \u{2013} ' + d.name;
}

function updateLastRefreshed() {
  var el = document.getElementById('device-refresh');
  if (el) {
    el.textContent = 'Auto refresh: 30s • ' + t('updated') + ': ' + formatTracePulseClock(new Date());
  }
}

function load() {
  Promise.all([
    fetch('/api/settings', { cache: 'no-store' }).then(function(r) { return r.json(); }).catch(function() { return null; }),
    fetch('/api/device/' + encodeURIComponent(DEVICE_IP), { cache: 'no-store' }).then(function(r) { return r.json(); })
  ])
    .then(function(results) {
      var settings = results[0];
      var d = results[1];
      if (settings && settings.display && settings.display.timezone) {
        TIME_ZONE = settings.display.timezone;
        try { localStorage.setItem('tracepulse-timezone', TIME_ZONE); } catch (e) {}
      }
      if (settings && settings.alert && typeof settings.alert.spike_threshold === 'number') {
        window.TRACEPULSE_SPIKE_THRESHOLD = settings.alert.spike_threshold;
      }
      if (d.error) { document.getElementById('dev-title').textContent = d.error; return; }
      renderDetail(d);
    })
    .catch(function(e) { document.getElementById('dev-title').textContent = 'Error: ' + e; });
}

load();
setInterval(load, REFRESH_MS);
";

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

const DASHBOARD_JS: &str = r"
var sortCol = 'ip';
var sortAsc = true;
var COLS = ['ip','name','status','community','last_seen'];
var deletingIps = new Set();

function ipToNum(ip) {
  var p = ip.split('.');
  if (p.length !== 4) return 0;
  return ((parseInt(p[0],10)||0)*16777216)+((parseInt(p[1],10)||0)*65536)+((parseInt(p[2],10)||0)*256)+(parseInt(p[3],10)||0);
}

function statusOrder(s) {
  var o = {critical:0, offline:1, warning:2, unknown:3, online:4};
  return o[s] !== undefined ? o[s] : 5;
}

function cmpVal(a, b, col) {
  if (col === 'ip') return ipToNum(a.ip) - ipToNum(b.ip);
  if (col === 'status') return statusOrder(a.status) - statusOrder(b.status);
  var av = (a[col] || '').toLowerCase();
  var bv = (b[col] || '').toLowerCase();
  return av < bv ? -1 : av > bv ? 1 : 0;
}

function renderTable() {
  var sorted = DEVICES.slice().sort(function(a, b) {
    // スパイク機器を常に最上位に
    if (a.error_spike && !b.error_spike) return -1;
    if (!a.error_spike && b.error_spike) return 1;
    var r = cmpVal(a, b, sortCol);
    return sortAsc ? r : -r;
  });
  var tbody = document.getElementById('dash-tbody');
  if (sorted.length === 0) {
    tbody.innerHTML = '<tr><td colspan=6 class=empty>' + t('no_devices') + ' <a href=/discovery>' + t('nav_discovery') + '</a></td></tr>';
    return;
  }
  tbody.innerHTML = sorted.map(function(d) {
    var clsMap = {online:'status-online', offline:'status-offline', warning:'status-warning', critical:'status-critical'};
    var cls = clsMap[d.status] || 'status-unknown';
    var ls = formatTracePulseTimestamp(d.last_seen || d.last_seen_at);
    var rowCls = d.error_spike ? ' class=spike-row' : '';
    var spikeBadge = d.error_spike ? '<span class=spike-badge>\u26a0 Error Spike</span>' : '';
    var deleting = deletingIps.has(d.ip);
    var actionLabel = deleting ? t('unregistering') : t('unregister');
    var actionBtn = '<button class=\'btn-row-danger\' data-ip=\'' + encodeURIComponent(d.ip) + '\' data-name=\'' + encodeURIComponent(d.name || d.ip) + '\' onclick=\'unregisterDevice(this.dataset.ip,this.dataset.name)\' ' + (deleting ? 'disabled' : '') + '>' + actionLabel + '</button>';
    return '<tr'+rowCls+'><td><a href=/device/'+d.ip+' style=\'color:#38bdf8;text-decoration:none\'>'+d.ip+'</a></td><td>'+d.name+spikeBadge+'</td><td><span class='+cls+'>'+d.status+'</span></td><td>'+d.community+'</td><td>'+ls+'</td><td><div class=\'row-actions\'>'+actionBtn+'</div></td></tr>';
  }).join('');
}

function updateSortIndicators() {
  COLS.forEach(function(col) {
    var th = document.getElementById('th-' + col);
    var icon = document.getElementById('sort-' + col);
    if (!th) return;
    if (col === sortCol) {
      th.className = sortAsc ? 'sort-asc' : 'sort-desc';
      if (icon) icon.textContent = sortAsc ? '\u25b4' : '\u25be';
    } else {
      th.className = '';
      if (icon) icon.textContent = '';
    }
  });
}

function sortBy(col) {
  sortAsc = (sortCol === col) ? !sortAsc : true;
  sortCol = col;
  updateSortIndicators();
  renderTable();
}

function renderSummary(devices) {
  var online = 0, warn = 0, offline = 0, spikes = 0;
  devices.forEach(function(d) {
    if (d.status === 'online') online++;
    else if (d.status === 'warning') warn++;
    else if (d.status === 'offline' || d.status === 'critical') offline++;
    if (d.error_spike) spikes++;
  });
  var el = document.getElementById('summary-cards');
  if (!el) return;
  var spikeCls = spikes > 0 ? 'card card-spike' : 'card';
  el.innerHTML =
    '<div class=\'card card-online\'><div class=\'card-num\'>'+online+'</div><div class=\'card-label\'>'+t('online')+'</div></div>' +
    '<div class=\'card card-warning\'><div class=\'card-num\'>'+warn+'</div><div class=\'card-label\'>'+t('warning')+'</div></div>' +
    '<div class=\'card card-offline\'><div class=\'card-num\'>'+offline+'</div><div class=\'card-label\'>'+t('offline')+'</div></div>' +
    '<div class=\'card\'><div class=\'card-num\'>'+devices.length+'</div><div class=\'card-label\'>'+t('total')+'</div></div>' +
    '<div class=\''+spikeCls+'\'><div class=\'card-num\'>'+spikes+'</div><div class=\'card-label\'>'+t('error_spikes')+'</div></div>';
}

function unregisterDevice(ip, name) {
  ip = decodeURIComponent(ip || '');
  name = decodeURIComponent(name || '');
  if (!confirm(t('unregister_confirm') + '\n\n' + ip + (name ? ' (' + name + ')' : ''))) return;
  deletingIps.add(ip);
  renderTable();
  fetch('/api/device/' + encodeURIComponent(ip), { method: 'DELETE' })
    .then(function(r) { return r.json().then(function(body) { return { ok: r.ok, status: r.status, body: body }; }); })
    .then(function(res) {
      if (!res.ok || (res.body && res.body.error)) {
        throw new Error((res.body && res.body.error) || ('HTTP ' + res.status));
      }
      deletingIps.delete(ip);
      refresh();
      alert(t('unregister_success'));
    })
    .catch(function(e) {
      deletingIps.delete(ip);
      renderTable();
      alert(t('unregister_failed') + ': ' + e.message);
    });
}

function updateLastRefreshed() {
  var el = document.getElementById('last-refreshed');
  if (el) el.textContent = t('updated') + ': ' + formatTracePulseClock(new Date());
}

function refresh() {
  fetch('/api/devices')
    .then(function(r) { return r.json(); })
    .then(function(data) {
      DEVICES = data;
      renderTable();
      renderSummary(data);
      updateLastRefreshed();
    })
    .catch(function(e) { console.warn('refresh failed', e); });
}

renderTable();
updateSortIndicators();
updateLastRefreshed();
setInterval(refresh, 30000);
";

const DISCOVERY_CSS: &str = "<style>
  .scan-form { display:flex; flex-wrap:wrap; gap:.75rem; align-items:flex-start; margin-bottom:1.5rem; }
  .form-group { display:flex; flex-direction:column; gap:.25rem; }
  .form-group label { font-size:.85rem; color:#94a3b8; }
  .form-group input { background:#1e293b; border:1px solid #334155; color:#f1f5f9; padding:.5rem .75rem; border-radius:.375rem; font-size:.95rem; width:100%; box-sizing:border-box; }
  .form-group input:focus { outline:none; border-color:#3b82f6; }
  .scan-form .btn, .scan-form .manual-link { margin-top:1.45rem; }
  .manual-link { margin-left:auto; font-size:.85rem; }
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
  .host-hint.warn { color:#fbbf24; }
</style>";

const DISCOVERY_HTML: &str = "<main>
  <h1 data-i18n='discovery_title'>Device Discovery</h1>

  <div class='scan-form' id='scan-form'>
    <div class='form-group' style='flex:2;min-width:180px'>
      <label for='cidr' data-i18n='cidr_range'>CIDR Range</label>
      <input id='cidr' type='text' placeholder='192.168.1.0/24' data-i18n-placeholder='cidr_range_placeholder' required oninput='updateHint()'>
      <span id='host-hint' class='host-hint'></span>
    </div>
    <div class='form-group' style='flex:1;min-width:130px'>
      <label for='community' data-i18n='community'>Community</label>
      <input id='community' type='text' value='public' data-i18n-placeholder='public_placeholder'>
    </div>
    <button class='btn' id='scan-btn' onclick='startScan()' data-i18n='scan'>Scan</button>
    <button class='btn btn-secondary' id='cancel-btn' style='display:none' onclick='cancelScan()' data-i18n='cancel'>Cancel</button>
    <span class='manual-link'><a href='#' onclick='showManual();return false;' data-i18n='add_manually'>+ Add manually</a></span>
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
const DISCOVERY_JS: &str = r"
const SCAN_TIMEOUT_SECS = 300;  // 5分
const POLL_INTERVAL_MS  = 1500;

let currentJobId   = null;
let pollTimer      = null;
let elapsedTimer   = null;
let scanStartedAt  = null;
let cancelled      = false;

// /8 未満 (16777214台以上) は非現実的なので /8 を上限とする
const MAX_PREFIX = 8;
const CONCURRENCY = 256;

// ── CIDR hint ──
function cidrHostCount(cidr) {
  const m = cidr.match(/^(\d+\.\d+\.\d+\.\d+)\/(\d+)$/);
  if (!m) return null;
  const prefix = parseInt(m[2], 10);
  if (prefix < 0 || prefix > 32) return null;
  if (prefix === 32) return 1;
  if (prefix === 31) return 2;
  return Math.pow(2, 32 - prefix) - 2;
}

function updateHint() {
  const cidr = document.getElementById('cidr').value.trim();
  const hintEl = document.getElementById('host-hint');
  const total = cidrHostCount(cidr);
  if (!total) { hintEl.innerHTML = ''; return; }

  const prefix = parseInt(cidr.split('/')[1], 10);
  const tooLarge = prefix < MAX_PREFIX;
  const secs = Math.ceil(total * 0.5 / CONCURRENCY);

  if (tooLarge) {
    // /8 より広いCIDRは拒否
    hintEl.innerHTML =
      '<span style=\'color:#f87171\'>\u26a0 CIDR too large (/' + prefix + '). Please use /' + MAX_PREFIX + ' or smaller.</span>';
  } else {
    const warn = secs > 30;
    const timeStr = secs >= 60 ? Math.ceil(secs / 60) + ' min' : secs + 's';
    hintEl.innerHTML = '<span>' + total.toLocaleString() + ' hosts \u2022 est. ~' + timeStr + '</span>';
    hintEl.className = 'host-hint' + (warn ? ' warn' : '');
  }
}

// ── Scan start ──
async function startScan() {
  const cidr = document.getElementById('cidr').value.trim();
  const community = document.getElementById('community').value.trim() || 'public';
  if (!cidr) { alert('Please enter a CIDR range.'); return; }

  const prefix = parseInt((cidr.split('/')[1] || '33'), 10);
  if (prefix < MAX_PREFIX) {
    showScanError('CIDR /' + prefix + ' is too large. Please use /' + MAX_PREFIX + ' or smaller (e.g. /16, /24).');
    return;
  }

  const total = cidrHostCount(cidr);
  const maxHosts = total || 65534;

  cancelled = false;
  currentJobId = null;
  clearInterval(pollTimer);
  clearInterval(elapsedTimer);

  document.getElementById('scan-btn').style.display    = 'none';
  document.getElementById('cancel-btn').style.display  = '';
  document.getElementById('scan-error').style.display  = 'none';
  document.getElementById('results-section').style.display = 'none';
  document.getElementById('register-result').style.display = 'none';
  setProgress(0, 0, 0);
  document.getElementById('scan-progress').style.display = 'block';

  try {
    const res = await fetch('/api/discovery/scan', {
      method: 'POST',
      headers: {'Content-Type': 'application/x-www-form-urlencoded'},
      body: 'cidr=' + encodeURIComponent(cidr) + '&community=' + encodeURIComponent(community) + '&max_hosts=' + encodeURIComponent(maxHosts)
    });
    const data = await res.json();
    if (data.error) { finishScanError(data.error); return; }
    currentJobId = data.job_id;
    scanStartedAt = Date.now();
    startPolling(data.total || 0);
  } catch(e) {
    finishScanError('Failed to start scan: ' + e.message);
  }
}

function startPolling(total) {
  // Elapsed timer updates every second
  elapsedTimer = setInterval(function() {
    if (!scanStartedAt) return;
    const elapsed = Math.floor((Date.now() - scanStartedAt) / 1000);
    document.getElementById('progress-elapsed').textContent = 'Elapsed: ' + elapsed + 's';
    if (elapsed >= SCAN_TIMEOUT_SECS && currentJobId) {
      cancelScan();
      finishScanError('Scan timed out after ' + elapsed + 's. Try a smaller CIDR range or increase Max Hosts limit.');
    }
  }, 1000);

  pollTimer = setInterval(function() {
    if (!currentJobId || cancelled) return;
    fetch('/api/discovery/scan/' + currentJobId)
      .then(function(r) { return r.json(); })
      .then(function(data) {
        if (cancelled) return;
        if (data.error) { finishScanError(data.error); return; }
        const scanned = data.scanned || 0;
        const tot     = data.total   || total;
        setProgress(scanned, tot, data.elapsed || 0);
        if (data.status === 'done') {
          finishScanDone(data.devices || [], scanned, tot, data.elapsed || 0);
        } else if (data.status === 'error') {
          finishScanError(data.error || 'Scan failed');
        }
      })
      .catch(function(e) { finishScanError('Polling error: ' + e.message); });
  }, POLL_INTERVAL_MS);
}

function setProgress(scanned, total, elapsed) {
  const pct = total > 0 ? Math.round(scanned / total * 100) : 0;
  document.getElementById('progress-bar').style.width = pct + '%';
  document.getElementById('progress-stats').textContent = scanned + ' / ' + (total || '?') + '  (' + pct + '%)';
  document.getElementById('progress-elapsed').textContent = t('elapsed') + ': ' + elapsed + 's';
  document.getElementById('progress-label').textContent = t('scanning');
}

function stopPolling() {
  clearInterval(pollTimer);
  clearInterval(elapsedTimer);
  pollTimer = null;
  elapsedTimer = null;
}

function cancelScan() {
  cancelled = true;
  currentJobId = null;
  stopPolling();
  document.getElementById('scan-btn').style.display   = '';
  document.getElementById('cancel-btn').style.display = 'none';
  document.getElementById('scan-progress').style.display = 'none';
}

function finishScanError(msg) {
  stopPolling();
  document.getElementById('scan-btn').style.display   = '';
  document.getElementById('cancel-btn').style.display = 'none';
  document.getElementById('scan-progress').style.display = 'none';
  showScanError(msg);
}

function finishScanDone(devices, scanned, total, elapsed) {
  stopPolling();
  currentJobId = null;
  document.getElementById('scan-btn').style.display   = '';
  document.getElementById('cancel-btn').style.display = 'none';
  document.getElementById('scan-progress').style.display = 'none';
  renderResults(devices, scanned, total, elapsed);
}

function showScanError(msg) {
  const el = document.getElementById('scan-error');
  el.textContent = '\u26a0 ' + msg;
  el.style.display = 'block';
}

// ── Results ──
function renderResults(devices, scanned, total, elapsed) {
  const tbody = document.getElementById('results-body');
  tbody.innerHTML = '';
  let hasSelectable = false;
  const summary = '(scanned ' + scanned + '/' + total + ', ' + elapsed + 's)';
  if (devices.length === 0) {
    tbody.innerHTML = '<tr><td colspan=\'5\' class=\'empty\'>' + t('no_snmp_devices_found') + ' in ' + scanned + ' hosts scanned.</td></tr>';
    document.getElementById('results-title').textContent = t('scan_results_label') + ' \u2014 0 ' + t('found') + ' ' + summary;
  } else {
    document.getElementById('results-title').textContent = t('scan_results_label') + ' \u2014 ' + devices.length + ' ' + t('found') + ' ' + summary;
    devices.forEach(function(d) {
      const isExisting = EXISTING.has(d.ip);
      const badge = isExisting ? '<span class=\'badge-registered\'>' + t('registered') + '</span>' : '';
      const statusCls = d.status === 'online' ? 'status-online' : 'status-unknown';
      const row = document.createElement('tr');
      row.innerHTML =
        '<td><input type=\'checkbox\' class=\'row-cb\' data-ip=\'' + d.ip + '\' data-name=\'' + d.name + '\' data-community=\'' + d.community + '\'' + (isExisting ? ' disabled' : ' onchange=\'updateRegisterBtn()\'') + '></td>' +
        '<td>' + d.ip + '</td>' +
        '<td>' + d.name + '</td>' +
        '<td><span class=\'' + statusCls + '\'>' + d.status + '</span></td>' +
        '<td>' + badge + '</td>';
      tbody.appendChild(row);
      if (!isExisting) hasSelectable = true;
    });
  }
  document.getElementById('results-section').style.display = 'block';
  document.getElementById('select-all').checked = false;
  updateRegisterBtn();
}

function updateRegisterBtn() {
  const checked = document.querySelectorAll('.row-cb:checked').length;
  const btn = document.getElementById('register-btn');
  const action = document.getElementById('register-action');
  const countEl = document.getElementById('select-count');
  if (checked > 0) {
    action.style.display = 'flex';
    btn.textContent = '\u2713 ' + t('register') + ' ' + checked + ' device' + (checked > 1 ? 's' : '');
    if (countEl) countEl.textContent = checked + ' ' + t('selected');
  } else {
    action.style.display = 'none';
    if (countEl) countEl.textContent = '';
  }
}

function toggleAll(master) {
  document.querySelectorAll('.row-cb:not(:disabled)').forEach(function(cb) { cb.checked = master.checked; });
  updateRegisterBtn();
}

// ── Register ──
async function registerSelected() {
  const selected = Array.from(document.querySelectorAll('.row-cb:checked')).map(function(cb) {
    return { ip: cb.dataset.ip, name: cb.dataset.name, community: cb.dataset.community };
  });
  if (selected.length === 0) { alert(t('select_at_least_one_device')); return; }
  const btn = document.getElementById('register-btn');
  btn.disabled = true; btn.textContent = t('registering');
  try {
    const res = await fetch('/api/discovery/register', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify(selected)
    });
    const data = await res.json();
    const resultEl = document.getElementById('register-result');
    if (data.error) {
      resultEl.className = 'reg-error';
      resultEl.textContent = '\u2717 ' + data.error;
    } else {
      let msg = '';
      if (data.registered.length > 0) msg += '\u2713 ' + t('registered') + ': ' + data.registered.join(', ') + '. ';
      if (data.skipped.length > 0) msg += 'Skipped (duplicate): ' + data.skipped.join(', ') + '. ';
      if (data.errors.length > 0) msg += 'Errors: ' + data.errors.join(', ') + '.';
      resultEl.className = data.registered.length > 0 ? 'reg-success' : 'reg-error';
      resultEl.textContent = msg.trim();
      data.registered.forEach(function(ip) { EXISTING.add(ip); });
      document.querySelectorAll('.row-cb').forEach(function(cb) {
        if (EXISTING.has(cb.dataset.ip)) {
          cb.disabled = true; cb.checked = false;
          const td = cb.closest('tr').querySelector('td:last-child');
          if (td) td.innerHTML = '<span class=\'badge-registered\'>' + t('registered') + '</span>';
        }
      });
      updateRegisterBtn();
    }
    resultEl.style.display = 'block';
  } catch(e) {
    const resultEl = document.getElementById('register-result');
    resultEl.className = 'reg-error';
    resultEl.textContent = '\u2717 ' + t('request_failed') + ': ' + e.message;
    resultEl.style.display = 'block';
  } finally { btn.disabled = false; updateRegisterBtn(); }
}

// ── Manual add ──
function showManual() { document.getElementById('manual-box').style.display = 'block'; }
function hideManual() {
  document.getElementById('manual-box').style.display = 'none';
  document.getElementById('manual-result').textContent = '';
}

async function addManual() {
  const ip = document.getElementById('manual-ip').value.trim();
  const lastName = ip.split('.').pop();
  const name = document.getElementById('manual-name').value.trim() || ('device-' + lastName);
  const community = document.getElementById('manual-community').value.trim() || 'public';
  const resultEl = document.getElementById('manual-result');
  if (!ip) { resultEl.textContent = '\u26a0 IP address is required.'; return; }
  try {
    const res = await fetch('/api/discovery/register', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify([{ip: ip, name: name, community: community}])
    });
    const data = await res.json();
    if (data.error) {
      resultEl.style.color = '#fca5a5';
      resultEl.textContent = '\u2717 ' + data.error;
    } else if (data.skipped.length > 0) {
      resultEl.style.color = '#fbbf24';
      resultEl.textContent = '\u26a0 ' + ip + ' is already registered.';
    } else if (data.registered.length > 0) {
      resultEl.style.color = '#86efac';
      resultEl.textContent = '\u2713 ' + ip + ' registered successfully.';
      EXISTING.add(ip);
      document.getElementById('manual-ip').value = '';
      document.getElementById('manual-name').value = '';
    } else {
      resultEl.style.color = '#fca5a5';
      resultEl.textContent = '\u2717 Registration failed.';
    }
  } catch(e) {
    resultEl.style.color = '#fca5a5';
    resultEl.textContent = '\u2717 ' + e.message;
  }
}

document.getElementById('cidr').addEventListener('keydown', function(e) { if (e.key === 'Enter') startScan(); });
";

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
    s.replace('\\', r"\\").replace('"', "\\\"").replace('\n', r"\n")
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
            let name = name.unwrap_or_else(|| format!("device-{}", ip.split('.').last().unwrap_or("x")));
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

fn run_polling_loop(config: Arc<Mutex<AppConfig>>, repo: Arc<Mutex<Repository>>) {
    use crate::db::models::{AlertEvent, DeviceMetrics, InterfaceSample};
    use crate::monitor::calculate_bandwidth_utilization_from_delta;
    use crate::snmp::SnmpClient;
    use std::time::Duration;

    println!("Polling engine started");
    const OFFLINE_GRACE_INTERVALS: i64 = 3;

    loop {
        let (interval, spike_threshold) = config.lock()
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
        let handles: Vec<_> = devices.into_iter().map(|device| {
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

                let client = SnmpClient::with_snmp_config(&dev_config.community, snmp_overrides);

                // 死活確認
                let is_online = client.probe_device(&dev_config).is_ok();
                let status = if is_online {
                    "online"
                } else {
                    let grace_secs = interval.saturating_mul(OFFLINE_GRACE_INTERVALS as u64) as i64;
                    let last_seen_secs = device.last_seen_at.as_ref()
                        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
                        .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_seconds())
                        .unwrap_or(i64::MAX);
                    if last_seen_secs <= grace_secs {
                        "warning"
                    } else {
                        "offline"
                    }
                };
                println!("polling: {} ({}) -> {}", device.name, device.ip, status);

                let mut result = PollResult {
                    ip: device.ip.clone(),
                    name: device.name.clone(),
                    device_id,
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
                            sampled_at: now.clone(),
                        });
                    }

                    // インターフェース
                    let indexes = client.discover_interface_indexes(&dev_config)
                        .unwrap_or_else(|_| vec![1, 2, 3]);
                    println!("polling: {} ({}) if_indexes={:?}", device.name, device.ip, indexes);
                    for if_index in indexes.iter().take(16) {
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
                                    in_discards: iface.in_discards,
                                    out_discards: iface.out_discards,
                                    in_octets: iface.in_octets,
                                    out_octets: iface.out_octets,
                                    bandwidth_utilization: iface.bandwidth_utilization,
                                    sampled_at: now.clone(),
                                });
                            }
                            Err(e) => {
                                eprintln!("polling: query_interface failed {} if_index={}: {}", device.ip, if_index, e);
                            }
                        }
                    }
                }
                result
            })
        }).collect();

        // 全スレッドの完了を待ち、結果を DB に一括書き込み
        for handle in handles {
            let result = match handle.join() {
                Ok(r) => r,
                Err(_) => continue,
            };

            if let Ok(r) = repo.lock() {
                let _ = r.update_device_status(&result.ip, &result.status);

                if let Some(metrics) = result.metrics {
                    let _ = r.save_device_metrics(&metrics);
                }

                for sample in &result.samples {
                    let mut sample = sample.clone();
                    if let Ok(Some(prev)) = r.get_latest_interface_sample(sample.device_id, sample.if_index) {
                        sample.bandwidth_utilization = calculate_bandwidth_utilization_from_delta(
                            prev.in_octets.saturating_add(prev.out_octets),
                            sample.in_octets.saturating_add(sample.out_octets),
                            interval,
                            1_000_000_000,
                        );
                    }
                    let _ = r.save_sample(&sample);
                }

                // スパイク検出（インターフェース単位）
                match r.check_interface_spikes(result.device_id, spike_threshold) {
                    Ok(spikes) => {
                        for spike in spikes {
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
                        }
                    }
                    Err(e) => eprintln!("spike check failed for {}: {}", result.ip, e),
                }
            }
        }
    }
}

// ポーリング結果を運ぶ型（スレッド間で Send 可能）
struct PollResult {
    ip: String,
    name: String,
    device_id: i64,
    status: String,
    metrics: Option<crate::db::models::DeviceMetrics>,
    samples: Vec<crate::db::models::InterfaceSample>,
}

impl PollResult {
    fn default_for(ip: String) -> Self {
        Self { ip, name: String::new(), device_id: 0, status: "offline".to_string(), metrics: None, samples: vec![] }
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
    let s = secs % 60; secs /= 60;
    let mi = secs % 60; secs /= 60;
    let h = secs % 24; secs /= 24;
    // days since 1970-01-01
    let mut days = secs;
    let mut y = 1970u64;
    loop {
        let dy = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 366 } else { 365 };
        if days < dy { break; }
        days -= dy; y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let months = [31u64,if leap{29}else{28},31,30,31,30,31,31,30,31,30,31];
    let mut mo = 1u64;
    for dm in &months {
        if days < *dm { break; }
        days -= dm; mo += 1;
    }
    (y, mo, days + 1, h, mi, s)
}
