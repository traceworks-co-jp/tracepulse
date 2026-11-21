pub mod flap_guard;
pub mod outbox;
pub mod settings;
pub mod slack;
pub mod teams;

use crate::alert::{AlertEvent, AlertKind};
use crate::notifier::flap_guard::{AlertBatch, AlertKey, FlapGuard};
use outbox::Outbox;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::net::{TcpStream, UdpSocket};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Critical,
    Offline,
}

impl NotificationSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
            Self::Offline => "Offline",
        }
    }

    pub fn is_immediate(self) -> bool {
        matches!(self, Self::Critical | Self::Offline)
    }

    pub fn theme_color(self) -> &'static str {
        match self {
            Self::Info => "#2563eb",
            Self::Warning => "#d97706",
            Self::Critical => "#dc2626",
            Self::Offline => "#111827",
        }
    }
}

impl From<crate::alert::AlertSeverity> for NotificationSeverity {
    fn from(value: crate::alert::AlertSeverity) -> Self {
        match value {
            crate::alert::AlertSeverity::Info => Self::Info,
            crate::alert::AlertSeverity::Warning => Self::Warning,
            crate::alert::AlertSeverity::Critical => Self::Critical,
            crate::alert::AlertSeverity::Offline => Self::Offline,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertNotification {
    pub id: String,
    pub device_id: Option<i64>,
    pub interface_id: Option<i64>,
    #[serde(skip)]
    pub core_kind: Option<AlertKind>,
    pub device_name: String,
    pub device_ip: String,
    pub interface_name: Option<String>,
    pub alert_kind: String,
    pub severity: NotificationSeverity,
    pub title: String,
    pub message: String,
    pub observed_value: Option<String>,
    pub threshold: Option<String>,
    pub occurred_at: String,
    pub recommended_actions: Vec<String>,
}

impl AlertNotification {
    pub fn from_event(event: AlertEvent) -> Self {
        let alert_kind = event.kind.to_string();
        let resolved = alert_kind.starts_with("RESOLVED_");
        let severity = if resolved {
            NotificationSeverity::Info
        } else {
            NotificationSeverity::from(event.severity)
        };
        let title = if resolved {
            format!(
                "[RESOLVED] {} on {}",
                alert_kind.trim_start_matches("RESOLVED_"),
                event.device_name
            )
        } else {
            format!("{} on {}", event.kind, event.device_name)
        };
        let recommended_actions = default_recommended_actions(&event.kind, severity);

        Self {
            id: event.id,
            device_id: event.device_id,
            interface_id: event.interface_id,
            core_kind: Some(event.kind.clone()),
            device_name: event.device_name,
            device_ip: event.device_ip,
            interface_name: event.interface_name,
            alert_kind,
            severity,
            title,
            message: event.message,
            observed_value: event.observed_value,
            threshold: event.threshold,
            occurred_at: event.occurred_at.to_rfc3339(),
            recommended_actions,
        }
    }

    pub fn flap_key(&self) -> AlertKey {
        AlertKey {
            device_id: self.device_id,
            interface_id: self.interface_id,
            alert_kind: self.alert_kind.clone(),
            severity: self.severity,
        }
    }

    pub fn is_immediate(&self) -> bool {
        self.is_resolved()
            || self.severity.is_immediate()
            || self.core_kind == Some(AlertKind::DeviceOffline)
    }

    pub fn is_resolved(&self) -> bool {
        self.alert_kind.starts_with("RESOLVED_")
    }

    pub fn actions_or_defaults(&self) -> Vec<String> {
        if !self.recommended_actions.is_empty() {
            return self.recommended_actions.clone();
        }

        default_recommended_actions_for_label(&self.alert_kind, self.severity)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct NotifierConfig {
    pub slack: WebhookEndpointConfig,
    pub teams: WebhookEndpointConfig,
    pub webhook: WebhookEndpointConfig,
    pub pagerduty: PagerDutyConfig,
    pub syslog: SyslogConfig,
    pub smtp: SmtpConfig,
    pub flap_guard: FlapGuardConfig,
    pub retry: RetryConfig,
    pub send_resolved: bool,
}

impl Default for NotifierConfig {
    fn default() -> Self {
        Self {
            slack: WebhookEndpointConfig {
                enabled: false,
                webhook_url: None,
                webhook_url_env: Some("TRACEPULSE_SLACK_WEBHOOK_URL".to_string()),
            },
            teams: WebhookEndpointConfig {
                enabled: false,
                webhook_url: None,
                webhook_url_env: Some("TRACEPULSE_TEAMS_WEBHOOK_URL".to_string()),
            },
            webhook: WebhookEndpointConfig {
                enabled: false,
                webhook_url: None,
                webhook_url_env: Some("TRACEPULSE_WEBHOOK_URL".to_string()),
            },
            pagerduty: PagerDutyConfig::default(),
            syslog: SyslogConfig::default(),
            smtp: SmtpConfig::default(),
            flap_guard: FlapGuardConfig::default(),
            retry: RetryConfig::default(),
            send_resolved: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct PagerDutyConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub integration_key_env: String,
}

impl Default for PagerDutyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "https://events.pagerduty.com/v2/enqueue".to_string(),
            integration_key_env: "TRACEPULSE_PAGERDUTY_ROUTING_KEY".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SyslogConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub transport: SyslogTransport,
    pub format: SyslogFormat,
    pub app_name: String,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyslogTransport {
    #[default]
    Udp,
    Tcp,
    Tls,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyslogFormat {
    #[default]
    Cef,
    Rfc5424,
    Rfc3164,
    #[serde(alias = "leef")]
    Leef,
}

impl Default for SyslogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 514,
            transport: SyslogTransport::Udp,
            format: SyslogFormat::Cef,
            app_name: "tracepulse".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SmtpConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password_env: Option<String>,
    pub from: String,
    pub to: Vec<String>,
    pub starttls: bool,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 587,
            username: None,
            password_env: None,
            from: "tracepulse@localhost".to_string(),
            to: Vec::new(),
            starttls: true,
        }
    }
}

impl NotifierConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, NotifierError> {
        let content = std::fs::read_to_string(path).map_err(NotifierError::ConfigIo)?;
        Self::from_toml_str(&content)
    }

    pub fn from_toml_str(content: &str) -> Result<Self, NotifierError> {
        let value = content
            .parse::<toml::Value>()
            .map_err(NotifierError::ConfigParse)?;

        if let Some(notifier) = value.get("notifier") {
            return notifier
                .clone()
                .try_into()
                .map_err(NotifierError::ConfigParse);
        }

        value.try_into().map_err(NotifierError::ConfigParse)
    }

    pub fn from_env() -> Self {
        Self {
            slack: WebhookEndpointConfig {
                enabled: env::var("TRACEPULSE_SLACK_WEBHOOK_URL").is_ok(),
                webhook_url: None,
                webhook_url_env: Some("TRACEPULSE_SLACK_WEBHOOK_URL".to_string()),
            },
            teams: WebhookEndpointConfig {
                enabled: env::var("TRACEPULSE_TEAMS_WEBHOOK_URL").is_ok(),
                webhook_url: None,
                webhook_url_env: Some("TRACEPULSE_TEAMS_WEBHOOK_URL".to_string()),
            },
            webhook: WebhookEndpointConfig {
                enabled: env::var("TRACEPULSE_WEBHOOK_URL").is_ok(),
                webhook_url: None,
                webhook_url_env: Some("TRACEPULSE_WEBHOOK_URL".to_string()),
            },
            pagerduty: PagerDutyConfig::default(),
            syslog: SyslogConfig::default(),
            smtp: SmtpConfig::default(),
            flap_guard: FlapGuardConfig::default(),
            retry: RetryConfig::default(),
            send_resolved: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct WebhookEndpointConfig {
    pub enabled: bool,
    pub webhook_url: Option<String>,
    pub webhook_url_env: Option<String>,
}

impl WebhookEndpointConfig {
    fn resolve(&self, fallback_env: &str) -> Result<Option<String>, NotifierError> {
        if !self.enabled {
            return Ok(None);
        }

        if let Some(url) = &self.webhook_url {
            let resolved = expand_env_placeholders(url)?;
            if !resolved.trim().is_empty() {
                return Ok(Some(resolved));
            }
        }

        let env_name = self.webhook_url_env.as_deref().unwrap_or(fallback_env);
        match env::var(env_name) {
            Ok(url) if !url.trim().is_empty() => Ok(Some(url)),
            _ => Err(NotifierError::MissingWebhookUrl(env_name.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(default)]
pub struct FlapGuardConfig {
    pub window_seconds: u64,
}

impl Default for FlapGuardConfig {
    fn default() -> Self {
        Self { window_seconds: 10 }
    }
}

impl FlapGuardConfig {
    pub fn window(&self) -> Duration {
        Duration::from_secs(self.window_seconds)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(default)]
pub struct RetryConfig {
    pub max_attempts: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self { max_attempts: 3 }
    }
}

// バックオフは運用での調整需要がないため固定値とする。
const INITIAL_BACKOFF: Duration = Duration::from_millis(300);
const MAX_BACKOFF: Duration = Duration::from_millis(5_000);

#[derive(Debug, Clone)]
struct ResolvedNotifierConfig {
    slack_webhook_url: Option<String>,
    teams_webhook_url: Option<String>,
    generic_webhook_url: Option<String>,
    pagerduty: Option<PagerDutyConfig>,
    syslog: Option<SyslogConfig>,
    smtp: Option<SmtpConfig>,
    retry: RetryConfig,
    send_resolved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationChannel {
    Slack,
    Teams,
    PagerDuty,
    Syslog,
    Webhook,
    Smtp,
}

#[derive(Debug, Clone)]
pub struct DeliveryResult {
    pub channel: NotificationChannel,
    pub status_code: u16,
    pub attempts: u32,
}

#[derive(Debug)]
pub enum NotifierError {
    ConfigIo(std::io::Error),
    ConfigParse(toml::de::Error),
    MissingWebhookUrl(String),
    MissingEnvironmentVariable(String),
    Http(reqwest::Error),
    Io(String),
    Suppressed,
    GateUnavailable(String),
    WebhookRejected {
        channel: NotificationChannel,
        status_code: u16,
        body: String,
    },
}

impl Display for NotifierError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigIo(err) => write!(formatter, "notifier config could not be read: {err}"),
            Self::ConfigParse(err) => {
                write!(formatter, "notifier config could not be parsed: {err}")
            }
            Self::MissingWebhookUrl(env_name) => {
                write!(formatter, "notifier webhook URL is missing; set {env_name}")
            }
            Self::MissingEnvironmentVariable(env_name) => {
                write!(
                    formatter,
                    "environment variable {env_name} is required by notifier config"
                )
            }
            Self::Http(err) => write!(formatter, "notifier HTTP request failed: {err}"),
            Self::Io(err) => write!(formatter, "notifier I/O failed: {err}"),
            Self::GateUnavailable(err) => write!(formatter, "maintenance gate: {err}"),
            Self::Suppressed => write!(formatter, "notification suppressed by maintenance"),
            Self::WebhookRejected {
                channel,
                status_code,
                body,
            } => write!(
                formatter,
                "{channel:?} webhook rejected notifier payload with status {status_code}: {body}"
            ),
        }
    }
}

impl Error for NotifierError {}

impl From<reqwest::Error> for NotifierError {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value.without_url())
    }
}

pub trait MaintenanceGate: Send + Sync {
    fn suppress_notification(&self, alert: &AlertNotification, now: i64) -> Result<bool, String>;
}

#[derive(Clone, Copy)]
struct NotificationGate<'a> {
    store: &'a dyn MaintenanceGate,
    first_alert: &'a AlertNotification,
    last_alert: &'a AlertNotification,
}

impl NotificationGate<'_> {
    fn check(self) -> Result<(), NotifierError> {
        let now = chrono::Utc::now().timestamp();
        for alert in [self.first_alert, self.last_alert] {
            match self.store.suppress_notification(alert, now) {
                Ok(true) => return Err(NotifierError::Suppressed),
                Ok(false) => {}
                Err(error) => {
                    tracing::error!(alert_id = %alert.id, %error,
                        "maintenance notification gate failed; refusing delivery");
                    return Err(NotifierError::GateUnavailable(error));
                }
            }
            if std::ptr::eq(self.first_alert, self.last_alert) {
                break;
            }
        }
        Ok(())
    }
}

fn check_gate(gate: Option<NotificationGate<'_>>) -> Result<(), NotifierError> {
    gate.map_or(Ok(()), NotificationGate::check)
}

pub struct AdvancedNotifier {
    client: Client,
    config: ResolvedNotifierConfig,
    flap_guard: FlapGuard,
    window: Duration,
    maintenance: Option<Arc<dyn MaintenanceGate>>,
    outbox: Option<Outbox>,
    source: &'static str,
}

pub type Notifier = AdvancedNotifier;

pub async fn run_alert_listener(
    mut receiver: broadcast::Receiver<AlertEvent>,
    config: NotifierConfig,
) -> Result<(), NotifierError> {
    let flush_interval = flush_interval_for(config.flap_guard.window());
    let mut notifier = Notifier::new(config)?;
    let mut interval = tokio::time::interval(flush_interval);

    loop {
        tokio::select! {
            received = receiver.recv() => {
                match received {
                    Ok(event) => {
                        notifier.dispatch(event).await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        notifier.flush_due().await?;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        notifier.flush_all().await?;
                        return Ok(());
                    }
                }
            }
            _ = interval.tick() => {
                notifier.flush_due().await?;
            }
        }
    }
}

impl AdvancedNotifier {
    pub fn new(config: NotifierConfig) -> Result<Self, NotifierError> {
        let resolved = ResolvedNotifierConfig {
            slack_webhook_url: config.slack.resolve("TRACEPULSE_SLACK_WEBHOOK_URL")?,
            teams_webhook_url: config.teams.resolve("TRACEPULSE_TEAMS_WEBHOOK_URL")?,
            generic_webhook_url: config.webhook.resolve("TRACEPULSE_WEBHOOK_URL")?,
            pagerduty: resolve_pagerduty(&config.pagerduty)?,
            syslog: config.syslog.enabled.then_some(config.syslog.clone()),
            smtp: config.smtp.enabled.then_some(config.smtp.clone()),
            retry: config.retry,
            send_resolved: config.send_resolved,
        };

        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build()?,
            config: resolved,
            flap_guard: FlapGuard::new(config.flap_guard.window()),
            window: config.flap_guard.window(),
            maintenance: None,
            outbox: None,
            source: "tracepulse",
        })
    }

    /// Attach the fail-closed maintenance gate to normal alert delivery.
    pub fn with_maintenance(mut self, store: Arc<dyn MaintenanceGate>) -> Self {
        self.maintenance = Some(store);
        self
    }

    pub fn with_outbox(mut self, outbox: Outbox) -> Self {
        self.outbox = Some(outbox);
        self
    }

    pub fn with_source(mut self, source: &'static str) -> Self {
        self.source = source;
        self
    }

    /// Create a delivery-only worker without sharing or cloning memory-mode flap state.
    pub fn delivery_worker(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            flap_guard: FlapGuard::new(self.window),
            window: self.window,
            maintenance: self.maintenance.clone(),
            outbox: self.outbox.clone(),
            source: self.source,
        }
    }

    pub async fn dispatch(
        &mut self,
        event: AlertEvent,
    ) -> Result<Vec<DeliveryResult>, NotifierError> {
        self.ingest(AlertNotification::from_event(event)).await
    }

    pub async fn ingest(
        &mut self,
        alert: AlertNotification,
    ) -> Result<Vec<DeliveryResult>, NotifierError> {
        if self.outbox.is_some() {
            self.enqueue(alert)?;
            return self.flush_outbox(false).await;
        }
        let gate = self.maintenance.as_deref().map(|store| NotificationGate {
            store,
            first_alert: &alert,
            last_alert: &alert,
        });
        match check_gate(gate) {
            Err(NotifierError::Suppressed) => return Ok(Vec::new()),
            result => result?,
        }
        if alert.is_resolved() && !self.config.send_resolved {
            return Ok(Vec::new());
        }
        let mut delivered = Vec::new();
        for batch in self.flap_guard.ingest(alert) {
            delivered.extend(self.dispatch_batch(&batch).await?);
        }
        Ok(delivered)
    }

    pub async fn flush_due(&mut self) -> Result<Vec<DeliveryResult>, NotifierError> {
        if self.outbox.is_some() {
            return self.flush_outbox(false).await;
        }
        let batches = self.flap_guard.flush_due();
        self.dispatch_batches(&batches).await
    }

    pub async fn flush_all(&mut self) -> Result<Vec<DeliveryResult>, NotifierError> {
        if self.outbox.is_some() {
            return self.flush_outbox(true).await;
        }
        let batches = self.flap_guard.flush_all();
        self.dispatch_batches(&batches).await
    }

    pub fn enqueue(&self, alert: AlertNotification) -> Result<(), NotifierError> {
        let outbox = self
            .outbox
            .as_ref()
            .ok_or_else(|| NotifierError::Io("outbox not attached".into()))?;
        if alert.is_resolved() && !self.config.send_resolved {
            return Ok(());
        }
        let gate = self.maintenance.as_deref().map(|store| NotificationGate {
            store,
            first_alert: &alert,
            last_alert: &alert,
        });
        // A database error must not send, but preserving the work permits later recovery.
        if matches!(check_gate(gate), Err(NotifierError::Suppressed)) {
            return Ok(());
        }
        outbox.enqueue(&alert, &self.channels(), self.window)
    }

    /// Stage a newly rebuilt release summary inside the maintenance transaction.
    pub fn enqueue_release(
        &self,
        db: &rusqlite::Connection,
        alert: &AlertNotification,
    ) -> Result<(), NotifierError> {
        outbox::enqueue_on(db, alert, &self.channels(), Duration::ZERO)
    }

    fn channels(&self) -> Vec<NotificationChannel> {
        use NotificationChannel::*;
        [
            (Slack, self.config.slack_webhook_url.is_some()),
            (Teams, self.config.teams_webhook_url.is_some()),
            (PagerDuty, self.config.pagerduty.is_some()),
            (Syslog, self.config.syslog.is_some()),
            (Webhook, self.config.generic_webhook_url.is_some()),
            (Smtp, self.config.smtp.is_some()),
        ]
        .into_iter()
        .filter_map(|(channel, enabled)| enabled.then_some(channel))
        .collect()
    }

    async fn flush_outbox(
        &self,
        flush_buffered: bool,
    ) -> Result<Vec<DeliveryResult>, NotifierError> {
        let outbox = self.outbox.as_ref().expect("attached outbox");
        let mut delivered = Vec::new();
        for id in outbox.ready(flush_buffered)? {
            let Some(work) = outbox.claim(id)? else {
                continue;
            };
            let gate = self.maintenance.as_deref().map(|store| NotificationGate {
                store,
                first_alert: &work.batch.first_alert,
                last_alert: &work.batch.last_alert,
            });
            let verdict = if gate.is_none() {
                Err(NotifierError::Io(
                    "outbox requires a maintenance gate".into(),
                ))
            } else {
                check_gate(gate)
            };
            match verdict {
                Err(NotifierError::Suppressed) => outbox.finish(&work, "suppressed")?,
                Err(_) => outbox.defer(&work, false)?,
                Ok(()) if !self.channels().contains(&work.channel) => {
                    outbox.finish(&work, "disabled")?
                }
                Ok(()) => {
                    use NotificationChannel::*;
                    let result = self
                        .send_channel(
                            work.channel,
                            &work.batch,
                            gate,
                            RetryConfig { max_attempts: 1 },
                        )
                        .await;
                    match result {
                        Ok(mut result) => {
                            outbox.finish(&work, "delivered")?;
                            result.attempts = work.attempts + 1;
                            delivered.push(result);
                        }
                        Err(NotifierError::Suppressed) => outbox.finish(&work, "suppressed")?,
                        Err(NotifierError::GateUnavailable(_)) => outbox.defer(&work, false)?,
                        Err(_) => {
                            let max_attempts = if work.channel == PagerDuty {
                                3
                            } else {
                                self.config.retry.max_attempts.max(1)
                            };
                            tracing::warn!(channel = ?work.channel, attempt = work.attempts + 1, "notification delivery failed");
                            if work.attempts + 1 >= max_attempts {
                                outbox.finish(&work, "failed")?;
                            } else {
                                outbox.defer(&work, true)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(delivered)
    }

    async fn dispatch_batches(
        &self,
        batches: &[AlertBatch],
    ) -> Result<Vec<DeliveryResult>, NotifierError> {
        let mut delivered = Vec::new();
        for batch in batches {
            delivered.extend(self.dispatch_batch(batch).await?);
        }
        Ok(delivered)
    }

    async fn dispatch_batch(
        &self,
        batch: &AlertBatch,
    ) -> Result<Vec<DeliveryResult>, NotifierError> {
        let mut delivered = Vec::new();
        let gate = self.maintenance.as_deref().map(|store| NotificationGate {
            store,
            first_alert: &batch.first_alert,
            last_alert: &batch.last_alert,
        });
        match check_gate(gate) {
            Err(NotifierError::Suppressed) => return Ok(delivered),
            result => result?,
        }
        for channel in self.channels() {
            match self
                .send_channel(channel, batch, gate, self.config.retry)
                .await
            {
                Ok(result) => delivered.push(result),
                Err(NotifierError::Suppressed) => break,
                Err(error) => return Err(error),
            }
        }
        Ok(delivered)
    }

    async fn send_channel(
        &self,
        channel: NotificationChannel,
        batch: &AlertBatch,
        gate: Option<NotificationGate<'_>>,
        retry: RetryConfig,
    ) -> Result<DeliveryResult, NotifierError> {
        check_gate(gate)?;
        let (url, payload) = match channel {
            NotificationChannel::Slack => (
                &self.config.slack_webhook_url,
                slack::build_slack_payload(batch),
            ),
            NotificationChannel::Teams => (
                &self.config.teams_webhook_url,
                teams::build_teams_payload(batch),
            ),
            NotificationChannel::Webhook => (
                &self.config.generic_webhook_url,
                serde_json::json!({
                    "source": self.source, "alert": alert_json(&batch.last_alert), "occurrences": batch.count,
                }),
            ),
            NotificationChannel::PagerDuty => {
                return send_pagerduty(
                    &self.client,
                    self.config.pagerduty.as_ref().unwrap(),
                    batch,
                    gate,
                    if self.outbox.is_some() {
                        retry
                    } else {
                        RetryConfig { max_attempts: 3 }
                    },
                )
                .await;
            }
            NotificationChannel::Syslog => {
                return send_syslog(self.config.syslog.as_ref().unwrap(), batch, gate).await;
            }
            NotificationChannel::Smtp => {
                return send_smtp(self.config.smtp.as_ref().unwrap(), batch, gate).await;
            }
        };
        send_json_with_retry_gated(
            &self.client,
            channel,
            url.as_deref().unwrap(),
            &payload,
            retry,
            gate,
        )
        .await
    }
}

/// Web UI から呼ばれる疎通テスト。実アラートを待たずに Webhook 設定を検証する。
/// Explicit configuration tests intentionally bypass maintenance suppression.
pub async fn send_test_notification(
    config: &NotifierConfig,
    channel: NotificationChannel,
) -> Result<DeliveryResult, NotifierError> {
    if matches!(channel, NotificationChannel::PagerDuty) {
        let pagerduty = resolve_pagerduty(&config.pagerduty)?
            .ok_or_else(|| NotifierError::Io("PagerDuty is disabled".to_string()))?;
        return send_pagerduty(
            &Client::new(),
            &pagerduty,
            &test_alert_batch(),
            None,
            config.retry,
        )
        .await;
    }
    if matches!(channel, NotificationChannel::Syslog) {
        let batch = test_alert_batch();
        let syslog = config
            .syslog
            .enabled
            .then_some(config.syslog.clone())
            .ok_or_else(|| NotifierError::Io("Syslog is disabled".to_string()))?;
        return send_syslog(&syslog, &batch, None).await;
    }
    if matches!(channel, NotificationChannel::Webhook) {
        let url = config
            .webhook
            .resolve("TRACEPULSE_WEBHOOK_URL")?
            .ok_or_else(|| {
                NotifierError::MissingWebhookUrl("TRACEPULSE_WEBHOOK_URL".to_string())
            })?;
        let payload = serde_json::json!({
            "source": "tracepulse",
            "alert": alert_json(&test_alert_batch().last_alert),
        });
        return send_json_with_retry(
            &Client::new(),
            NotificationChannel::Webhook,
            &url,
            &payload,
            config.retry,
        )
        .await;
    }
    if matches!(channel, NotificationChannel::Smtp) {
        let smtp = config
            .smtp
            .enabled
            .then_some(config.smtp.clone())
            .ok_or_else(|| NotifierError::Io("SMTP is disabled".to_string()))?;
        return send_smtp(&smtp, &test_alert_batch(), None).await;
    }

    let (endpoint, fallback_env) = match channel {
        NotificationChannel::Slack => (&config.slack, "TRACEPULSE_SLACK_WEBHOOK_URL"),
        NotificationChannel::Teams => (&config.teams, "TRACEPULSE_TEAMS_WEBHOOK_URL"),
        NotificationChannel::PagerDuty
        | NotificationChannel::Syslog
        | NotificationChannel::Webhook
        | NotificationChannel::Smtp => {
            return Err(NotifierError::MissingEnvironmentVariable(
                "channel-specific transport does not support webhook tests".to_string(),
            ));
        }
    };

    let Some(url) = endpoint.resolve(fallback_env)? else {
        return Err(NotifierError::MissingWebhookUrl(
            endpoint
                .webhook_url_env
                .clone()
                .unwrap_or_else(|| fallback_env.to_string()),
        ));
    };

    let batch = test_alert_batch();
    let payload = match channel {
        NotificationChannel::Slack => slack::build_slack_payload(&batch),
        NotificationChannel::Teams => teams::build_teams_payload(&batch),
        NotificationChannel::PagerDuty
        | NotificationChannel::Syslog
        | NotificationChannel::Webhook
        | NotificationChannel::Smtp => {
            return Err(NotifierError::MissingEnvironmentVariable(
                "PagerDuty and Syslog use their configured transports".to_string(),
            ));
        }
    };

    send_json_with_retry(&Client::new(), channel, &url, &payload, config.retry).await
}

fn resolve_pagerduty(config: &PagerDutyConfig) -> Result<Option<PagerDutyConfig>, NotifierError> {
    if !config.enabled {
        return Ok(None);
    }
    if env::var(&config.integration_key_env)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err(NotifierError::MissingEnvironmentVariable(
            config.integration_key_env.clone(),
        ));
    }
    Ok(Some(config.clone()))
}

async fn send_pagerduty(
    client: &Client,
    config: &PagerDutyConfig,
    batch: &AlertBatch,
    gate: Option<NotificationGate<'_>>,
    retry: RetryConfig,
) -> Result<DeliveryResult, NotifierError> {
    let routing_key = env::var(&config.integration_key_env).map_err(|_| {
        NotifierError::MissingEnvironmentVariable(config.integration_key_env.clone())
    })?;
    let alert = &batch.last_alert;
    let payload = serde_json::json!({
        "routing_key": routing_key,
        "event_action": "trigger",
        "dedup_key": alert.id,
        "payload": {
            "summary": alert.title,
            "source": alert.device_name,
            "severity": match alert.severity {
                NotificationSeverity::Info => "info",
                NotificationSeverity::Warning => "warning",
                NotificationSeverity::Critical => "critical",
                NotificationSeverity::Offline => "critical",
            },
            "timestamp": alert.occurred_at,
            "custom_details": {
                "device_ip": alert.device_ip,
                "alert_kind": alert.alert_kind,
                "message": alert.message,
            }
        }
    });
    send_json_with_retry_gated(
        client,
        NotificationChannel::PagerDuty,
        &config.endpoint,
        &payload,
        retry,
        gate,
    )
    .await
}

async fn send_syslog(
    config: &SyslogConfig,
    batch: &AlertBatch,
    gate: Option<NotificationGate<'_>>,
) -> Result<DeliveryResult, NotifierError> {
    let alert = &batch.last_alert;
    let address = format!("{}:{}", config.host, config.port);
    let message = match config.format {
        SyslogFormat::Cef => format!(
            "CEF:0|TracePulse|tracepulse|1|{}|{}|{}|src={} dst={} msg={}",
            alert.alert_kind,
            escape_syslog(&alert.title),
            syslog_severity(alert.severity),
            escape_syslog(&alert.device_ip),
            escape_syslog(&alert.device_name),
            escape_syslog(&alert.message),
        ),
        SyslogFormat::Leef => format!(
            "LEEF:2.0|TracePulse|tracepulse|1|{}|devTime={}\tsev={}\tsrc={}\tdst={}\tmsg={}",
            escape_syslog(&alert.alert_kind),
            escape_syslog(&alert.occurred_at),
            syslog_severity(alert.severity),
            escape_syslog(&alert.device_ip),
            escape_syslog(&alert.device_name),
            escape_syslog(&alert.message),
        ),
        SyslogFormat::Rfc5424 => format!(
            "<{}>1 {} {} {} - - [tracepulse@1 alert_kind=\"{}\" device_ip=\"{}\"] {}",
            16 + syslog_severity(alert.severity),
            alert.occurred_at,
            config.app_name,
            alert.id,
            escape_syslog(&alert.alert_kind),
            escape_syslog(&alert.device_ip),
            escape_syslog(&alert.message),
        ),
        SyslogFormat::Rfc3164 => format!(
            "<{}>{} TracePulse: {} - {}",
            16 + syslog_severity(alert.severity),
            chrono::DateTime::parse_from_rfc3339(&alert.occurred_at)
                .map(|value| value.format("%b %e %H:%M:%S").to_string())
                .unwrap_or_else(|_| alert.occurred_at.clone()),
            escape_syslog(&alert.title),
            escape_syslog(&alert.message),
        ),
    };
    match config.transport {
        SyslogTransport::Udp => {
            let socket = UdpSocket::bind("0.0.0.0:0")
                .map_err(|error| NotifierError::Io(error.to_string()))?;
            check_gate(gate)?;
            socket
                .send_to(message.as_bytes(), &address)
                .map_err(|error| NotifierError::Io(error.to_string()))?;
        }
        SyslogTransport::Tcp => {
            let mut stream = TcpStream::connect(&address)
                .map_err(|error| NotifierError::Io(error.to_string()))?;
            check_gate(gate)?;
            stream
                .write_all(format!("{} ", message.len()).as_bytes())
                .and_then(|_| stream.write_all(message.as_bytes()))
                .map_err(|error| NotifierError::Io(error.to_string()))?;
        }
        SyslogTransport::Tls => {
            let root_store = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };
            let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .map_err(|error| NotifierError::Io(error.to_string()))?
            .with_root_certificates(root_store)
            .with_no_client_auth();
            let server_name = rustls::pki_types::ServerName::try_from(config.host.clone())
                .map_err(|error| NotifierError::Io(format!("invalid syslog TLS host: {error}")))?;
            let connection = rustls::ClientConnection::new(Arc::new(tls_config), server_name)
                .map_err(|error| NotifierError::Io(error.to_string()))?;
            let stream = TcpStream::connect(&address)
                .map_err(|error| NotifierError::Io(error.to_string()))?;
            let mut stream = rustls::StreamOwned::new(connection, stream);
            check_gate(gate)?;
            stream
                .write_all(format!("{} ", message.len()).as_bytes())
                .and_then(|_| stream.write_all(message.as_bytes()))
                .map_err(|error| NotifierError::Io(error.to_string()))?;
        }
    }
    Ok(DeliveryResult {
        channel: NotificationChannel::Syslog,
        status_code: 200,
        attempts: 1,
    })
}

fn alert_json(alert: &AlertNotification) -> Value {
    serde_json::json!({
        "id": alert.id,
        "kind": alert.alert_kind,
        "severity": alert.severity.as_str(),
        "title": alert.title,
        "message": alert.message,
        "device": alert.device_name,
        "device_ip": alert.device_ip,
        "occurred_at": alert.occurred_at,
    })
}

async fn send_smtp(
    config: &SmtpConfig,
    batch: &AlertBatch,
    gate: Option<NotificationGate<'_>>,
) -> Result<DeliveryResult, NotifierError> {
    use lettre::message::{Mailbox, Message, SinglePart, header::ContentType};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

    let from: Mailbox = config
        .from
        .parse()
        .map_err(|error| NotifierError::Io(format!("invalid SMTP from address: {error}")))?;
    let mut builder = Message::builder().from(from).subject(format!(
        "[{}] {} ({} occurrence(s))",
        batch.last_alert.severity.as_str(),
        batch.last_alert.title,
        batch.count
    ));
    for recipient in &config.to {
        builder = builder.to(recipient
            .parse()
            .map_err(|error| NotifierError::Io(format!("invalid SMTP recipient: {error}")))?);
    }
    let body = format!(
        "<html><body><h2>{}</h2><p>{}</p><p>Device: {} ({})</p><p>Occurrences: {}</p><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"360\" height=\"48\"><polyline points=\"0,40 60,28 120,34 180,12 240,24 300,8 360,18\" fill=\"none\" stroke=\"#dc2626\" stroke-width=\"3\"/></svg></body></html>",
        batch.last_alert.title,
        batch.last_alert.message,
        batch.last_alert.device_name,
        batch.last_alert.device_ip,
        batch.count,
    );
    let email = builder
        .singlepart(
            SinglePart::builder()
                .header(ContentType::TEXT_HTML)
                .body(body),
        )
        .map_err(|error| NotifierError::Io(error.to_string()))?;
    let mut transport = if config.starttls {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
    }
    .map_err(|error| NotifierError::Io(error.to_string()))?
    .port(config.port)
    .timeout(Some(Duration::from_secs(30)));
    if let (Some(username), Some(password_env)) = (&config.username, &config.password_env) {
        let password = env::var(password_env)
            .map_err(|_| NotifierError::MissingEnvironmentVariable(password_env.clone()))?;
        transport = transport.credentials(Credentials::new(username.clone(), password));
    }
    let transport = transport.build();
    check_gate(gate)?;
    transport
        .send(email)
        .await
        .map_err(|error| NotifierError::Io(error.to_string()))?;
    Ok(DeliveryResult {
        channel: NotificationChannel::Smtp,
        status_code: 250,
        attempts: 1,
    })
}

fn syslog_severity(severity: NotificationSeverity) -> u8 {
    match severity {
        NotificationSeverity::Info => 3,
        NotificationSeverity::Warning => 5,
        NotificationSeverity::Critical | NotificationSeverity::Offline => 10,
    }
}

fn escape_syslog(value: &str) -> String {
    value.replace(['|', '\n', '\r', '\t'], " ")
}

fn test_alert_batch() -> AlertBatch {
    let now = std::time::Instant::now();
    let alert = AlertNotification {
        id: "test-notification".to_string(),
        device_id: None,
        interface_id: None,
        core_kind: None,
        device_name: "TracePulse".to_string(),
        device_ip: "127.0.0.1".to_string(),
        interface_name: None,
        alert_kind: "TestNotification".to_string(),
        severity: NotificationSeverity::Info,
        title: "TracePulse test notification".to_string(),
        message: "This is a test notification sent from the TracePulse settings page. \
Webhook delivery is working."
            .to_string(),
        observed_value: None,
        threshold: None,
        occurred_at: chrono::Utc::now().to_rfc3339(),
        recommended_actions: vec![
            "No action required. This message confirms the webhook configuration.".to_string(),
        ],
    };

    AlertBatch {
        key: alert.flap_key(),
        first_alert: alert.clone(),
        last_alert: alert,
        count: 1,
        first_seen: now,
        last_seen: now,
    }
}

async fn send_json_with_retry(
    client: &Client,
    channel: NotificationChannel,
    url: &str,
    payload: &Value,
    retry: RetryConfig,
) -> Result<DeliveryResult, NotifierError> {
    send_json_with_retry_gated(client, channel, url, payload, retry, None).await
}

async fn send_json_with_retry_gated(
    client: &Client,
    channel: NotificationChannel,
    url: &str,
    payload: &Value,
    retry: RetryConfig,
    gate: Option<NotificationGate<'_>>,
) -> Result<DeliveryResult, NotifierError> {
    let max_attempts = retry.max_attempts.max(1);
    let mut backoff = INITIAL_BACKOFF;

    for attempt in 1..=max_attempts {
        let request = client.post(url).json(payload);
        check_gate(gate)?;
        let response = request.send().await;
        match response {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(DeliveryResult {
                        channel,
                        status_code: response.status().as_u16(),
                        attempts: attempt,
                    });
                }

                if attempt == max_attempts {
                    let status_code = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();
                    return Err(NotifierError::WebhookRejected {
                        channel,
                        status_code,
                        body,
                    });
                }

                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
            Err(_) if attempt < max_attempts => {
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
            Err(err) => return Err(NotifierError::from(err)),
        }
    }

    unreachable!("retry loop always returns")
}

fn expand_env_placeholders(value: &str) -> Result<String, NotifierError> {
    let mut resolved = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find("${") {
        resolved.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find('}') else {
            resolved.push_str(&rest[start..]);
            return Ok(resolved);
        };
        let env_name = &after_start[..end];
        let env_value = env::var(env_name)
            .map_err(|_| NotifierError::MissingEnvironmentVariable(env_name.to_string()))?;
        resolved.push_str(&env_value);
        rest = &after_start[end + 1..];
    }

    resolved.push_str(rest);
    Ok(resolved)
}

fn flush_interval_for(window: Duration) -> Duration {
    if window.is_zero() {
        Duration::from_millis(250)
    } else {
        window.min(Duration::from_secs(1))
    }
}

fn default_recommended_actions(
    alert_kind: &AlertKind,
    severity: NotificationSeverity,
) -> Vec<String> {
    match alert_kind {
        AlertKind::InterfaceSpike => {
            return vec![
                "Check recent interface traffic deltas and top talkers.".to_string(),
                "Confirm whether the traffic pattern matches a planned backup or batch job."
                    .to_string(),
                "Review in/out errors and discards on the affected interface.".to_string(),
            ];
        }
        AlertKind::ErrorRate => {
            return vec![
                "Inspect physical link quality, transceiver status, and cable path.".to_string(),
                "Compare error counters against the previous polling interval.".to_string(),
                "Check duplex, speed, and MTU consistency on both link ends.".to_string(),
            ];
        }
        AlertKind::Custom(kind)
            if matches!(
                kind.as_str(),
                "PREDICTIVE_ERROR_RATE"
                    | "PREDICTIVE_TREND"
                    | "IF_ERROR_RATE_SPIKE"
                    | "IF_ERROR_WEEKLY_TREND"
            ) =>
        {
            return vec![
                "Inspect physical link quality, transceiver status, and cable path.".to_string(),
                "Compare error counters against the previous polling interval.".to_string(),
                "Check duplex, speed, and MTU consistency on both link ends.".to_string(),
            ];
        }
        AlertKind::Custom(kind) if kind == "PREDICTIVE_DOM" => {
            return vec![
                "Inspect the SFP and fiber path for contamination, bending, or degradation."
                    .to_string(),
                "Compare Rx optical power with the transceiver vendor specification.".to_string(),
            ];
        }
        AlertKind::Custom(kind) if kind == "PREDICTIVE_ENV" => {
            return vec![
                "Compare the current temperature or fan reading with its historical baseline."
                    .to_string(),
                "Inspect chassis airflow, fan operation, and environmental conditions.".to_string(),
            ];
        }
        AlertKind::DeviceOffline => {
            return vec![
                "Verify device reachability from the monitoring segment.".to_string(),
                "Check power, uplink, routing, and recent maintenance activity.".to_string(),
                "Escalate to on-call network operations if the device remains unreachable."
                    .to_string(),
            ];
        }
        AlertKind::HealthDegraded | AlertKind::Custom(_) => {}
    }

    match severity {
        NotificationSeverity::Offline => vec![
            "Verify device reachability from the monitoring segment.".to_string(),
            "Check power, uplink, routing, and recent maintenance activity.".to_string(),
            "Escalate to on-call network operations if the device remains unreachable.".to_string(),
        ],
        NotificationSeverity::Critical => vec![
            "Confirm current health metrics and isolate the failing subsystem.".to_string(),
            "Check related interface, CPU, memory, and error-rate trends.".to_string(),
            "Prepare rollback or traffic shift if the condition is service-impacting.".to_string(),
        ],
        NotificationSeverity::Warning => vec![
            "Review the last few polling intervals for trend confirmation.".to_string(),
            "Check whether the threshold should be tuned for this device role.".to_string(),
        ],
        NotificationSeverity::Info => {
            vec!["Review the event context and confirm whether action is required.".to_string()]
        }
    }
}

fn default_recommended_actions_for_label(
    alert_kind: &str,
    severity: NotificationSeverity,
) -> Vec<String> {
    let normalized = alert_kind.to_ascii_lowercase();
    if normalized.contains("spike") || normalized.contains("traffic") {
        return default_recommended_actions(&AlertKind::InterfaceSpike, severity);
    }

    if normalized.contains("error") || normalized.contains("discard") {
        return default_recommended_actions(&AlertKind::ErrorRate, severity);
    }

    match severity {
        NotificationSeverity::Offline => {
            default_recommended_actions(&AlertKind::DeviceOffline, severity)
        }
        _ => default_recommended_actions(&AlertKind::HealthDegraded, severity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::{AlertBroadcaster, AlertEvent};
    use crate::device::types::DeviceConfig;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    #[tokio::test]
    async fn outbox_restarts_without_repeating_a_successful_channel() {
        let dir = tempfile::tempdir_in("target").unwrap();
        let path = dir.path().join("notification.db");
        let store = Arc::new(crate::maintenance::MaintenanceStore::test_fixture());
        let (slack_url, slack) = spawn_webhook_server();
        let failed_webhook = TcpListener::bind("127.0.0.1:0").unwrap();
        let dead_url = format!("http://{}", failed_webhook.local_addr().unwrap());
        let reject = thread::spawn(move || {
            let (mut stream, _) = failed_webhook.accept().unwrap();
            read_webhook_body(&mut stream);
            stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let endpoint = |url| WebhookEndpointConfig {
            enabled: true,
            webhook_url: Some(url),
            webhook_url_env: None,
        };
        let mut config = NotifierConfig {
            slack: endpoint(slack_url),
            webhook: endpoint(dead_url),
            ..NotifierConfig::default()
        };
        let alert = maintenance_alert(1, NotificationSeverity::Critical);
        {
            let mut notifier = Notifier::new(config.clone())
                .unwrap()
                .with_maintenance(store.clone())
                .with_outbox(Outbox::open(&path).unwrap());
            let delivered = notifier.ingest(alert.clone()).await.unwrap();
            assert_eq!(
                delivered.iter().map(|d| d.channel).collect::<Vec<_>>(),
                [NotificationChannel::Slack]
            );
            slack.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        reject.join().unwrap();

        let slack_repeat = TcpListener::bind("127.0.0.1:0").unwrap();
        config.slack = endpoint(format!("http://{}", slack_repeat.local_addr().unwrap()));
        let (webhook_url, webhook) = spawn_webhook_server();
        config.webhook = endpoint(webhook_url);
        let mut restarted = Notifier::new(config)
            .unwrap()
            .with_maintenance(store)
            .with_outbox(Outbox::open(&path).unwrap());
        restarted.enqueue(alert).unwrap(); // Replayed input must not create another batch.
        tokio::time::sleep(Duration::from_millis(330)).await;
        let delivered = restarted.flush_due().await.unwrap();
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].channel, NotificationChannel::Webhook);
        assert_eq!(delivered[0].attempts, 2);
        webhook.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_no_request(&slack_repeat);
    }

    #[tokio::test]
    async fn outbox_rechecks_mute_after_a_durable_flap_batch() {
        let dir = tempfile::tempdir_in("target").unwrap();
        let path = dir.path().join("notification.db");
        let store = maintenance_store();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let alert = maintenance_alert(1, NotificationSeverity::Warning);
        {
            let notifier = maintenance_notifier(store.clone(), url.clone())
                .with_outbox(Outbox::open(&path).unwrap());
            notifier.enqueue(alert).unwrap();
        }
        store
            .set_maintenance(
                Some(1),
                true,
                None,
                "work",
                "test",
                chrono::Utc::now().timestamp(),
            )
            .unwrap();
        let mut notifier =
            maintenance_notifier(store, url).with_outbox(Outbox::open(&path).unwrap());
        assert!(notifier.flush_all().await.unwrap().is_empty());
        assert_no_request(&listener);
        let status: String = rusqlite::Connection::open(path)
            .unwrap()
            .query_row("SELECT status FROM notification_outbox", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status, "suppressed");
    }

    #[tokio::test]
    async fn explicit_syslog_tests_deliver_to_local_udp_and_tcp_receivers() {
        assert_eq!(NotifierConfig::default().syslog.app_name, "tracepulse");
        for (format, expected) in [
            (SyslogFormat::Cef, "CEF:0|TracePulse|tracepulse|"),
            (SyslogFormat::Leef, "LEEF:2.0|TracePulse|tracepulse|"),
            (SyslogFormat::Rfc5424, ">1 "),
            (SyslogFormat::Rfc3164, "TracePulse:"),
        ] {
            let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut config = NotifierConfig::default();
            config.syslog.enabled = true;
            config.syslog.host = "127.0.0.1".into();
            config.syslog.port = socket.local_addr().unwrap().port();
            config.syslog.format = format;
            config.syslog.transport = SyslogTransport::Udp;
            send_test_notification(&config, NotificationChannel::Syslog)
                .await
                .unwrap();
            let mut buffer = [0; 4096];
            let size = socket.recv(&mut buffer).unwrap();
            assert!(
                std::str::from_utf8(&buffer[..size])
                    .unwrap()
                    .contains(expected)
            );

            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            config.syslog.port = listener.local_addr().unwrap().port();
            config.syslog.transport = SyslogTransport::Tcp;
            send_test_notification(&config, NotificationChannel::Syslog)
                .await
                .unwrap();
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut framed = String::new();
            stream.read_to_string(&mut framed).unwrap();
            let (length, message) = framed.split_once(' ').unwrap();
            assert_eq!(length.parse::<usize>().unwrap(), message.len());
            assert!(message.contains(expected));
        }
    }

    #[test]
    fn expands_env_placeholders_inside_webhook_url() {
        unsafe { env::set_var("TRACEPULSE_TEST_WEBHOOK", "example-token") };

        let resolved = expand_env_placeholders("https://hooks.example/${TRACEPULSE_TEST_WEBHOOK}")
            .expect("placeholder should expand");

        assert_eq!(resolved, "https://hooks.example/example-token");
    }

    #[test]
    fn parses_notifier_config_from_toml_with_env_placeholders() {
        let config = NotifierConfig::from_toml_str(
            r#"
            [slack]
            enabled = true
            webhook_url = "${TRACEPULSE_SLACK_WEBHOOK_URL}"

            [teams]
            enabled = false

            [flap_guard]
            window_seconds = 15

            [retry]
            max_attempts = 4
            "#,
        )
        .expect("config should parse");

        assert!(config.slack.enabled);
        assert_eq!(config.flap_guard.window_seconds, 15);
        assert_eq!(config.retry.max_attempts, 4);
    }

    #[test]
    fn parses_notifier_config_from_nested_app_config_section() {
        let config = NotifierConfig::from_toml_str(
            r#"
            [polling]
            interval_seconds = 30

            [notifier.slack]
            enabled = true
            webhook_url_env = "SLACK_WEBHOOK_URL"

            [notifier.flap_guard]
            window_seconds = 20
            "#,
        )
        .expect("nested notifier section should parse");

        assert!(config.slack.enabled);
        assert_eq!(
            config.slack.webhook_url_env.as_deref(),
            Some("SLACK_WEBHOOK_URL")
        );
        assert_eq!(config.flap_guard.window_seconds, 20);
    }

    #[test]
    fn parses_destination_card_settings() {
        let config = NotifierConfig::from_toml_str(
            r#"
            [syslog]
            enabled = true
            host = "192.0.2.10"
            port = 6514
            transport = "tls"
            format = "rfc5424"
            app_name = "tp"

            [smtp]
            enabled = true
            host = "mail.example.test"
            port = 587
            from = "tracepulse@example.test"
            to = ["noc@example.test"]
            starttls = true
            "#,
        )
        .expect("destination settings should parse");

        assert!(config.syslog.enabled);
        assert_eq!(config.syslog.transport, SyslogTransport::Tls);
        assert_eq!(config.syslog.format, SyslogFormat::Rfc5424);
        assert!(config.smtp.enabled);
        assert_eq!(config.smtp.to, vec!["noc@example.test"]);
    }

    #[test]
    fn resolved_events_are_info_and_immediate() {
        let device = DeviceConfig {
            id: Some(7),
            name: "edge-1".to_string(),
            ip: "192.0.2.7".to_string(),
            community: "public".to_string(),
            device_type: "router".to_string(),
            status: "online".to_string(),
            last_seen_at: None,
        };
        let notification = AlertNotification::from_event(AlertEvent::custom(
            "RESOLVED_IF_ERROR_RATE_SPIKE",
            &device,
            3,
            "xe-0/0/3",
            "normal",
            "error ratio > 0.001% or positive acceleration",
            "[RESOLVED] IF error rate returned to normal",
        ));

        assert!(notification.is_resolved());
        assert!(notification.is_immediate());
        assert_eq!(notification.severity, NotificationSeverity::Info);
        assert!(notification.title.starts_with("[RESOLVED]"));
    }

    #[test]
    fn offline_events_are_critical_and_keep_reachability_actions() {
        let device = DeviceConfig {
            id: Some(7),
            name: "edge-1".to_string(),
            ip: "192.0.2.7".to_string(),
            community: "public".to_string(),
            device_type: "router".to_string(),
            status: "offline".to_string(),
            last_seen_at: None,
        };
        let notification =
            AlertNotification::from_event(AlertEvent::device_offline(&device, "SNMP timeout"));

        assert_eq!(notification.alert_kind, "DEVICE_OFFLINE");
        assert_eq!(notification.severity, NotificationSeverity::Critical);
        assert!(notification.is_immediate());
        assert!(
            notification
                .recommended_actions
                .iter()
                .any(|action| action.contains("reachability"))
        );
    }

    #[tokio::test]
    async fn core_alert_event_is_delivered_to_slack_and_teams_webhooks() {
        let (slack_url, slack_body) = spawn_webhook_server();
        let (teams_url, teams_body) = spawn_webhook_server();
        let config = NotifierConfig {
            slack: WebhookEndpointConfig {
                enabled: true,
                webhook_url: Some(slack_url),
                webhook_url_env: None,
            },
            teams: WebhookEndpointConfig {
                enabled: true,
                webhook_url: Some(teams_url),
                webhook_url_env: None,
            },
            webhook: WebhookEndpointConfig::default(),
            pagerduty: PagerDutyConfig::default(),
            syslog: SyslogConfig::default(),
            smtp: SmtpConfig::default(),
            flap_guard: FlapGuardConfig::default(),
            retry: RetryConfig { max_attempts: 1 },
            send_resolved: true,
        };
        let broadcaster = AlertBroadcaster::new();
        let receiver = broadcaster.subscribe();
        let listener = tokio::spawn(run_alert_listener(receiver, config));
        let device = DeviceConfig {
            id: Some(42),
            name: "edge-core-1".to_string(),
            ip: "192.0.2.42".to_string(),
            community: "public".to_string(),
            device_type: "router".to_string(),
            status: "offline".to_string(),
            last_seen_at: None,
        };

        broadcaster.publish(AlertEvent::device_offline(&device, "SNMP timeout"));
        drop(broadcaster);

        tokio::time::timeout(Duration::from_secs(3), listener)
            .await
            .expect("listener should stop after broadcaster is dropped")
            .expect("listener task should not panic")
            .expect("listener should deliver webhooks");

        let slack_payload = slack_body
            .recv_timeout(Duration::from_secs(1))
            .expect("Slack webhook should receive a payload");
        let teams_payload = teams_body
            .recv_timeout(Duration::from_secs(1))
            .expect("Teams webhook should receive a payload");

        assert!(slack_payload.contains("DEVICE_OFFLINE"));
        assert!(slack_payload.contains("edge-core-1"));
        assert!(teams_payload.contains("AdaptiveCard"));
        assert!(teams_payload.contains("SNMP timeout"));
    }

    fn maintenance_store() -> Arc<crate::maintenance::MaintenanceStore> {
        Arc::new(crate::maintenance::MaintenanceStore::test_fixture())
    }

    fn maintenance_alert(device_id: i64, severity: NotificationSeverity) -> AlertNotification {
        let mut alert = test_alert_batch().last_alert;
        alert.id = format!("maintenance-{device_id}");
        alert.device_id = Some(device_id);
        alert.severity = severity;
        alert
    }

    fn maintenance_notifier(
        store: Arc<crate::maintenance::MaintenanceStore>,
        url: String,
    ) -> Notifier {
        Notifier::new(NotifierConfig {
            webhook: WebhookEndpointConfig {
                enabled: true,
                webhook_url: Some(url),
                webhook_url_env: None,
            },
            retry: RetryConfig { max_attempts: 3 },
            ..NotifierConfig::default()
        })
        .unwrap()
        .with_maintenance(store)
    }

    fn assert_no_request(listener: &TcpListener) {
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "maintenance-suppressed notification must not open an HTTP connection"
        );
    }

    #[tokio::test]
    async fn maintenance_global_mute_drops_alert_before_flap_buffering() {
        let store = maintenance_store();
        let now = chrono::Utc::now().timestamp();
        store
            .set_maintenance(None, true, None, "planned work", "test", now)
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut notifier = maintenance_notifier(
            store.clone(),
            format!("http://{}", listener.local_addr().unwrap()),
        );
        assert!(
            notifier
                .ingest(maintenance_alert(1, NotificationSeverity::Warning))
                .await
                .unwrap()
                .is_empty()
        );
        assert!(notifier.flap_guard.flush_all().is_empty());
        store
            .set_maintenance(None, false, None, "finished", "test", now + 1)
            .unwrap();
        assert!(notifier.flush_all().await.unwrap().is_empty());
        assert_no_request(&listener);
    }

    #[tokio::test]
    async fn maintenance_mute_after_buffering_suppresses_flush_but_not_other_device() {
        let store = maintenance_store();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut notifier = maintenance_notifier(
            store.clone(),
            format!("http://{}", listener.local_addr().unwrap()),
        );
        assert!(
            notifier
                .ingest(maintenance_alert(1, NotificationSeverity::Warning))
                .await
                .unwrap()
                .is_empty()
        );
        store
            .set_maintenance(
                Some(1),
                true,
                None,
                "planned work",
                "test",
                chrono::Utc::now().timestamp(),
            )
            .unwrap();
        assert!(notifier.flush_all().await.unwrap().is_empty());
        assert_no_request(&listener);

        let (url, received) = spawn_webhook_server();
        notifier.config.generic_webhook_url = Some(url);
        let delivered = notifier
            .ingest(maintenance_alert(2, NotificationSeverity::Critical))
            .await
            .unwrap();
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].status_code, 200);
        received.recv_timeout(Duration::from_secs(1)).unwrap();
    }

    #[tokio::test]
    async fn maintenance_expiry_allows_new_alerts_but_not_historical_alerts() {
        let store = maintenance_store();
        let now = chrono::Utc::now().timestamp();
        store
            .set_maintenance(
                Some(1),
                true,
                Some(3600),
                "planned work",
                "test",
                now - 7200,
            )
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut notifier = maintenance_notifier(
            store.clone(),
            format!("http://{}", listener.local_addr().unwrap()),
        );
        let mut historical = maintenance_alert(1, NotificationSeverity::Critical);
        historical.occurred_at = chrono::DateTime::from_timestamp(now - 7000, 0)
            .unwrap()
            .to_rfc3339();
        assert!(notifier.ingest(historical).await.unwrap().is_empty());
        assert_no_request(&listener);

        let (url, received) = spawn_webhook_server();
        notifier.config.generic_webhook_url = Some(url);
        let delivered = notifier
            .ingest(maintenance_alert(1, NotificationSeverity::Critical))
            .await
            .unwrap();
        assert_eq!(delivered.len(), 1);
        received.recv_timeout(Duration::from_secs(1)).unwrap();
    }

    #[tokio::test]
    async fn maintenance_gate_failure_is_an_error_and_never_sends() {
        let store = maintenance_store();
        let poisoned = store.clone();
        assert!(
            thread::spawn(move || {
                let _lock = poisoned.database.lock().unwrap();
                panic!("simulate unavailable maintenance database");
            })
            .join()
            .is_err()
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut notifier =
            maintenance_notifier(store, format!("http://{}", listener.local_addr().unwrap()));
        assert!(matches!(
            notifier
                .ingest(maintenance_alert(1, NotificationSeverity::Critical))
                .await,
            Err(NotifierError::GateUnavailable(_))
        ));
        assert_no_request(&listener);
    }

    #[tokio::test]
    async fn maintenance_mute_between_http_attempts_prevents_retry_for_every_http_channel() {
        use std::sync::atomic::{AtomicBool, Ordering};

        const ROUTING_KEY_ENV: &str = "TRACEPULSE_TEST_MAINTENANCE_ROUTING_KEY";
        unsafe { env::set_var(ROUTING_KEY_ENV, "test-routing-key") };
        for channel in [
            NotificationChannel::Slack,
            NotificationChannel::Teams,
            NotificationChannel::Webhook,
            NotificationChannel::PagerDuty,
        ] {
            let store = maintenance_store();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let server_store = store.clone();
            let finished = Arc::new(AtomicBool::new(false));
            let server_finished = finished.clone();
            let server = thread::spawn(move || {
                listener.set_nonblocking(true).unwrap();
                let deadline = std::time::Instant::now() + Duration::from_secs(10);
                let mut requests = 0;
                while !server_finished.load(Ordering::SeqCst)
                    && std::time::Instant::now() < deadline
                {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream.set_nonblocking(false).unwrap();
                            stream
                                .set_read_timeout(Some(Duration::from_secs(5)))
                                .unwrap();
                            read_webhook_body(&mut stream);
                            requests += 1;
                            server_store
                                .set_maintenance(
                                    Some(1),
                                    true,
                                    None,
                                    "enabled during retry",
                                    "test",
                                    chrono::Utc::now().timestamp(),
                                )
                                .unwrap();
                            stream.write_all(
                                b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                            ).unwrap();
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(error) => panic!("mock HTTP server failed: {error}"),
                    }
                }
                (requests, listener)
            });
            let mut notifier = maintenance_notifier(store, url.clone());
            notifier.config.generic_webhook_url = None;
            match channel {
                NotificationChannel::Slack => notifier.config.slack_webhook_url = Some(url),
                NotificationChannel::Teams => notifier.config.teams_webhook_url = Some(url),
                NotificationChannel::Webhook => notifier.config.generic_webhook_url = Some(url),
                NotificationChannel::PagerDuty => {
                    notifier.config.pagerduty = Some(PagerDutyConfig {
                        enabled: true,
                        endpoint: url,
                        integration_key_env: ROUTING_KEY_ENV.to_string(),
                    });
                }
                _ => unreachable!(),
            }
            let result = notifier
                .ingest(maintenance_alert(1, NotificationSeverity::Critical))
                .await;
            finished.store(true, Ordering::SeqCst);
            let (requests, listener) = server.join().unwrap();
            assert!(
                matches!(&result, Ok(delivered) if delivered.is_empty()),
                "{channel:?}: {result:?}"
            );
            assert_eq!(requests, 1, "{channel:?} retried after maintenance started");
            assert_no_request(&listener);
        }
    }

    fn spawn_webhook_server() -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let url = format!("http://{}", listener.local_addr().expect("local address"));
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("webhook request should arrive");
            let body = read_webhook_body(&mut stream);
            sender.send(body).expect("test should receive webhook body");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK")
                .expect("response should be writable");
        });

        (url, receiver)
    }

    fn read_webhook_body(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        let headers_end = loop {
            let read = stream
                .read(&mut buffer)
                .expect("request should be readable");
            if read == 0 {
                panic!("HTTP request ended before its headers");
            }
            request.extend_from_slice(&buffer[..read]);
            if let Some(position) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                break position + 4;
            }
        };

        let headers = String::from_utf8_lossy(&request[..headers_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        while request.len().saturating_sub(headers_end) < content_length {
            let read = stream.read(&mut buffer).expect("body should be readable");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }

        let body_end = headers_end + content_length.min(request.len().saturating_sub(headers_end));
        String::from_utf8_lossy(&request[headers_end..body_end]).to_string()
    }
}
