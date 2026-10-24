use crate::alert::broadcaster::AlertBroadcaster;
use crate::alert::event::AlertEvent;
use crate::config::AppConfig;
use crate::db::models::InterfaceSample;
use crate::db::repository::Repository;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::monitor::system::SystemMonitor;
use crate::snmp::SnmpClient;
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct PollingEngine {
    pub config: AppConfig,
    pub repository: Repository,
    alerts: AlertBroadcaster,
}

impl PollingEngine {
    pub fn new(config: AppConfig, repository: Repository) -> Self {
        Self::with_broadcaster(config, repository, AlertBroadcaster::new())
    }

    pub fn with_broadcaster(
        config: AppConfig,
        repository: Repository,
        alerts: AlertBroadcaster,
    ) -> Self {
        Self {
            config,
            repository,
            alerts,
        }
    }

    /// 外部モジュールがアラートイベントを購読するためのレシーバーを返す。
    pub fn subscribe_alerts(&self) -> broadcast::Receiver<AlertEvent> {
        self.alerts.subscribe()
    }

    pub fn alert_broadcaster(&self) -> AlertBroadcaster {
        self.alerts.clone()
    }

    /// 検知したアラートを購読者へ配信する。
    pub fn publish_alert(&self, event: AlertEvent) -> usize {
        self.alerts.publish(event)
    }

    pub fn poll_device(
        &self,
        device: &DeviceConfig,
    ) -> Result<(SystemMonitor, Vec<InterfaceSample>), AppError> {
        let client =
            SnmpClient::with_snmp_config(device.community.clone(), self.config.snmp.clone());
        let info = client.query_device(device)?;
        let system = SystemMonitor::from_snmp_device_info(&info);

        let interface_indexes = client.discover_interface_indexes(device)?;
        let mut interfaces = Vec::new();

        for if_index in interface_indexes {
            match client.query_interface(device, if_index, None) {
                Ok(interface) => interfaces.push(interface.into_sample(0)),
                Err(_) => continue,
            }
        }

        if interfaces.is_empty() {
            for if_index in 1..=3 {
                match client.query_interface(device, if_index, None) {
                    Ok(interface) => interfaces.push(interface.into_sample(0)),
                    Err(_) => continue,
                }
            }
        }

        match client.query_dom(device) {
            Ok(powers) => {
                for interface in &mut interfaces {
                    if let Some(power) = powers.get(&(interface.if_index as u32)) {
                        interface.rx_optical_power_dbm = power.rx_power_dbm;
                        interface.tx_power_dbm = power.tx_power_dbm;
                    }
                }
            }
            Err(error) => {
                tracing::warn!(device = %device.ip, %error, "Optional DOM collection unavailable")
            }
        }
        Ok((system, interfaces))
    }
}
