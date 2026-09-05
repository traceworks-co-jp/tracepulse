use crate::error::AppError;
use rusqlite::Connection;
use std::path::Path;

pub fn initialize_database(path: &Path) -> Result<Connection, AppError> {
    let connection = Connection::open(path)?;

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

        CREATE INDEX IF NOT EXISTS idx_device_metrics_device_sampled
            ON device_metrics(device_id, sampled_at DESC);
        CREATE INDEX IF NOT EXISTS idx_interface_samples_device_sampled
            ON interface_samples(device_id, sampled_at DESC);
        CREATE INDEX IF NOT EXISTS idx_alert_events_device_created
            ON alert_events(device_id, created_at DESC);
        "#,
    )?;

    ensure_interface_sample_columns(&connection)?;
    ensure_device_metrics_nullable(&connection)?;
    ensure_device_metrics_columns(&connection)?;

    Ok(connection)
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
