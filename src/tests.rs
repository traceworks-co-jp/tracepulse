#[cfg(test)]
mod tests {
    use crate::alert::rules::AlertRules;
    use crate::alert::scorer::HealthScorer;
    use crate::app_state::AppState;
    use crate::config::AppConfig;
    use crate::db::models::{AlertEvent, DeviceMetrics, InterfaceSample};
    use crate::db::repository::Repository;
    use crate::device::discovery::{parse_cidr, validate_community_scan_range};
    use crate::device::registry::DeviceRegistry;
    use crate::device::types::DeviceConfig;
    use crate::monitor::interface::InterfaceMonitor;
    use crate::monitor::system::SystemMonitor;
    use crate::snmp::SnmpDeviceInfo;
    use rusqlite::Connection;

    #[test]
    fn parses_valid_cidr() {
        let result = parse_cidr("192.168.1.0/24");
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_invalid_cidr() {
        let result = parse_cidr("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn community_scan_range_allows_single_subnet() {
        assert!(validate_community_scan_range("192.168.1.0/24").is_ok());
        assert!(validate_community_scan_range("192.168.1.0/28").is_ok());
        assert!(validate_community_scan_range("192.168.1.5/32").is_ok());
    }

    #[test]
    fn community_scan_range_rejects_wider_than_slash24() {
        assert!(validate_community_scan_range("192.168.0.0/23").is_err());
        assert!(validate_community_scan_range("10.0.0.0/8").is_err());
    }

    #[test]
    fn device_registry_avoids_duplicates() {
        let mut registry = DeviceRegistry::new();
        let device = DeviceConfig::new("192.168.1.10", "public");
        registry.add_device(device.clone()).unwrap();

        let duplicate = DeviceConfig::new("192.168.1.10", "public");
        let result = registry.add_device(duplicate);
        assert!(result.is_err());
    }

    #[test]
    fn alert_rules_detect_spike() {
        let rules = AlertRules::new(0.05, 10, 80, 60);
        assert!(rules.detect_spike(20, 40));
    }

    #[test]
    fn health_scorer_returns_expected_status() {
        let scorer = HealthScorer::new(80, 60);
        let score = scorer.score(4.0, 18.0, 30, 25);
        assert!(score <= 100);
    }

    #[test]
    fn system_monitor_maps_snmp_values() {
        let info = SnmpDeviceInfo {
            sys_name: "core-sw-01".to_string(),
            sys_descr: "Cisco IOS".to_string(),
            cpu_usage: Some(62),
            memory_usage: Some(48),
            memory_used_bytes: Some(512 * 1024 * 1024),
            hardware_sensors: vec![],
        };

        let monitor = SystemMonitor::from_snmp_device_info(&info);
        assert_eq!(monitor.sys_name, "core-sw-01");
        assert_eq!(monitor.cpu_usage, Some(62));
        assert_eq!(monitor.memory_usage, Some(48));
        assert_eq!(monitor.memory_used_bytes, Some(512 * 1024 * 1024));
    }

    #[test]
    fn interface_monitor_maps_link_status_and_bandwidth() {
        let monitor = InterfaceMonitor::from_snmp_snapshot(
            1,
            "GigabitEthernet1/0/1",
            1,
            4,
            2,
            8,
            9,
            0,
            500_000,
            750_000,
        );

        assert_eq!(monitor.if_name, "GigabitEthernet1/0/1");
        assert_eq!(monitor.link_status, "up");
        assert!(monitor.bandwidth_utilization >= 0.0);
        assert_eq!(monitor.in_errors, 4);
    }

    #[test]
    fn app_state_uses_single_repository_connection() {
        let config = AppConfig::default();
        let connection = Connection::open_in_memory().unwrap();
        let state = AppState::new(config.clone(), connection);

        assert_eq!(
            state.config.polling.interval_seconds,
            config.polling.interval_seconds
        );
        assert!(state.registry.list_devices().is_empty());
    }

    #[test]
    fn repository_persists_device_config_roundtrip() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS devices (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    ip TEXT NOT NULL UNIQUE,
                    community TEXT NOT NULL,
                    device_type TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'unknown',
                    last_seen_at TEXT,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                "#,
            )
            .unwrap();

        let repository = Repository::new(connection);
        let device = DeviceConfig::new("192.168.1.30", "public");

        repository.save_device_config(&device).unwrap();
        let saved = repository.list_devices().unwrap();

        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].ip, "192.168.1.30");
        assert_eq!(saved[0].community, "public");
    }

    #[test]
    fn repository_removes_device_and_related_history() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS devices (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    ip TEXT NOT NULL UNIQUE,
                    community TEXT NOT NULL,
                    device_type TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'unknown',
                    last_seen_at TEXT,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE IF NOT EXISTS interface_samples (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    device_id INTEGER NOT NULL,
                    if_index INTEGER NOT NULL,
                    if_name TEXT NOT NULL,
                    link_status TEXT NOT NULL,
                    in_errors INTEGER NOT NULL DEFAULT 0,
                    out_errors INTEGER NOT NULL DEFAULT 0,
                    in_discards INTEGER NOT NULL DEFAULT 0,
                    out_discards INTEGER NOT NULL DEFAULT 0,
                    late_collisions INTEGER NOT NULL DEFAULT 0,
                    in_octets INTEGER NOT NULL DEFAULT 0,
                    out_octets INTEGER NOT NULL DEFAULT 0,
                    bandwidth_utilization REAL NOT NULL DEFAULT 0.0,
                    sampled_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE IF NOT EXISTS alert_events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    device_id INTEGER NOT NULL,
                    alert_type TEXT NOT NULL,
                    severity TEXT NOT NULL,
                    details TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE IF NOT EXISTS device_metrics (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    device_id INTEGER NOT NULL,
                    cpu_usage INTEGER,
                    memory_usage INTEGER,
                    memory_used_bytes INTEGER,
                    sampled_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                "#,
            )
            .unwrap();

        let repository = Repository::new(connection);
        let device = DeviceConfig::new("192.168.1.31", "public");
        let device_id = repository.save_device_config(&device).unwrap();

        repository
            .save_sample(&InterfaceSample {
                id: None,
                device_id,
                if_index: 1,
                if_name: "Gi0/0".to_string(),
                link_status: "up".to_string(),
                in_errors: 0,
                out_errors: 0,
                in_discards: 0,
                out_discards: 0,
                late_collisions: 0,
                in_octets: 0,
                out_octets: 0,
                bandwidth_utilization: 0.0,
                sampled_at: "2026-08-30T00:00:00Z".to_string(),
            })
            .unwrap();
        repository
            .save_alert(&AlertEvent {
                id: None,
                device_id,
                alert_type: "test".to_string(),
                severity: "info".to_string(),
                details: "test".to_string(),
                created_at: None,
            })
            .unwrap();
        repository
            .save_device_metrics(&DeviceMetrics {
                id: None,
                device_id,
                cpu_usage: Some(10),
                memory_usage: Some(20),
                memory_used_bytes: Some(128 * 1024 * 1024),
                sampled_at: "2026-08-30T00:00:00Z".to_string(),
            })
            .unwrap();

        repository.remove_device_by_ip("192.168.1.31").unwrap();

        assert!(repository.list_devices().unwrap().is_empty());
        let remaining = repository.count_rows("interface_samples").unwrap();
        let alerts_remaining = repository.count_rows("alert_events").unwrap();
        let metrics_remaining = repository.count_rows("device_metrics").unwrap();
        assert_eq!(remaining, 0);
        assert_eq!(alerts_remaining, 0);
        assert_eq!(metrics_remaining, 0);
    }
}
