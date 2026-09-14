use crate::error::AppError;
use rusqlite::Connection;
use std::path::Path;

pub fn initialize_database(path: &Path) -> Result<Connection, AppError> {
    let connection = Connection::open(path)?;

    connection.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;",
    )?;

    connection.execute_batch(
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

        CREATE UNIQUE INDEX IF NOT EXISTS idx_devices_ip ON devices(ip);

        CREATE TABLE IF NOT EXISTS interface_samples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            if_index INTEGER NOT NULL,
            if_name TEXT NOT NULL,
            link_status TEXT NOT NULL,
            in_errors INTEGER NOT NULL DEFAULT 0,
            out_errors INTEGER NOT NULL DEFAULT 0,
            in_packets INTEGER NOT NULL DEFAULT 0,
            out_packets INTEGER NOT NULL DEFAULT 0,
            in_discards INTEGER NOT NULL DEFAULT 0,
            out_discards INTEGER NOT NULL DEFAULT 0,
            late_collisions INTEGER NOT NULL DEFAULT 0,
            in_octets INTEGER NOT NULL DEFAULT 0,
            out_octets INTEGER NOT NULL DEFAULT 0,
            bandwidth_utilization REAL NOT NULL DEFAULT 0.0,
            sampled_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(device_id) REFERENCES devices(id)
        );

        CREATE TABLE IF NOT EXISTS alert_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            alert_type TEXT NOT NULL,
            severity TEXT NOT NULL,
            details TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(device_id) REFERENCES devices(id)
        );

        CREATE TABLE IF NOT EXISTS device_metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            cpu_usage INTEGER,
            memory_usage INTEGER,
            memory_used_bytes INTEGER,
            sampled_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(device_id) REFERENCES devices(id)
        );

        CREATE TABLE IF NOT EXISTS flow_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exporter_ip TEXT,
            source_ip TEXT NOT NULL,
            destination_ip TEXT NOT NULL,
            source_port INTEGER NOT NULL,
            destination_port INTEGER NOT NULL,
            protocol TEXT NOT NULL,
            bytes INTEGER NOT NULL DEFAULT 0,
            packets INTEGER NOT NULL DEFAULT 0,
            ingress_if_index INTEGER,
            egress_if_index INTEGER,
            tcp_flags INTEGER NOT NULL DEFAULT 0,
            sampling_rate INTEGER NOT NULL DEFAULT 1,
            dscp INTEGER NOT NULL DEFAULT 0,
            bgp_next_hop TEXT,
            observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS aggregated_flows_1m (
            bucket_start TEXT NOT NULL,
            protocol TEXT NOT NULL,
            bytes INTEGER NOT NULL DEFAULT 0,
            packets INTEGER NOT NULL DEFAULT 0,
            flow_count INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (bucket_start, protocol)
        );

        CREATE TABLE IF NOT EXISTS aggregated_conversations_1m (
            bucket_start TEXT NOT NULL,
            source_ip TEXT NOT NULL,
            destination_ip TEXT NOT NULL,
            source_port INTEGER NOT NULL,
            destination_port INTEGER NOT NULL,
            protocol TEXT NOT NULL,
            bytes INTEGER NOT NULL DEFAULT 0,
            packets INTEGER NOT NULL DEFAULT 0,
            tcp_flags INTEGER NOT NULL DEFAULT 0,
            ingress_if_index INTEGER,
            egress_if_index INTEGER,
            PRIMARY KEY (bucket_start, source_ip, destination_ip, source_port, destination_port, protocol)
        );

        CREATE TABLE IF NOT EXISTS aggregated_conversations_1m_v2 (
            bucket_start TEXT NOT NULL,
            exporter_ip TEXT NOT NULL,
            source_ip TEXT NOT NULL,
            destination_ip TEXT NOT NULL,
            source_port INTEGER NOT NULL,
            destination_port INTEGER NOT NULL,
            protocol TEXT NOT NULL,
            bytes INTEGER NOT NULL DEFAULT 0,
            packets INTEGER NOT NULL DEFAULT 0,
            tcp_flags INTEGER NOT NULL DEFAULT 0,
            ingress_if_index INTEGER,
            egress_if_index INTEGER,
            PRIMARY KEY (bucket_start, exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol)
        );

        CREATE INDEX IF NOT EXISTS idx_device_metrics_device_sampled
            ON device_metrics(device_id, sampled_at DESC);
        CREATE INDEX IF NOT EXISTS idx_interface_samples_device_sampled
            ON interface_samples(device_id, sampled_at DESC);
        CREATE INDEX IF NOT EXISTS idx_alert_events_device_created
            ON alert_events(device_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_flow_records_observed
            ON flow_records(observed_at DESC);
        CREATE INDEX IF NOT EXISTS idx_aggregated_flows_bucket
            ON aggregated_flows_1m(bucket_start DESC);
        CREATE INDEX IF NOT EXISTS idx_aggregated_conversations_bucket
            ON aggregated_conversations_1m(bucket_start DESC);
        CREATE INDEX IF NOT EXISTS idx_aggregated_conversations_v2_bucket
            ON aggregated_conversations_1m_v2(bucket_start DESC, exporter_ip);
        "#,
    )?;

    ensure_interface_sample_columns(&connection)?;
    ensure_device_metrics_nullable(&connection)?;
    ensure_device_metrics_columns(&connection)?;
    ensure_flow_record_columns(&connection)?;

    Ok(connection)
}

