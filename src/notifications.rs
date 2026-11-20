//! Edition-neutral notification provider contract.

use crate::{
    alert::{AlertBroadcaster, AlertEvent, AlertKind},
    config::AlertConfig,
    error::AppError,
    maintenance::MaintenanceStore,
    notifier::{
        AlertNotification, Notifier, NotifierConfig, outbox::Outbox,
        settings::NotifierSettingsStore,
    },
};
use rusqlite::{Connection, params};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

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

/// Attach once to Community's alert bus.
pub struct NotificationRuntime {
    pub settings: Arc<NotifierSettingsStore>,
    pub maintenance: Arc<MaintenanceStore>,
    outbox: Outbox,
    db_path: PathBuf,
    config_path: PathBuf,
}

impl NotificationRuntime {
    pub fn start(
        config_path: PathBuf,
        db_path: PathBuf,
        broadcaster: AlertBroadcaster,
        shutdown: Arc<AtomicBool>,
    ) -> Result<Arc<Self>, AppError> {
        let runtime = Arc::new(Self::open(config_path, db_path)?);
        if tokio::runtime::Handle::try_current().is_ok() {
            runtime.spawn(broadcaster, shutdown);
        } else {
            let executor = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()?;
            let service = runtime.clone();
            let receiver = broadcaster.subscribe();
            std::thread::Builder::new()
                .name("tracepulse-notifications".into())
                .spawn(move || {
                    executor.block_on(async move {
                        if let Err(error) = service.spawn_receiver(receiver, shutdown).await {
                            tracing::error!(%error, "notification runtime stopped");
                        }
                    });
                })?;
        }
        Ok(runtime)
    }

    pub fn open(config_path: PathBuf, db_path: PathBuf) -> Result<Self, AppError> {
        let config = match std::fs::metadata(&config_path) {
            Ok(_) => {
                NotifierConfig::load(&config_path).map_err(|e| AppError::Config(e.to_string()))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                NotifierConfig::from_env()
            }
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            settings: Arc::new(NotifierSettingsStore::new(config_path.clone(), config)),
            maintenance: Arc::new(MaintenanceStore::open(&db_path).map_err(AppError::Database)?),
            outbox: Outbox::open(&db_path).map_err(|e| AppError::Database(e.to_string()))?,
            db_path,
            config_path,
        })
    }

    pub fn spawn(
        &self,
        broadcaster: AlertBroadcaster,
        shutdown: Arc<AtomicBool>,
    ) -> tokio::task::JoinHandle<()> {
        self.spawn_receiver(broadcaster.subscribe(), shutdown)
    }

    fn spawn_receiver(
        &self,
        mut receiver: tokio::sync::broadcast::Receiver<AlertEvent>,
        shutdown: Arc<AtomicBool>,
    ) -> tokio::task::JoinHandle<()> {
        let settings = self.settings.handle();
        let maintenance = self.maintenance.clone();
        let outbox = self.outbox.clone();
        let db_path = self.db_path.clone();
        let config_path = self.config_path.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut current: Option<Notifier> = None;
            let mut applied = None;
            let mut last_reconcile = None;
            let mut drain = None::<tokio::task::JoinHandle<()>>;
            loop {
                let generation = settings.generation();
                if applied != Some(generation)
                    && let Some(config) = settings.current()
                {
                    if let Some(task) = drain.take() {
                        let _ = task.await;
                    }
                    applied = Some(generation);
                    match Notifier::new(config) {
                        Ok(notifier) => {
                            current = Some(
                                notifier
                                    .with_outbox(outbox.clone())
                                    .with_maintenance(maintenance.clone())
                                    .with_source("tracepulse"),
                            )
                        }
                        Err(error) => {
                            tracing::error!(%error, "notification configuration could not be applied")
                        }
                    }
                }
                tokio::select! {
                    received = receiver.recv() => match received {
                        Ok(event) if is_basic(&event.kind) => {
                            if let Some(notifier) = &current
                                && let Err(error) = notifier.enqueue(AlertNotification::from_event(event)) {
                                tracing::error!(%error, "could not queue notification");
                            }
                        }
                        Ok(_) => {}
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => tracing::warn!(skipped, "notification receiver lagged"),
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    },
                    _ = tick.tick() => {
                        if shutdown.load(Ordering::Acquire) { break; }
                        if let Some(notifier) = &current {
                            if let Err(error) = consume_port_journal(&db_path, notifier) {
                                tracing::error!(%error, "port alert journal will be retried");
                            }
                            let now = chrono::Utc::now().timestamp();
                            if last_reconcile.is_none_or(|last| now - last >= 5) {
                                last_reconcile = Some(now);
                                let thresholds = match load_release_alert_config(&config_path) {
                                    Ok(config) => Some(config),
                                    Err(error) => {
                                        tracing::error!(%error, "system metric release evaluation deferred");
                                        None
                                    }
                                };
                                if let Err(error) = maintenance.enqueue_release_summaries(notifier, now, thresholds.as_ref()) {
                                    tracing::error!(%error, "maintenance release reconciliation failed");
                                }
                            }
                            if drain.as_ref().is_none_or(|task| task.is_finished()) {
                                let mut worker = notifier.delivery_worker();
                                drain = Some(tokio::spawn(async move {
                                    if let Err(error) = worker.flush_due().await {
                                        tracing::error!(%error, "notification outbox will be retried");
                                    }
                                }));
                            }
                        }
                    }
                }
            }
            if let Some(task) = drain {
                let _ = task.await;
            }
            // Buffered batches already live in SQLite and remain available after restart.
        })
    }
}

