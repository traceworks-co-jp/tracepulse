pub mod alert;
pub mod app;
pub mod app_state;
pub mod cleanup;
pub mod config;
pub mod db;
pub mod device;
pub mod error;
pub mod flow;
pub mod monitor;
pub mod snmp;
pub mod tests;
pub mod ui;
pub mod web;

use crate::app::AppRunner;
use crate::config::{AppConfig, AppMode};
use crate::error::AppError;
use std::path::PathBuf;

pub fn run() -> Result<(), AppError> {
    let args: Vec<String> = std::env::args().collect();
    let mode = parse_mode(&args);

    // 実行ファイルと同じディレクトリに config/DB を配置する
    let config_path = config_path();
    let config = AppConfig::load(&config_path)?;

    let db_path = exe_dir().join("data.db");
    let conn = crate::db::sqlite::initialize_database(&db_path)?;

    let runner = AppRunner::new(config, conn);
    match mode {
        AppMode::Cli => runner.run_cli(),
        AppMode::Web => runner.run_web(),
    }
}

fn parse_mode(args: &[String]) -> AppMode {
    if args.iter().any(|arg| arg == "--cli") {
        AppMode::Cli
    } else {
        AppMode::Web
    }
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    exe_dir().join("config.toml")
}