fn ensure_flow_record_columns(connection: &Connection) -> Result<(), AppError> {
    let mut stmt = connection.prepare("PRAGMA table_info(flow_records)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|row| row.ok())
        .collect();
    for (name, definition) in [
        ("ingress_if_index", "INTEGER"),
        ("exporter_ip", "TEXT"),
        ("egress_if_index", "INTEGER"),
        ("tcp_flags", "INTEGER NOT NULL DEFAULT 0"),
        ("sampling_rate", "INTEGER NOT NULL DEFAULT 1"),
        ("dscp", "INTEGER NOT NULL DEFAULT 0"),
        ("bgp_next_hop", "TEXT"),
    ] {
        if !columns.iter().any(|column| column == name) {
            connection.execute(
                &format!("ALTER TABLE flow_records ADD COLUMN {name} {definition}"),
                [],
            )?;
        }
    }
    connection.execute("CREATE INDEX IF NOT EXISTS idx_flow_records_protocol_time ON flow_records(observed_at, protocol)", [])?;
    connection.execute("CREATE INDEX IF NOT EXISTS idx_flow_records_pair_time ON flow_records(observed_at, source_ip, destination_ip)", [])?;
    Ok(())
}

fn ensure_interface_sample_columns(connection: &Connection) -> Result<(), AppError> {
    let mut stmt = connection.prepare("PRAGMA table_info(interface_samples)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.iter().any(|c| c == "in_octets") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN in_octets INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "in_packets") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN in_packets INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "out_packets") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN out_packets INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "out_octets") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN out_octets INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "late_collisions") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN late_collisions INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "fcs_errors") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN fcs_errors INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "alignment_errors") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN alignment_errors INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "frame_too_longs") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN frame_too_longs INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "internal_mac_receive_errors") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN internal_mac_receive_errors INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.iter().any(|c| c == "rx_optical_power_dbm") {
        connection.execute(
            "ALTER TABLE interface_samples ADD COLUMN rx_optical_power_dbm REAL",
            [],
        )?;
    }

    Ok(())
}

fn ensure_device_metrics_nullable(connection: &Connection) -> Result<(), AppError> {
    let mut stmt = connection.prepare("PRAGMA table_info(device_metrics)")?;
    let columns: Vec<(String, i64)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let cpu_not_null = columns
        .iter()
        .find(|(name, _)| name == "cpu_usage")
        .map(|(_, notnull)| *notnull)
        .unwrap_or(0);
    let mem_not_null = columns
        .iter()
        .find(|(name, _)| name == "memory_usage")
        .map(|(_, notnull)| *notnull)
        .unwrap_or(0);

    if cpu_not_null == 0 && mem_not_null == 0 {
        return Ok(());
    }

    connection.execute_batch(
        r#"
        BEGIN IMMEDIATE;
        ALTER TABLE device_metrics RENAME TO device_metrics_legacy;
        CREATE TABLE device_metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            cpu_usage INTEGER,
            memory_usage INTEGER,
            memory_used_bytes INTEGER,
            sampled_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(device_id) REFERENCES devices(id)
        );
        INSERT INTO device_metrics (id, device_id, cpu_usage, memory_usage, memory_used_bytes, sampled_at)
            SELECT id, device_id, cpu_usage, memory_usage, NULL, sampled_at
            FROM device_metrics_legacy;
        DROP TABLE device_metrics_legacy;
        CREATE INDEX IF NOT EXISTS idx_device_metrics_device_sampled
            ON device_metrics(device_id, sampled_at DESC);
        COMMIT;
        "#,
    )?;

    Ok(())
}

fn ensure_device_metrics_columns(connection: &Connection) -> Result<(), AppError> {
    let mut stmt = connection.prepare("PRAGMA table_info(device_metrics)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.iter().any(|c| c == "memory_used_bytes") {
        connection.execute(
            "ALTER TABLE device_metrics ADD COLUMN memory_used_bytes INTEGER",
            [],
        )?;
    }

    Ok(())
}
