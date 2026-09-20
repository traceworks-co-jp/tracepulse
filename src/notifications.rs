//! Web GUI / TUI から通知チャンネル設定を参照するための、エディション非依存の拡張点。

use crate::alert::AlertEvent;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 1 チャンネル分の設定状態。Webhook URL は表示前にマスクされる。
#[derive(Debug, Clone)]
pub struct NotificationChannelStatus {
    pub key: String,
    pub name: String,
    pub enabled: bool,
    pub webhook: String,
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationStatus {
    pub channels: Vec<NotificationChannelStatus>,
    pub flap_window_seconds: u64,
    pub retry_max_attempts: u32,
}

/// 通知チャンネル設定の読み書きと疎通テストを担う実装を、上位エディションから差し込むための拡張点。
pub trait NotificationSettingsProvider: Send + Sync {
    /// 現在の通知設定を JSON 文字列で返す。
    fn load_json(&self) -> String;
    /// JSON 形式の設定を検証・永続化する。
    fn save_json(&self, body: &str) -> Result<(), String>;
    /// `slack` / `teams` / `all` のいずれかへテスト通知を送り、結果メッセージを返す。
    fn send_test(&self, channel: &str) -> Result<String, String>;
    /// TUI 表示向けの、マスク済み設定サマリーを返す。
    fn status(&self) -> NotificationStatus;
    /// Core が発行したアラートを通知チャンネルへ配送する。
    fn dispatch(&self, _event: &AlertEvent) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
struct WebhookChannel {
    enabled: bool,
    webhook_url: String,
    webhook_url_env: String,
}

impl Default for WebhookChannel {
    fn default() -> Self {
        Self {
            enabled: false,
            webhook_url: String::new(),
            webhook_url_env: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
struct CommunityNotificationConfig {
    slack: WebhookChannel,
    teams: WebhookChannel,
    flap_guard: FlapGuardConfig,
    retry: RetryConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
struct FlapGuardConfig {
    window_seconds: u64,
}

impl Default for FlapGuardConfig {
    fn default() -> Self {
        Self { window_seconds: 0 }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
struct RetryConfig {
    max_attempts: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self { max_attempts: 1 }
    }
}

impl Default for CommunityNotificationConfig {
    fn default() -> Self {
        Self {
            slack: WebhookChannel {
                webhook_url_env: "TRACEPULSE_SLACK_WEBHOOK_URL".to_string(),
                ..WebhookChannel::default()
            },
            teams: WebhookChannel {
                webhook_url_env: "TRACEPULSE_TEAMS_WEBHOOK_URL".to_string(),
                ..WebhookChannel::default()
            },
            flap_guard: FlapGuardConfig::default(),
            retry: RetryConfig::default(),
        }
    }
}

pub struct CommunityNotificationSettings {
    config_path: PathBuf,
    client: reqwest::blocking::Client,
    last_sent: Mutex<std::collections::HashMap<String, std::time::Instant>>,
}

impl CommunityNotificationSettings {
    pub fn new(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
            client: reqwest::blocking::Client::new(),
            last_sent: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn load_config(&self) -> Result<CommunityNotificationConfig, String> {
        let root = read_config_table(&self.config_path)?;
        root.get("notifications")
            .cloned()
            .map(|value| value.try_into().map_err(|error| error.to_string()))
            .transpose()
            .map(|config| config.unwrap_or_default())
    }

    fn save_config(&self, config: &CommunityNotificationConfig) -> Result<(), String> {
        let mut root = read_config_table(&self.config_path)?;
        let value = toml::Value::try_from(config).map_err(|error| error.to_string())?;
        root.insert("notifications".to_string(), value);
        let content = toml::to_string_pretty(&root).map_err(|error| error.to_string())?;
        std::fs::write(&self.config_path, content).map_err(|error| error.to_string())
    }

    fn send_to_channel(
        &self,
        name: &str,
        channel: &WebhookChannel,
        message: &str,
    ) -> Result<(), String> {
        if !channel.enabled {
            return Ok(());
        }
        let url = resolve_webhook(channel)?;
        let attempts = self.load_config()?.retry.max_attempts.max(1);
        let mut last_error = String::new();
        for _ in 0..attempts {
            match self
                .client
                .post(&url)
                .json(&json!({ "text": message }))
                .send()
            {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) => {
                    last_error = format!("{name} webhook returned HTTP {}", response.status())
                }
                Err(error) => last_error = error.to_string(),
            }
        }
        Err(last_error)
    }
}

impl NotificationSettingsProvider for CommunityNotificationSettings {
    fn load_json(&self) -> String {
        match self.load_config() {
            Ok(mut config) => {
                config.slack.webhook_url.clear();
                config.teams.webhook_url.clear();
                serde_json::to_string(&config)
                    .unwrap_or_else(|error| json!({ "error": error.to_string() }).to_string())
            }
            Err(error) => json!({ "error": error }).to_string(),
        }
    }

    fn save_json(&self, body: &str) -> Result<(), String> {
        let mut config: CommunityNotificationConfig =
            serde_json::from_str(body).map_err(|error| error.to_string())?;
        let existing = self.load_config()?;
        if config.slack.webhook_url.trim().is_empty() {
            config.slack.webhook_url = existing.slack.webhook_url;
        }
        if config.teams.webhook_url.trim().is_empty() {
            config.teams.webhook_url = existing.teams.webhook_url;
        }
        validate_config(&config)?;
        self.save_config(&config)
    }

    fn send_test(&self, channel: &str) -> Result<String, String> {
        let config = self.load_config()?;
        let channels: Vec<(&str, &WebhookChannel)> = match channel {
            "slack" => vec![("slack", &config.slack)],
            "teams" => vec![("teams", &config.teams)],
            "all" => vec![("slack", &config.slack), ("teams", &config.teams)],
            _ => return Err(format!("unknown notification channel: {channel}")),
        };
        let mut sent = Vec::new();
        for (name, channel) in channels {
            self.send_to_channel(name, channel, "TracePulse notification test")?;
            sent.push(name);
        }
        Ok(format!("test notification sent: {}", sent.join(", ")))
    }

    fn status(&self) -> NotificationStatus {
        let Ok(config) = self.load_config() else {
            return NotificationStatus::default();
        };
        NotificationStatus {
            channels: vec![
                channel_status("slack", "Slack", &config.slack),
                channel_status("teams", "Teams", &config.teams),
            ],
            flap_window_seconds: config.flap_guard.window_seconds,
            retry_max_attempts: config.retry.max_attempts,
        }
    }

    fn dispatch(&self, event: &AlertEvent) -> Result<(), String> {
        let config = self.load_config()?;
        let key = format!("{}:{}", event.device_ip, event.kind);
        if config.flap_guard.window_seconds > 0 {
            let mut last_sent = self
                .last_sent
                .lock()
                .map_err(|_| "notification state lock failed".to_string())?;
            if let Some(previous) = last_sent.get(&key)
                && previous.elapsed().as_secs() < config.flap_guard.window_seconds
            {
                return Ok(());
            }
            last_sent.insert(key, std::time::Instant::now());
        }
        let message = format!(
            "{}\nDevice: {} ({})\n{}",
            event.kind, event.device_name, event.device_ip, event.message
        );
        let mut errors = Vec::new();
        for (name, channel) in [("slack", &config.slack), ("teams", &config.teams)] {
            if let Err(error) = self.send_to_channel(name, channel, &message) {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}

fn read_config_table(path: &Path) -> Result<toml::Table, String> {
    if !path.exists() {
        return Ok(toml::Table::new());
    }
    std::fs::read_to_string(path)
        .map_err(|error| error.to_string())?
        .parse::<toml::Table>()
        .map_err(|error| error.to_string())
}

fn resolve_webhook(channel: &WebhookChannel) -> Result<String, String> {
    let url = if channel.webhook_url.trim().is_empty() {
        std::env::var(&channel.webhook_url_env)
            .map_err(|_| "enabled notification channel has no webhook URL".to_string())?
    } else {
        channel.webhook_url.trim().to_string()
    };
    if url.starts_with("https://") {
        Ok(url)
    } else {
        Err("webhook URL must use https".to_string())
    }
}

fn validate_config(config: &CommunityNotificationConfig) -> Result<(), String> {
    if config.flap_guard.window_seconds > 3_600 {
        return Err("flap guard window must be 0-3600 seconds".to_string());
    }
    if !(1..=10).contains(&config.retry.max_attempts) {
        return Err("retry attempts must be 1-10".to_string());
    }
    for (name, channel) in [("Slack", &config.slack), ("Teams", &config.teams)] {
        if channel.enabled
            && channel.webhook_url.trim().is_empty()
            && channel.webhook_url_env.trim().is_empty()
        {
            return Err(format!(
                "{name} is enabled but no webhook URL is configured"
            ));
        }
        if !channel.webhook_url.trim().is_empty() && !channel.webhook_url.starts_with("https://") {
            return Err(format!("{name} webhook URL must use https"));
        }
    }
    Ok(())
}

fn channel_status(key: &str, name: &str, channel: &WebhookChannel) -> NotificationChannelStatus {
    NotificationChannelStatus {
        key: key.to_string(),
        name: name.to_string(),
        enabled: channel.enabled,
        webhook: mask_webhook_url(&channel.webhook_url),
        source: if channel.webhook_url.is_empty() {
            format!("env: {}", channel.webhook_url_env)
        } else {
            "config.toml".to_string()
        },
    }
}

/// Webhook URL をホストと先頭数文字だけ残してマスクする。
pub fn mask_webhook_url(url: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_the_secret_portion_of_a_webhook_url() {
        let masked = mask_webhook_url("https://hooks.slack.com/services/T000/B000/XXXXXXXX");

        assert_eq!(masked, "hooks.slack.com/services****");
    }

    #[test]
    fn masks_unknown_schemes_entirely() {
        assert_eq!(mask_webhook_url("not-a-url"), "****");
    }
}
