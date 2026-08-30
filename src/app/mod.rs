use crate::config::AppConfig;
use crate::db::models::DeviceMetrics;
use crate::db::repository::Repository;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::monitor::{calculate_bandwidth_utilization_from_delta, PollingEngine};
use crate::ui::TuiRenderer;
use crate::web::server::WebServer;
use rusqlite::Connection;

pub struct AppRunner {
    pub config: AppConfig,
    pub connection: Connection,
}

impl AppRunner {
    pub fn new(config: AppConfig, connection: Connection) -> Self {
        Self { config, connection }
    }

    pub fn run_cli(self) -> Result<(), AppError> {
        println!("TracePulse CLI mode started");
        println!("Polling interval: {}s", self.config.polling.interval_seconds);
        println!("Default SNMP community: {}", self.config.snmp.default_community);
        println!("Database ready at: data.db");

        let repository = Repository::new(self.connection);
        let registered_devices = repository.list_devices()?;
        let renderer_devices: Vec<DeviceConfig> = registered_devices
            .iter()
            .map(|device| DeviceConfig {
                id: device.id,
                name: device.name.clone(),
                ip: device.ip.clone(),
                community: device.community.clone(),
                device_type: device.device_type.clone(),
                status: device.status.clone(),
                last_seen_at: device.last_seen_at.clone(),
            })
            .collect();

        if renderer_devices.is_empty() {
            println!("No devices registered yet.");
            println!("Use auto-discovery or add devices manually before starting monitoring.");
        } else {
            let engine = PollingEngine::new(self.config.clone(), repository);
            for device in &renderer_devices {
                match engine.poll_device(device) {
                    Ok((system, interfaces)) => {
                        if let Some(device_id) = device.id {
                            let now = chrono::Utc::now().to_rfc3339();
                            let metrics = DeviceMetrics {
                                id: None,
                                device_id,
                                cpu_usage: system.cpu_usage,
                                memory_usage: system.memory_usage,
                                sampled_at: now,
                            };
                            let _ = engine.repository.save_device_metrics(&metrics);

                            for iface in &interfaces {
                                let mut sample = iface.clone();
                                if let Ok(Some(prev)) = engine.repository.get_latest_interface_sample(device_id, sample.if_index) {
                                    sample.bandwidth_utilization = calculate_bandwidth_utilization_from_delta(
                                        prev.in_octets.saturating_add(prev.out_octets),
                                        sample.in_octets.saturating_add(sample.out_octets),
                                        self.config.polling.interval_seconds,
                                        1_000_000_000,
                                    );
                                }
                                let _ = engine.repository.save_sample(&sample);
                            }

                            let spike_threshold = self.config.alert.spike_threshold;
                            match engine.repository.check_interface_spikes(device_id, spike_threshold) {
                                Ok(spikes) if !spikes.is_empty() => {
                                    for spike in spikes {
                                        println!(
                                            "SPIKE {} ({}) {} (if-{}) total_delta={} in_errors={} out_errors={} in_discards={} out_discards={}",
                                            device.name,
                                            device.ip,
                                            spike.if_name,
                                            spike.if_index,
                                            spike.total_delta,
                                            spike.in_errors_delta,
                                            spike.out_errors_delta,
                                            spike.in_discards_delta,
                                            spike.out_discards_delta
                                        );
                                    }
                                }
                                Ok(_) => {}
                                Err(err) => println!("{} ({}) -> spike check failed: {}", device.name, device.ip, err),
                            }
                        }
                        let cpu_text = system
                            .cpu_usage
                            .map(|v| format!("{}%", v))
                            .unwrap_or_else(|| "N/A".to_string());
                        println!(
                            "{} ({}) -> cpu={}, interfaces={}",
                            device.name,
                            device.ip,
                            cpu_text,
                            interfaces.len()
                        );
                    }
                    Err(err) => {
                        println!("{} ({}) -> {}", device.name, device.ip, err);
                    }
                }
            }
        }

        let renderer = TuiRenderer::new(renderer_devices);
        renderer.render();

        Ok(())
    }

    pub fn run_web(self) -> Result<(), AppError> {
        println!("TracePulse WebGUI mode started");
        let repository = Repository::new(self.connection);
        let server = WebServer::new("127.0.0.1", 8080, self.config, repository);
        server.start()?;
        Ok(())
    }
}
