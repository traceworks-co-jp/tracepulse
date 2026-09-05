use crate::device::types::DeviceConfig;

#[derive(Debug, Clone)]
pub struct TuiRenderer {
    devices: Vec<DeviceConfig>,
}

impl TuiRenderer {
    pub fn new(devices: Vec<DeviceConfig>) -> Self {
        Self { devices }
    }

    pub fn render(&self) {
        println!("┌──────────────────────────────────────────────┐");
        println!("│ TracePulse - CLI / TUI Mode                 │");
        println!("├──────────────────────────────────────────────┤");
        if self.devices.is_empty() {
            println!("│ No devices registered yet                   │");
        } else {
            for device in &self.devices {
                println!(
                    "│ {:<15} {:<15} {:<8} │",
                    device.name, device.ip, device.status
                );
            }
        }
        println!("└──────────────────────────────────────────────┘");
    }
}
