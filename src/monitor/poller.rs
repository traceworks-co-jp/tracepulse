use crate::config::AppConfig;
use crate::db::models::InterfaceSample;
use crate::db::repository::Repository;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::monitor::system::SystemMonitor;
use crate::snmp::SnmpClient;

#[derive(Debug)]
pub struct PollingEngine {
    pub config: AppConfig,
    pub repository: Repository,
}

impl PollingEngine {
    pub fn new(config: AppConfig, repository: Repository) -> Self {
        Self { config, repository }
    }

    pub fn poll_device(&self, device: &DeviceConfig) -> Result<(SystemMonitor, Vec<InterfaceSample>), AppError> {
        let client = SnmpClient::with_snmp_config(device.community.clone(), self.config.snmp.clone());
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