/// Read the live [alert] settings with the same serde defaults as AppConfig, without credential migration.
pub fn load_release_alert_config(path: &std::path::Path) -> Result<AlertConfig, AppError> {
    #[derive(serde::Deserialize)]
    struct Sections {
        #[serde(default)]
        alert: AlertConfig,
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => zeroize::Zeroizing::new(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AlertConfig::default());
        }
        Err(error) => return Err(error.into()),
    };
    let config = toml::from_str::<Sections>(&text)
        .map_err(|_| AppError::Config("invalid alert configuration (contents redacted)".into()))?
        .alert;
    if !config.cpu.validate() || !config.memory.validate() {
        return Err(AppError::Config(
            "invalid CPU or memory alert thresholds".into(),
        ));
    }
    Ok(config)
}

/// Write the recoverable port notification in the same transaction as the detector state.
pub fn journal_port_alert(db: &Connection, event: &AlertEvent) -> Result<(), AppError> {
    if !matches!(
        event.kind.as_str(),
        "PORT_LINK_DOWN" | "RESOLVED_PORT_LINK_DOWN"
    ) {
        return Err(AppError::Validation("not a port link alert".into()));
    }
    let payload = serde_json::to_string(&AlertNotification::from_event(event.clone()))
        .map_err(|e| AppError::Database(e.to_string()))?;
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS notification_port_journal(
             id INTEGER PRIMARY KEY, event_id TEXT NOT NULL UNIQUE, payload TEXT NOT NULL);",
    )?;
    db.execute(
        "INSERT OR IGNORE INTO notification_port_journal(event_id,payload) VALUES(?1,?2)",
        params![event.id, payload],
    )?;
    Ok(())
}

/// Reconcile a bounded batch, including events committed before a process crash.
/// The journal is acknowledged only after the notifier has evaluated and admitted the event.
pub fn consume_port_journal(path: &std::path::Path, notifier: &Notifier) -> Result<(), AppError> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    db.busy_timeout(Duration::from_secs(3))?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='notification_port_journal')",
        [], |row| row.get(0),
    )?;
    if !exists {
        return Ok(());
    }
    let entries = {
        let mut query = db.prepare(
            "SELECT event_id,payload FROM notification_port_journal ORDER BY id LIMIT 128",
        )?;
        query
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?
    };
    for (id, payload) in entries {
        let alert =
            serde_json::from_str(&payload).map_err(|e| AppError::Database(e.to_string()))?;
        notifier
            .enqueue(alert)
            .map_err(|e| AppError::Database(e.to_string()))?;
        db.execute(
            "DELETE FROM notification_port_journal WHERE event_id=?1",
            [id],
        )?;
    }
    Ok(())
}

fn is_basic(kind: &AlertKind) -> bool {
    match kind {
        AlertKind::Custom(name) => matches!(
            name.trim_start_matches("RESOLVED_"),
            "DEVICE_OFFLINE"
                | "PORT_LINK_DOWN"
                | "CPU_HIGH"
                | "MEMORY_HIGH"
                | "BANDWIDTH_HIGH"
                | "ERROR_RATE_HIGH"
        ),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        alert::AlertSeverity, device::types::DeviceConfig, notifier::WebhookEndpointConfig,
    };

    #[test]
    fn basic_dispatch_includes_online_recovery_but_not_enterprise_findings() {
        assert!(is_basic(&AlertKind::Custom(
            "RESOLVED_DEVICE_OFFLINE".into()
        )));
        assert!(!is_basic(&AlertKind::Custom("IF_ERROR_RATE_SPIKE".into())));
        assert!(!is_basic(&AlertKind::Custom(
            "RESOLVED_IF_ERROR_RATE_SPIKE".into()
        )));
    }

    #[test]
    fn port_journal_recovers_after_commit_and_deduplicates_broadcast() {
        let dir = tempfile::tempdir_in("target").unwrap();
        let path = dir.path().join("port-journal.db");
        let mut db = Connection::open(&path).unwrap();
        let mut device = DeviceConfig::new("192.0.2.1", "");
        device.id = Some(1);
        let event = AlertEvent::new(
            AlertKind::Custom("PORT_LINK_DOWN".into()),
            AlertSeverity::Critical,
            &device,
            "link down",
        );
        let tx = db.transaction().unwrap();
        journal_port_alert(&tx, &event).unwrap();
        tx.commit().unwrap();

        let store = Arc::new(MaintenanceStore::open(&path).unwrap());
        let notifier = Notifier::new(NotifierConfig {
            webhook: WebhookEndpointConfig {
                enabled: true,
                webhook_url: Some("http://127.0.0.1:1".into()),
                webhook_url_env: None,
            },
            ..NotifierConfig::default()
        })
        .unwrap()
        .with_maintenance(store);
        assert!(consume_port_journal(&path, &notifier).is_err());
        let notifier = notifier.with_outbox(Outbox::open(&path).unwrap());
        consume_port_journal(&path, &notifier).unwrap();
        notifier
            .enqueue(AlertNotification::from_event(event))
            .unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM notification_outbox", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM notification_port_journal", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
    }
}
