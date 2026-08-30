use crate::device::types::DeviceConfig;
use crate::error::AppError;

#[derive(Debug, Clone, Default)]
pub struct DeviceRegistry {
    devices: Vec<DeviceConfig>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_device(&mut self, device: DeviceConfig) -> Result<(), AppError> {
        if device.ip.trim().is_empty() {
            return Err(AppError::Validation("device IP cannot be empty".to_string()));
        }

        if self.devices.iter().any(|entry| entry.ip == device.ip) {
            return Err(AppError::Validation(format!("device {} already registered", device.ip)));
        }

        self.devices.push(device);
        Ok(())
    }

    pub fn update_device(&mut self, ip: &str, status: &str) -> Result<(), AppError> {
        let Some(device) = self.devices.iter_mut().find(|entry| entry.ip == ip) else {
            return Err(AppError::Validation(format!("device {} is not registered", ip)));
        };

        device.status = status.to_string();
        device.last_seen_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(())
    }

    pub fn list_devices(&self) -> &[DeviceConfig] {
        &self.devices
    }

    pub fn remove_device(&mut self, ip: &str) -> Result<(), AppError> {
        let index = self
            .devices
            .iter()
            .position(|entry| entry.ip == ip)
            .ok_or_else(|| AppError::Validation(format!("device {} was not found", ip)))?;

        self.devices.remove(index);
        Ok(())
    }
}
