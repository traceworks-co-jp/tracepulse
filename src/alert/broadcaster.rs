use crate::alert::event::AlertEvent;
use tokio::sync::broadcast;

/// アラートイベントチャネルの既定バッファ数。
pub const ALERT_CHANNEL_CAPACITY: usize = 100;

/// アラートイベントの配信元。クローンしても同一チャネルを共有する。
#[derive(Debug, Clone)]
pub struct AlertBroadcaster {
    sender: broadcast::Sender<AlertEvent>,
}

impl AlertBroadcaster {
    pub fn new() -> Self {
        Self::with_capacity(ALERT_CHANNEL_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn from_sender(sender: broadcast::Sender<AlertEvent>) -> Self {
        Self { sender }
    }

    /// 外部モジュールが購読するためのレシーバーを取得する。
    pub fn subscribe(&self) -> broadcast::Receiver<AlertEvent> {
        self.sender.subscribe()
    }

    pub fn sender(&self) -> broadcast::Sender<AlertEvent> {
        self.sender.clone()
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// イベントを配信し、届いた購読者数を返す。購読者が居ない場合もエラーにしない。
    pub fn publish(&self, event: AlertEvent) -> usize {
        self.sender.send(event).unwrap_or(0)
    }
}

impl Default for AlertBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::event::AlertKind;
    use crate::alert::status::AlertSeverity;
    use crate::device::types::DeviceConfig;

    fn sample_device() -> DeviceConfig {
        DeviceConfig {
            id: Some(1),
            name: "sw-01".to_string(),
            ip: "192.168.0.1".to_string(),
            community: "public".to_string(),
            device_type: "switch".to_string(),
            status: "online".to_string(),
            last_seen_at: None,
        }
    }

    #[test]
    fn subscriber_receives_published_event() {
        let broadcaster = AlertBroadcaster::new();
        let mut receiver = broadcaster.subscribe();

        let device = sample_device();
        broadcaster.publish(AlertEvent::new(
            AlertKind::InterfaceSpike,
            AlertSeverity::Warning,
            &device,
            "spike detected",
        ));

        let event = receiver.try_recv().expect("event should be delivered");
        assert_eq!(event.kind, AlertKind::InterfaceSpike);
        assert_eq!(event.device_id, Some(1));
        assert_eq!(event.device_ip, "192.168.0.1");
    }

    #[test]
    fn publish_without_subscriber_is_not_an_error() {
        let broadcaster = AlertBroadcaster::new();
        let device = sample_device();
        let delivered = broadcaster.publish(AlertEvent::device_offline(&device, "timeout"));
        assert_eq!(delivered, 0);
    }
}
