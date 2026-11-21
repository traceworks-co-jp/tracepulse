use crate::notifications::{
    NotificationChannelStatus, NotificationSettingsProvider, NotificationStatus,
};
use crate::notifier::{
    FlapGuardConfig, NotificationChannel, NotifierConfig, RetryConfig, WebhookEndpointConfig,
    send_test_notification,
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const MAX_FLAP_WINDOW_SECONDS: u64 = 3_600;
const MAX_RETRY_ATTEMPTS: u32 = 10;

/// Web UI から通知設定を読み書きし、稼働中のアラートパイプラインへ反映させる共有ストア。
pub struct NotifierSettingsStore {
    config_path: PathBuf,
    config: Arc<Mutex<NotifierConfig>>,
    generation: Arc<AtomicU64>,
}

impl NotifierSettingsStore {
    pub fn new(config_path: impl Into<PathBuf>, config: NotifierConfig) -> Self {
        Self {
            config_path: config_path.into(),
            config: Arc::new(Mutex::new(config)),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 通知パイプライン側が最新設定を参照するためのハンドル。
    pub fn handle(&self) -> NotifierSettingsHandle {
        NotifierSettingsHandle {
            config: Arc::clone(&self.config),
            generation: Arc::clone(&self.generation),
        }
    }

    fn snapshot(&self) -> Result<NotifierConfig, String> {
        self.config
            .lock()
            .map(|config| config.clone())
            .map_err(|_| "notifier config lock failed".to_string())
    }
}

/// 設定の世代番号を監視して、変更時に Notifier を作り直すためのハンドル。
#[derive(Clone)]
pub struct NotifierSettingsHandle {
    config: Arc<Mutex<NotifierConfig>>,
    generation: Arc<AtomicU64>,
}

impl NotifierSettingsHandle {
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    pub fn current(&self) -> Option<NotifierConfig> {
        self.config.lock().ok().map(|config| config.clone())
    }
}

impl NotificationSettingsProvider for NotifierSettingsStore {
    fn send_resolved_enabled(&self) -> Result<bool, String> {
        Ok(self.snapshot()?.send_resolved)
    }

    fn load_json(&self) -> String {
        match self.snapshot() {
            Ok(config) => match serde_json::to_value(config) {
                Ok(mut value) => {
                    for channel in ["slack", "teams", "webhook"] {
                        if let Some(endpoint) =
                            value.get_mut(channel).and_then(Value::as_object_mut)
                        {
                            endpoint.remove("webhook_url");
                        }
                    }
                    serde_json::to_string(&value)
                        .unwrap_or_else(|err| format!(r#"{{"error":"{err}"}}"#))
                }
                Err(err) => format!(r#"{{"error":"{err}"}}"#),
            },
            Err(err) => format!(r#"{{"error":"{err}"}}"#),
        }
    }

    fn save_json(&self, body: &str) -> Result<(), String> {
        let parsed: Value = serde_json::from_str(body).map_err(|err| err.to_string())?;
        let config = parse_notifier_config(&parsed, &self.snapshot()?)?;

        persist_notifier_config(&self.config_path, &config)?;

        let mut current = self
            .config
            .lock()
            .map_err(|_| "notifier config lock failed".to_string())?;
        *current = config;
        drop(current);
        self.generation.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn send_test(&self, channel: &str) -> Result<String, String> {
        let config = self.snapshot()?;
        let targets = match channel {
            "slack" => {
                if !config.slack.enabled {
                    return Err("Slack notifications are disabled".to_string());
                }
                vec![NotificationChannel::Slack]
            }
            "teams" => {
                if !config.teams.enabled {
                    return Err("Teams notifications are disabled".to_string());
                }
                vec![NotificationChannel::Teams]
            }
            "syslog" => {
                if !config.syslog.enabled {
                    return Err("Syslog notifications are disabled".to_string());
                }
                vec![NotificationChannel::Syslog]
            }
            "webhook" => {
                if !config.webhook.enabled {
                    return Err("Generic webhook notifications are disabled".to_string());
                }
                vec![NotificationChannel::Webhook]
            }
            "smtp" => {
                if !config.smtp.enabled {
                    return Err("SMTP notifications are disabled".to_string());
                }
                vec![NotificationChannel::Smtp]
            }
            "pagerduty" => {
                if !config.pagerduty.enabled {
                    return Err("PagerDuty notifications are disabled".to_string());
                }
                vec![NotificationChannel::PagerDuty]
            }
            "all" => {
                let mut targets = Vec::new();
                if config.slack.enabled {
                    targets.push(NotificationChannel::Slack);
                }
                if config.teams.enabled {
                    targets.push(NotificationChannel::Teams);
                }
                if config.syslog.enabled {
                    targets.push(NotificationChannel::Syslog);
                }
                if config.webhook.enabled {
                    targets.push(NotificationChannel::Webhook);
                }
                if config.smtp.enabled {
                    targets.push(NotificationChannel::Smtp);
                }
                if config.pagerduty.enabled {
                    targets.push(NotificationChannel::PagerDuty);
                }
                if targets.is_empty() {
                    return Err("no notification channel is enabled".to_string());
                }
                targets
            }
            other => return Err(format!("unknown channel: {other}")),
        };

        run_blocking_test(config, targets)
    }

    fn status(&self) -> NotificationStatus {
        let Ok(config) = self.snapshot() else {
            return NotificationStatus::default();
        };

        NotificationStatus {
            channels: vec![
                channel_status(
                    "slack",
                    "Slack",
                    &config.slack,
                    "TRACEPULSE_SLACK_WEBHOOK_URL",
                ),
                channel_status(
                    "teams",
                    "Teams",
                    &config.teams,
                    "TRACEPULSE_TEAMS_WEBHOOK_URL",
                ),
            ],
            flap_window_seconds: config.flap_guard.window_seconds,
            retry_max_attempts: config.retry.max_attempts,
        }
    }
}

fn mask_webhook_url(url: &str) -> String {
    let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return "****".to_string();
    };

    let (host, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };

    let visible: String = path.chars().take(9).collect();
    if path.chars().count() > 9 {
        format!("{host}{visible}****")
    } else {
        format!("{host}{visible}")
    }
}

fn channel_status(
    key: &str,
    name: &str,
    endpoint: &WebhookEndpointConfig,
    fallback_env: &str,
) -> NotificationChannelStatus {
    let env_name = endpoint.webhook_url_env.as_deref().unwrap_or(fallback_env);

    let (webhook, source) = match endpoint.webhook_url.as_deref().map(str::trim) {
        // ${ENV} プレースホルダ自体は秘匿情報ではないのでそのまま見せる。
        Some(url) if url.starts_with("${") => (url.to_string(), "config.toml".to_string()),
        Some(url) if !url.is_empty() => (mask_webhook_url(url), "config.toml".to_string()),
        _ => match std::env::var(env_name) {
            Ok(url) if !url.trim().is_empty() => {
                (mask_webhook_url(url.trim()), format!("env: {env_name}"))
            }
            _ => (
                "(not configured)".to_string(),
                format!("env: {env_name} (unset)"),
            ),
        },
    };

    NotificationChannelStatus {
        key: key.to_string(),
        name: name.to_string(),
        enabled: endpoint.enabled,
        webhook,
        source,
    }
}

/// Web サーバのリクエストスレッドは tokio ランタイム上で動くため、
/// ネストした block_on を避けて専用スレッド + 単一スレッドランタイムで送信する。
fn run_blocking_test(
    config: NotifierConfig,
    targets: Vec<NotificationChannel>,
) -> Result<String, String> {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| err.to_string())?;

        runtime.block_on(async move {
            let mut summary = Vec::new();
            for channel in targets {
                let result = send_test_notification(&config, channel)
                    .await
                    .map_err(|err| err.to_string())?;
                summary.push(format!(
                    "{:?}: HTTP {} ({} attempt(s))",
                    result.channel, result.status_code, result.attempts
                ));
            }
            Ok(summary.join(" / "))
        })
    })
    .join()
    .map_err(|_| "test notification thread panicked".to_string())?
}

/// GUI が送らないキーは既存設定を維持する（バックオフなどは config.toml 側で調整する）。
fn parse_notifier_config(
    value: &Value,
    current: &NotifierConfig,
) -> Result<NotifierConfig, String> {
    let slack = parse_endpoint(
        value.get("slack"),
        "Slack",
        "TRACEPULSE_SLACK_WEBHOOK_URL",
        &current.slack,
    )?;
    let teams = parse_endpoint(
        value.get("teams"),
        "Teams",
        "TRACEPULSE_TEAMS_WEBHOOK_URL",
        &current.teams,
    )?;
    let webhook = parse_endpoint(
        value.get("webhook"),
        "Generic webhook",
        "TRACEPULSE_WEBHOOK_URL",
        &current.webhook,
    )?;
    let syslog = value
        .get("syslog")
        .map(|section| serde_json::from_value(section.clone()).map_err(|err| err.to_string()))
        .transpose()?
        .unwrap_or_else(|| current.syslog.clone());
    let smtp = value
        .get("smtp")
        .map(|section| serde_json::from_value(section.clone()).map_err(|err| err.to_string()))
        .transpose()?
        .unwrap_or_else(|| current.smtp.clone());
    let pagerduty = value
        .get("pagerduty")
        .map(|section| serde_json::from_value(section.clone()).map_err(|err| err.to_string()))
        .transpose()?
        .unwrap_or_else(|| current.pagerduty.clone());

    let window_seconds = value
        .pointer("/flap_guard/window_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(current.flap_guard.window_seconds);
    if window_seconds > MAX_FLAP_WINDOW_SECONDS {
        return Err(format!(
            "flap guard window must be 0-{MAX_FLAP_WINDOW_SECONDS} seconds"
        ));
    }

    let max_attempts = value
        .pointer("/retry/max_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(current.retry.max_attempts as u64) as u32;
    if !(1..=MAX_RETRY_ATTEMPTS).contains(&max_attempts) {
        return Err(format!("retry attempts must be 1-{MAX_RETRY_ATTEMPTS}"));
    }
    let send_resolved = value
        .get("send_resolved")
        .and_then(Value::as_bool)
        .unwrap_or(current.send_resolved);

    Ok(NotifierConfig {
        slack,
        teams,
        webhook,
        pagerduty,
        syslog,
        smtp,
        flap_guard: FlapGuardConfig { window_seconds },
        retry: RetryConfig { max_attempts },
        send_resolved,
    })
}

fn parse_endpoint(
    value: Option<&Value>,
    label: &str,
    fallback_env: &str,
    defaults: &WebhookEndpointConfig,
) -> Result<WebhookEndpointConfig, String> {
    let Some(value) = value else {
        return Ok(defaults.clone());
    };

    let enabled = value
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(defaults.enabled);
    let webhook_url = match value.get("webhook_url").and_then(Value::as_str) {
        Some(url) => non_empty(Some(url)).or_else(|| defaults.webhook_url.clone()),
        None => defaults.webhook_url.clone(),
    };
    let webhook_url_env = non_empty(value.get("webhook_url_env").and_then(Value::as_str))
        .or_else(|| Some(fallback_env.to_string()));

    if let Some(url) = &webhook_url {
        // ${ENV} プレースホルダ以外は https の Webhook URL に限定する。
        if !url.starts_with("${") && !url.starts_with("https://") {
            return Err(format!("{label} webhook URL must start with https://"));
        }
    }

    if enabled && webhook_url.is_none() && webhook_url_env.is_none() {
        return Err(format!(
            "{label} is enabled but neither a webhook URL nor an environment variable is set"
        ));
    }

    Ok(WebhookEndpointConfig {
        enabled,
        webhook_url,
        webhook_url_env,
    })
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// config.toml の [notifier] セクションだけを差し替え、他セクションは保持する。
fn persist_notifier_config(config_path: &Path, config: &NotifierConfig) -> Result<(), String> {
    let mut root: toml::value::Table = std::fs::read_to_string(config_path)
        .ok()
        .and_then(|content| content.parse::<toml::Table>().ok())
        .unwrap_or_default();

    root.insert("notifier".to_string(), notifier_to_toml(config));

    let rendered = toml::to_string_pretty(&root).map_err(|err| err.to_string())?;
    std::fs::write(config_path, rendered).map_err(|err| err.to_string())
}

fn notifier_to_toml(config: &NotifierConfig) -> toml::Value {
    let mut table = toml::value::Table::new();
    table.insert("slack".to_string(), endpoint_to_toml(&config.slack));
    table.insert("teams".to_string(), endpoint_to_toml(&config.teams));
    table.insert("webhook".to_string(), endpoint_to_toml(&config.webhook));
    table.insert(
        "pagerduty".to_string(),
        toml::Value::Table(toml::Table::from_iter([
            (
                "enabled".to_string(),
                toml::Value::Boolean(config.pagerduty.enabled),
            ),
            (
                "endpoint".to_string(),
                toml::Value::String(config.pagerduty.endpoint.clone()),
            ),
            (
                "integration_key_env".to_string(),
                toml::Value::String(config.pagerduty.integration_key_env.clone()),
            ),
        ])),
    );

    let mut syslog = toml::value::Table::new();
    syslog.insert(
        "enabled".to_string(),
        toml::Value::Boolean(config.syslog.enabled),
    );
    syslog.insert(
        "host".to_string(),
        toml::Value::String(config.syslog.host.clone()),
    );
    syslog.insert(
        "port".to_string(),
        toml::Value::Integer(config.syslog.port as i64),
    );
    syslog.insert(
        "transport".to_string(),
        toml::Value::String(format!("{:?}", config.syslog.transport).to_lowercase()),
    );
    syslog.insert(
        "format".to_string(),
        toml::Value::String(format!("{:?}", config.syslog.format).to_lowercase()),
    );
    syslog.insert(
        "app_name".to_string(),
        toml::Value::String(config.syslog.app_name.clone()),
    );
    table.insert("syslog".to_string(), toml::Value::Table(syslog));

    let mut smtp = toml::value::Table::new();
    smtp.insert(
        "enabled".to_string(),
        toml::Value::Boolean(config.smtp.enabled),
    );
    smtp.insert(
        "host".to_string(),
        toml::Value::String(config.smtp.host.clone()),
    );
    smtp.insert(
        "port".to_string(),
        toml::Value::Integer(config.smtp.port as i64),
    );
    smtp.insert(
        "from".to_string(),
        toml::Value::String(config.smtp.from.clone()),
    );
    smtp.insert(
        "starttls".to_string(),
        toml::Value::Boolean(config.smtp.starttls),
    );
    smtp.insert(
        "to".to_string(),
        toml::Value::Array(
            config
                .smtp
                .to
                .iter()
                .cloned()
                .map(toml::Value::String)
                .collect(),
        ),
    );
    if let Some(username) = &config.smtp.username {
        smtp.insert(
            "username".to_string(),
            toml::Value::String(username.clone()),
        );
    }
    if let Some(password_env) = &config.smtp.password_env {
        smtp.insert(
            "password_env".to_string(),
            toml::Value::String(password_env.clone()),
        );
    }
    table.insert("smtp".to_string(), toml::Value::Table(smtp));

    let mut flap_guard = toml::value::Table::new();
    flap_guard.insert(
        "window_seconds".to_string(),
        toml::Value::Integer(config.flap_guard.window_seconds as i64),
    );
    table.insert("flap_guard".to_string(), toml::Value::Table(flap_guard));

    let mut retry = toml::value::Table::new();
    retry.insert(
        "max_attempts".to_string(),
        toml::Value::Integer(config.retry.max_attempts as i64),
    );
    table.insert("retry".to_string(), toml::Value::Table(retry));
    table.insert(
        "send_resolved".to_string(),
        toml::Value::Boolean(config.send_resolved),
    );

    toml::Value::Table(table)
}

fn endpoint_to_toml(endpoint: &WebhookEndpointConfig) -> toml::Value {
    let mut table = toml::value::Table::new();
    table.insert(
        "enabled".to_string(),
        toml::Value::Boolean(endpoint.enabled),
    );
    if let Some(url) = &endpoint.webhook_url {
        table.insert("webhook_url".to_string(), toml::Value::String(url.clone()));
    }
    if let Some(env_name) = &endpoint.webhook_url_env {
        table.insert(
            "webhook_url_env".to_string(),
            toml::Value::String(env_name.clone()),
        );
    }
    toml::Value::Table(table)
}

#[cfg(test)]
mod tests {
    use super::mask_webhook_url;

    #[test]
    fn masks_webhook_secret_and_unknown_scheme() {
        assert_eq!(
            mask_webhook_url("https://hooks.example.com/services/T000/B000/XXXXXXXX"),
            "hooks.example.com/services****"
        );
        assert_eq!(mask_webhook_url("not-a-url"), "****");
    }
}
