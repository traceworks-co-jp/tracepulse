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

    pub fn publish_predictive_alerts(&self, device: &DeviceConfig, sample: &InterfaceSample) {
        let Ok(history) =
            self.repository
                .get_recent_interface_samples(sample.device_id, sample.if_index, 3)
        else {
            return;
        };
        let Some(indicators) = crate::monitor::predictive::evaluate_predictive(&history) else {
            return;
        };
        if indicators.error_ratio_warning {
            self.publish_alert(AlertEvent::predictive(
                crate::alert::AlertKind::PredictiveErrorRate,
                device,
                sample.if_index,
                &sample.if_name,
                format!("{:.6}%", indicators.error_ratio * 100.0),
                "0.001%",
                format!(
                    "[PRED] Error ratio rising on {}: {:.6}%",
                    sample.if_name,
                    indicators.error_ratio * 100.0
                ),
            ));
        }
        if indicators.trend_warning {
            self.publish_alert(AlertEvent::predictive(
                crate::alert::AlertKind::PredictiveTrend,
                device,
                sample.if_index,
                &sample.if_name,
                format!("{:.0}", indicators.error_acceleration),
                "> 0 errors/poll acceleration",
                format!(
                    "[PRED] Error acceleration rising on {}: {:.0}",
                    sample.if_name, indicators.error_acceleration
                ),
            ));
        }
        if indicators.dom_warning
            && let Some(power) = indicators.rx_optical_power_dbm
        {
            self.publish_alert(AlertEvent::predictive(
                crate::alert::AlertKind::PredictiveDom,
                device,
                sample.if_index,
                &sample.if_name,
                format!("{power:.1} dBm"),
                "-18 dBm",
                format!(
                    "[PRED] SFP Rx optical power degraded on {}: {power:.1} dBm",
                    sample.if_name
                ),
            ));
        }
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

        Ok((system, interfaces))
    }
}
