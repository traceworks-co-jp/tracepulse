use crate::alert::broadcaster::AlertBroadcaster;
use crate::alert::event::AlertEvent;
use crate::config::AppConfig;
use crate::db::models::DeviceMetrics;
use crate::db::repository::Repository;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::monitor::{PollingEngine, calculate_bandwidth_utilization_from_delta};
use crate::ui::TuiRenderer;
use crate::web::server::WebServer;
use rusqlite::Connection;
use tokio::sync::broadcast;

pub struct AppRunner {
    pub config: AppConfig,
    pub connection: Connection,
    alerts: AlertBroadcaster,
}

impl AppRunner {
    pub fn new(config: AppConfig, connection: Connection) -> Self {
        Self::with_broadcaster(config, connection, AlertBroadcaster::new())
    }

    pub fn with_broadcaster(
        config: AppConfig,
        connection: Connection,
        alerts: AlertBroadcaster,
    ) -> Self {
        Self {
            config,
            connection,
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

    pub fn run_cli(self) -> Result<(), AppError> {
        println!("TracePulse CLI mode started");
        println!(
            "Polling interval: {}s",
            self.config.polling.interval_seconds
        );
        println!(
            "Default SNMP community: {}",
            self.config.snmp.default_community
        );
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
            let engine = PollingEngine::with_broadcaster(
                self.config.clone(),
                repository,
                self.alerts.clone(),
            );
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
                                memory_used_bytes: system.memory_used_bytes,
                                sampled_at: now,
                            };
                            let _ = engine.repository.save_device_metrics(&metrics);

                            for iface in &interfaces {
                                let mut sample = iface.clone();
                                if let Ok(Some(prev)) = engine
                                    .repository
                                    .get_latest_interface_sample(device_id, sample.if_index)
                                {
                                    sample.bandwidth_utilization =
                                        calculate_bandwidth_utilization_from_delta(
                                            prev.in_octets.saturating_add(prev.out_octets),
                                            sample.in_octets.saturating_add(sample.out_octets),
                                            self.config.polling.interval_seconds,
                                            1_000_000_000,
                                        );
                                }
                                let _ = engine.repository.save_sample(&sample);
                            }

                            let spike_threshold = self.config.alert.spike_threshold;
                            match engine
                                .repository
                                .check_interface_spikes(device_id, spike_threshold)
                            {
                                Ok(spikes) if !spikes.is_empty() => {
                                    for spike in spikes {
                                        let event = AlertEvent::from_interface_spike(
                                            device,
                                            &spike,
                                            spike_threshold,
                                        );
                                        println!("{}", event.message);
                                        engine.publish_alert(event);
                                    }
                                }
                                Ok(_) => {}
                                Err(err) => println!(
                                    "{} ({}) -> spike check failed: {}",
                                    device.name, device.ip, err
                                ),
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
                        let event = AlertEvent::device_offline(device, &err);
                        println!("{}", event.message);
                        engine.publish_alert(event);
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
