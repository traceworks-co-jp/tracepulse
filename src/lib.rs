pub mod alert;
pub mod app;
pub mod app_state;
pub mod cleanup;
pub mod config;
pub mod db;
pub mod device;
pub mod diagnostics;
pub mod error;
pub mod flow;
pub mod monitor;
pub mod notifications;
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
    let mut config = AppConfig::load(&config_path)?;
    config.web.port = parse_web_port(&args, config.web.port)?;

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

fn parse_web_port(
    args: &[String],
    configured_port: std::num::NonZeroU16,
) -> Result<std::num::NonZeroU16, AppError> {
    let mut arguments = args.iter().skip(1);
    let mut override_port = None;
    while let Some(argument) = arguments.next() {
        let value = if argument == "--port" {
            arguments
                .next()
                .map(String::as_str)
                .ok_or_else(|| AppError::Validation("--port requires a value (1-65535)".into()))?
        } else if let Some(value) = argument.strip_prefix("--port=") {
            value
        } else {
            continue;
        };
        if override_port.is_some() {
            return Err(AppError::Validation(
                "--port can only be specified once".into(),
            ));
        }
        override_port = Some(value.parse::<std::num::NonZeroU16>().map_err(|_| {
            AppError::Validation("--port must be an integer from 1 to 65535".into())
        })?);
    }
    Ok(override_port.unwrap_or(configured_port))
}

#[cfg(test)]
mod startup_tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn web_port_uses_default_or_config_unless_overridden() {
        let defaults = AppConfig::default();
        assert_eq!(
            parse_web_port(&arguments(&["tracepulse", "--web"]), defaults.web.port)
                .unwrap()
                .get(),
            8080
        );
        let config: AppConfig = toml::from_str("[web]\nport = 9090\n").unwrap();
        assert_eq!(
            parse_web_port(&arguments(&["tracepulse"]), config.web.port)
                .unwrap()
                .get(),
            9090
        );
        for options in [
            vec!["tracepulse", "--web", "--port", "18083"],
            vec!["tracepulse", "--port=18083"],
        ] {
            assert_eq!(
                parse_web_port(&arguments(&options), config.web.port)
                    .unwrap()
                    .get(),
                18083
            );
        }
        let legacy: AppConfig = toml::from_str("[polling]\ninterval_seconds = 30\n").unwrap();
        assert_eq!(legacy.web.port.get(), 8080);
        let empty_web: AppConfig = toml::from_str("[web]\n").unwrap();
        assert_eq!(empty_web.web.port.get(), 8080);
    }

    #[test]
    fn web_port_rejects_missing_invalid_and_duplicate_arguments() {
        let port = AppConfig::default().web.port;
        for value in ["", "0", "-1", "65536", "9090.5", "abc"] {
            assert!(parse_web_port(&arguments(&["tracepulse", "--port", value]), port).is_err());
            assert!(
                parse_web_port(
                    &arguments(&["tracepulse", &format!("--port={value}")]),
                    port
                )
                .is_err()
            );
        }
        assert!(parse_web_port(&arguments(&["tracepulse", "--port"]), port).is_err());
        assert!(
            parse_web_port(
                &arguments(&["tracepulse", "--port", "9090", "--port=9091"]),
                port
            )
            .is_err()
        );
        for value in ["1", "65535"] {
            assert!(parse_web_port(&arguments(&["tracepulse", "--port", value]), port).is_ok());
        }
    }

    #[test]
    fn web_port_config_rejects_zero_and_out_of_range_values() {
        for value in ["0", "-1", "65536", "9090.5", "'abc'"] {
            assert!(toml::from_str::<AppConfig>(&format!("[web]\nport = {value}\n")).is_err());
        }
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
