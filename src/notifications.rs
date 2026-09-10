//! Web GUI / TUI から通知チャンネル設定を参照するための、エディション非依存の拡張点。

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
