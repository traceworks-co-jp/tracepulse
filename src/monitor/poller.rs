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

    pub fn update_device_status(
        &self,
        device: &DeviceConfig,
        status: &str,
        polling_succeeded: bool,
    ) -> Result<(), AppError> {
        if let Some(event) = self.repository.update_device_status_with_transition(
            device,
            status,
            polling_succeeded,
        )? {
            self.alerts.publish(event);
        }
        Ok(())
    }

    pub fn poll_device(
        &self,
        device: &DeviceConfig,
    ) -> Result<(SystemMonitor, Vec<InterfaceSample>), AppError> {
        let config = self.repository.resolved_snmp_config(
            &device.ip,
            &self.config.snmp,
            &device.community,
        )?;
        let client = SnmpClient::with_snmp_config(config.default_community.clone(), config);
        let info = client.query_device(device)?;
        if let Some(oid) = &info.sys_object_id
            && let Err(error) = self.repository.save_device_sys_object_id(&device.ip, oid)
        {
            tracing::warn!(device = %device.ip, %error, "SNMP identity could not be saved");
        }
        let system = SystemMonitor::from_snmp_device_info(&info);

        let interface_indexes = client.discover_interface_indexes(device)?;
        let mut interfaces = Vec::new();

        for if_index in interface_indexes {
            match client.query_interface(device, if_index, None) {
                Ok(interface) => {
                    if device.id.is_some()
                        && let Some(event) = self.repository.observe_port_link(
                            device,
                            interface.if_index,
                            &interface.if_name,
                            interface.if_admin_status,
                            interface.if_oper_status,
                            chrono::Utc::now().timestamp(),
                        )?
                    {
                        self.alerts.publish(event);
                    }
                    interfaces.push(interface.into_sample(0));
                }
                Err(_) => continue,
            }
        }

        if interfaces.is_empty() {
            for if_index in 1..=3 {
                match client.query_interface(device, if_index, None) {
                    Ok(interface) => {
                        if device.id.is_some()
                            && let Some(event) = self.repository.observe_port_link(
                                device,
                                interface.if_index,
                                &interface.if_name,
                                interface.if_admin_status,
                                interface.if_oper_status,
                                chrono::Utc::now().timestamp(),
                            )?
                        {
                            self.alerts.publish(event);
                        }
                        interfaces.push(interface.into_sample(0));
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sqlite::initialize_database;
    use std::path::Path;

    #[test]
    fn status_transitions_are_saved_and_published_once() {
        let repository =
            Repository::new(initialize_database(Path::new(":memory:")).unwrap()).unwrap();
        let mut device = DeviceConfig::new("192.0.2.1", "public");
        device.name = "edge-router".into();
        device.id = Some(repository.save_device_config(&device).unwrap());
        repository
            .update_device_status(&device.ip, "online")
            .unwrap();

        let broadcaster = AlertBroadcaster::new();
        let mut receiver = broadcaster.subscribe();
        let engine = PollingEngine::with_broadcaster(AppConfig::default(), repository, broadcaster);

        engine
            .update_device_status(&device, "offline", false)
            .unwrap();
        let offline = receiver.try_recv().unwrap();
        assert_eq!(offline.kind.as_str(), "DEVICE_OFFLINE");
        assert_eq!(
            offline.severity,
            crate::alert::status::AlertSeverity::Critical
        );
        assert!(receiver.try_recv().is_err());

        engine
            .update_device_status(&device, "offline", false)
            .unwrap();
        assert!(receiver.try_recv().is_err());

        engine
            .update_device_status(&device, "online", true)
            .unwrap();
        let recovered = receiver.try_recv().unwrap();
        assert_eq!(recovered.kind.as_str(), "RESOLVED_DEVICE_OFFLINE");
        assert_eq!(
            recovered.severity,
            crate::alert::status::AlertSeverity::Info
        );
        assert!(receiver.try_recv().is_err());

        let page = engine
            .repository
            .get_alert_history_page(&crate::db::models::AlertHistoryFilter::default(), 10, 0)
            .unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.items[0].alert_type, "RESOLVED_DEVICE_OFFLINE");
        assert_eq!(page.items[0].severity, "info");
        assert_eq!(page.items[1].alert_type, "DEVICE_OFFLINE");
        assert_eq!(page.items[1].severity, "critical");
    }
}
