use crate::alert::broadcaster::AlertBroadcaster;
use crate::alert::event::AlertEvent;
use crate::config::AppConfig;
use crate::db::repository::Repository;
use crate::error::AppError;
use crate::notifications::NotificationSettingsProvider;
use crate::ui::{TuiExtension, TuiRenderer};
use crate::web::server::WebServer;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct AppRunner {
    pub config: AppConfig,
    pub connection: Connection,
    alerts: AlertBroadcaster,
    notifications: Option<Arc<dyn NotificationSettingsProvider>>,
    flow_repository: Option<Arc<dyn crate::flow::FlowRepository>>,
    tui_extension: Option<Box<dyn TuiExtension>>,
    database_path: PathBuf,
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
            tui_extension: None,
            database_path: crate::exe_dir().join("data.db"),
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

    pub fn with_tui_extension(mut self, extension: Box<dyn TuiExtension>) -> Self {
        self.tui_extension = Some(extension);
        self
    }

    pub fn flow_repository(&self) -> Option<Arc<dyn crate::flow::FlowRepository>> {
        self.flow_repository.clone()
    }

    pub fn with_database_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.database_path = path.into();
        self
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// 外部モジュールがアラートイベントを購読するためのレシーバーを返す。
    pub fn subscribe_alerts(&self) -> broadcast::Receiver<AlertEvent> {
        self.alerts.subscribe()
    }

    pub fn alert_broadcaster(&self) -> AlertBroadcaster {
        self.alerts.clone()
    }

    pub fn run_cli(mut self) -> Result<(), AppError> {
        let extension = self.tui_extension.take();
        let renderer = TuiRenderer::new(self).with_extension(extension);
        renderer.run()
    }

    pub fn run_web(self) -> Result<(), AppError> {
        println!("TracePulse WebGUI mode started");
        let repository = Repository::new(self.connection);
        let server = WebServer::new("127.0.0.1", 8080, self.config, repository);
        server.start()?;
        Ok(())
    }
}
