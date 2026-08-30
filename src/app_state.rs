use crate::config::AppConfig;
use crate::db::repository::Repository;
use crate::device::DeviceRegistry;
use rusqlite::Connection;

pub struct AppState {
    pub config: AppConfig,
    pub registry: DeviceRegistry,
    pub repository: Repository,
}

impl AppState {
    pub fn new(config: AppConfig, connection: Connection) -> Self {
        Self {
            config,
            registry: DeviceRegistry::new(),
            repository: Repository::new(connection),
        }
    }
}
