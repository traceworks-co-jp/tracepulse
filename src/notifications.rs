//! Edition-neutral notification provider contract.

use crate::alert::AlertEvent;

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

pub trait NotificationSettingsProvider: Send + Sync {
    fn load_json(&self) -> String;
    fn save_json(&self, body: &str) -> Result<(), String>;
    fn send_test(&self, channel: &str) -> Result<String, String>;
    fn status(&self) -> NotificationStatus;

    fn dispatch(&self, _event: &AlertEvent) -> Result<(), String> {
        Ok(())
    }
}
