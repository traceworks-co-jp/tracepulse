use crate::alert::broadcaster::AlertBroadcaster;
use crate::alert::event::AlertEvent;
use crate::config::AppConfig;
use crate::db::repository::Repository;
use crate::error::AppError;
use crate::notifications::NotificationSettingsProvider;
use crate::ui::TuiRenderer;
use crate::web::server::WebServer;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct AppRunner {
    pub config: AppConfig,
    pub connection: Connection,
    alerts: AlertBroadcaster,
    notifications: Option<Arc<dyn NotificationSettingsProvider>>,
    flow_repository: Option<Arc<dyn crate::flow::FlowRepository>>,
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
            notifications: None,
            flow_repository: None,
        }
    }

    pub fn with_notification_provider(
        mut self,
        provider: Arc<dyn NotificationSettingsProvider>,
    ) -> Self {
        self.notifications = Some(provider);
        self
    }

    pub fn notification_provider(&self) -> Option<Arc<dyn NotificationSettingsProvider>> {
        self.notifications.clone()
    }

    pub fn with_flow_repository(
        mut self,
        repository: Arc<dyn crate::flow::FlowRepository>,
    ) -> Self {
        self.flow_repository = Some(repository);
        self
    }

    pub fn flow_repository(&self) -> Option<Arc<dyn crate::flow::FlowRepository>> {
        self.flow_repository.clone()
    }

    /// 外部モジュールがアラートイベントを購読するためのレシーバーを返す。
    pub fn subscribe_alerts(&self) -> broadcast::Receiver<AlertEvent> {
        self.alerts.subscribe()
    }

    pub fn alert_broadcaster(&self) -> AlertBroadcaster {
        self.alerts.clone()
    }

    pub fn run_cli(self) -> Result<(), AppError> {
        let renderer = TuiRenderer::new(self);
        renderer.run()
    }

    pub fn run_web(self) -> Result<(), AppError> {
        println!("TracePulse WebGUI mode started");
        if let Some(provider) = self.notifications.clone() {
            let mut receiver = self.subscribe_alerts();
            std::thread::spawn(move || {
                while let Ok(event) = receiver.blocking_recv() {
                    if let Err(error) = provider.dispatch(&event) {
                        eprintln!("notification delivery failed: {error}");
                    }
                }
            });
        }
        let repository = Repository::new(self.connection);
        let mut server = WebServer::new("127.0.0.1", 8080, self.config, repository);
        if let Some(provider) = self.notifications {
            server = server.with_notification_provider(provider);
        }
        server.start()?;
        Ok(())
    }
}
