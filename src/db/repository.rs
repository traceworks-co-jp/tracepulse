use crate::db::models::{
    AlertEvent, Device, DeviceMetrics, InterfacePortDelta, InterfaceSample, InterfaceSpike,
    RecentAlert,
};
use crate::error::AppError;
use rusqlite::params;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Repository {
    connection: rusqlite::Connection,
}

impl Repository {
    pub fn new(connection: rusqlite::Connection) -> Self {
        Self { connection }
    }

    pub fn save_device(&self, device: &Device) -> Result<i64, AppError> {
        let id = self.connection.execute(
            "INSERT INTO devices (name, ip, community, device_type, status, last_seen_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(ip) DO UPDATE SET
                 name = excluded.name,
                 community = excluded.community,
                 device_type = excluded.device_type,
                 status = excluded.status,
                 last_seen_at = excluded.last_seen_at,
                 updated_at = CURRENT_TIMESTAMP",
            params![
                device.name,
                device.ip,
                device.community,
                device.device_type,
                device.status,
                device.last_seen_at
            ],
        )?;

        Ok(id as i64)
    }

    pub fn save_device_config(
        &self,
        device: &crate::device::types::DeviceConfig,
    ) -> Result<i64, AppError> {
        let maybe_existing = self.find_device_by_ip(&device.ip)?;
        let id = match maybe_existing {
            Some(existing) => {
                self.connection.execute(
                    "UPDATE devices SET name = ?1, community = ?2, device_type = ?3, status = ?4, last_seen_at = ?5, updated_at = CURRENT_TIMESTAMP WHERE id = ?6",
                    params![device.name, device.community, device.device_type, device.status, device.last_seen_at, existing.id.unwrap_or(0)],
                )?;
                existing.id.unwrap_or(0)
            }
            None => {
                self.connection.execute(
                    "INSERT INTO devices (name, ip, community, device_type, status, last_seen_at, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
                    params![device.name, device.ip, device.community, device.device_type, device.status, device.last_seen_at],
                )?;
                self.connection.query_row(
                    "SELECT id FROM devices WHERE ip = ?1",
                    params![device.ip],
                    |row| row.get::<_, i64>(0),
                )?
            }
        };

        Ok(id)
    }

    pub fn find_device_by_ip(&self, ip: &str) -> Result<Option<Device>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, name, ip, community, device_type, status, last_seen_at, created_at, updated_at FROM devices WHERE ip = ?1",
        )?;

        match stmt.query_row(params![ip], |row| {
            Ok(Device {
                id: row.get(0)?,
                name: row.get(1)?,
                ip: row.get(2)?,
                community: row.get(3)?,
                device_type: row.get(4)?,
                status: row.get(5)?,
                last_seen_at: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        }) {
            Ok(device) => Ok(Some(device)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn update_device_status(&self, ip: &str, status: &str) -> Result<(), AppError> {
        self.connection.execute(
            "UPDATE devices SET status = ?1, last_seen_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE ip = ?2",
            params![status, ip],
        )?;

        Ok(())
    }

    pub fn remove_device_by_ip(&self, ip: &str) -> Result<(), AppError> {
        let device = self
            .find_device_by_ip(ip)?
            .ok_or_else(|| AppError::Validation(format!("device {} was not found", ip)))?;
        let device_id = device
            .id
            .ok_or_else(|| AppError::Validation(format!("device {} has no id", ip)))?;

        self.connection.execute(
            "DELETE FROM device_metrics WHERE device_id = ?1",
            params![device_id],
        )?;
        self.connection.execute(
            "DELETE FROM interface_samples WHERE device_id = ?1",
            params![device_id],
        )?;
        self.connection.execute(
            "DELETE FROM alert_events WHERE device_id = ?1",
            params![device_id],
        )?;
        self.connection
            .execute("DELETE FROM devices WHERE id = ?1", params![device_id])?;

        Ok(())
    }

    pub fn list_devices(&self) -> Result<Vec<Device>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, name, ip, community, device_type, status, last_seen_at, created_at, updated_at FROM devices",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Device {
                id: row.get(0)?,
                name: row.get(1)?,
                ip: row.get(2)?,
                community: row.get(3)?,
                device_type: row.get(4)?,
                status: row.get(5)?,
                last_seen_at: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        let mut devices = Vec::new();
        for item in rows {
            devices.push(item?);
        }

        Ok(devices)
    }

    pub fn count_rows(&self, table: &str) -> Result<i64, AppError> {
        let table = match table {
            "devices" | "interface_samples" | "alert_events" | "device_metrics" => table,
            _ => return Err(AppError::Validation(format!("unsupported table {}", table))),
        };

        let sql = format!("SELECT COUNT(*) FROM {table}");
        let count = self
            .connection
            .query_row(&sql, [], |row| row.get::<_, i64>(0))?;
        Ok(count)
    }

    pub fn save_sample(&self, sample: &InterfaceSample) -> Result<i64, AppError> {
        let id = self.connection.execute(
            "INSERT INTO interface_samples (device_id, if_index, if_name, link_status, in_errors, out_errors, in_discards, out_discards, late_collisions, in_octets, out_octets, bandwidth_utilization, sampled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                sample.device_id,
                sample.if_index,
                sample.if_name,
                sample.link_status,
                sample.in_errors,
                sample.out_errors,
                sample.in_discards,
                sample.out_discards,
                sample.late_collisions,
                sample.in_octets,
                sample.out_octets,
                sample.bandwidth_utilization,
                sample.sampled_at,
            ],
        )?;

        Ok(id as i64)
    }

    pub fn save_alert(&self, alert: &AlertEvent) -> Result<i64, AppError> {
        let id = self.connection.execute(
            "INSERT INTO alert_events (device_id, alert_type, severity, details, created_at)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
            params![
                alert.device_id,
                alert.alert_type,
                alert.severity,
                alert.details
            ],
        )?;

        Ok(id as i64)
    }

    pub fn cleanup_old_history(&self, history_days: u32) -> Result<usize, AppError> {
        let deleted = self.connection.execute(
            "DELETE FROM alert_events WHERE created_at < datetime('now', '-' || ?1 || ' days')",
            params![history_days.to_string()],
        )?;

        let _ = self.connection.execute(
            "DELETE FROM interface_samples WHERE sampled_at < datetime('now', '-' || ?1 || ' days')",
            params![history_days.to_string()],
        )?;

        Ok(deleted)
    }

    /// デバイスの各インターフェースについて、直近2サンプルの差分からスパイクを抽出する。
    pub fn check_interface_spikes(
        &self,
        device_id: i64,
        spike_threshold: u64,
    ) -> Result<Vec<InterfaceSpike>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT if_index, if_name, link_status,
                    in_errors, out_errors, in_discards, out_discards, late_collisions,
                    sampled_at
             FROM interface_samples
             WHERE device_id = ?1
             ORDER BY if_index ASC, sampled_at DESC, id DESC",
        )?;

        let mut by_if: std::collections::HashMap<i32, Vec<InterfaceSample>> =
            std::collections::HashMap::new();
        let rows = stmt.query_map(params![device_id], |row| {
            Ok(InterfaceSample {
                id: None,
                device_id,
                if_index: row.get(0)?,
                if_name: row.get(1)?,
                link_status: row.get(2)?,
                in_errors: row.get::<_, i64>(3)? as u64,
                out_errors: row.get::<_, i64>(4)? as u64,
                in_discards: row.get::<_, i64>(5)? as u64,
                out_discards: row.get::<_, i64>(6)? as u64,
                late_collisions: row.get::<_, i64>(7)? as u64,
                in_octets: 0,
                out_octets: 0,
                bandwidth_utilization: 0.0,
                sampled_at: row.get(8)?,
            })
        })?;

        for item in rows.filter_map(|r| r.ok()) {
            let entry = by_if.entry(item.if_index).or_default();
            if entry.len() < 2 {
                entry.push(item);
            }
        }

        let mut spikes = Vec::new();
        for (_if_index, samples) in by_if {
            if samples.len() < 2 {
                continue;
            }
            let latest = &samples[0];
            let previous = &samples[1];
            let in_errors_delta = counter32_delta(previous.in_errors, latest.in_errors);
            let out_errors_delta = counter32_delta(previous.out_errors, latest.out_errors);
            let in_discards_delta = counter32_delta(previous.in_discards, latest.in_discards);
            let out_discards_delta = counter32_delta(previous.out_discards, latest.out_discards);
            let late_collisions_delta =
                counter32_delta(previous.late_collisions, latest.late_collisions);
            let total_delta =
                in_errors_delta + out_errors_delta + in_discards_delta + out_discards_delta;
            if total_delta < spike_threshold {
                continue;
            }

            spikes.push(InterfaceSpike {
                if_index: latest.if_index,
                if_name: latest.if_name.clone(),
                link_status: latest.link_status.clone(),
                in_errors_delta,
                out_errors_delta,
                in_discards_delta,
                out_discards_delta,
                late_collisions_delta,
                total_delta,
                latest_sampled_at: latest.sampled_at.clone(),
                previous_sampled_at: previous.sampled_at.clone(),
            });
        }

        spikes.sort_by(|a, b| {
            b.total_delta
                .cmp(&a.total_delta)
                .then(a.if_index.cmp(&b.if_index))
        });
        Ok(spikes)
    }

    /// 直近サンプル時刻を返す（ポーリング重複防止用）
    pub fn latest_sample_time(&self, device_id: i64) -> Result<Option<String>, AppError> {
        match self.connection.query_row(
            "SELECT sampled_at FROM interface_samples WHERE device_id = ?1 ORDER BY sampled_at DESC LIMIT 1",
            params![device_id],
            |row| row.get::<_, String>(0),
        ) {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ─── Device Metrics ───────────────────────────────────────────────────

    pub fn save_device_metrics(&self, metrics: &DeviceMetrics) -> Result<(), AppError> {
        self.connection.execute(
            "INSERT INTO device_metrics (device_id, cpu_usage, memory_usage, memory_used_bytes, sampled_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                metrics.device_id,
                metrics.cpu_usage.map(|v| v as i64),
                metrics.memory_usage.map(|v| v as i64),
                metrics.memory_used_bytes.map(|v| v as i64),
                metrics.sampled_at
            ],
        )?;
        Ok(())
    }

    /// 直近 `limit` 件の CPU/メモリ履歴を時系列昇順で返す
    pub fn get_device_metrics_history(
        &self,
        device_id: i64,
        limit: usize,
    ) -> Result<Vec<DeviceMetrics>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, device_id, cpu_usage, memory_usage, memory_used_bytes, sampled_at
             FROM device_metrics
             WHERE device_id = ?1
             ORDER BY sampled_at DESC
             LIMIT ?2",
        )?;
        let rows: Vec<DeviceMetrics> = stmt
            .query_map(params![device_id, limit as i64], |row| {
                Ok(DeviceMetrics {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    cpu_usage: row.get::<_, Option<i64>>(2)?.map(|v| v as u32),
                    memory_usage: row.get::<_, Option<i64>>(3)?.map(|v| v as u32),
                    memory_used_bytes: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                    sampled_at: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        // DESC で取得 → 昇順に反転して返す
        let mut asc = rows;
        asc.reverse();
        Ok(asc)
    }

    // ─── Interface History ────────────────────────────────────────────────

    /// インターフェース別の直近サンプル履歴（各 if_index ごとに最新 limit 件）
    pub fn get_interface_history(
        &self,
        device_id: i64,
        limit: usize,
    ) -> Result<Vec<InterfaceSample>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, device_id, if_index, if_name, link_status,
                    in_errors, out_errors, in_discards, out_discards, late_collisions,
                    in_octets, out_octets,
                    bandwidth_utilization, sampled_at
             FROM interface_samples
             WHERE device_id = ?1
             ORDER BY sampled_at DESC
             LIMIT ?2",
        )?;
        let mut rows: Vec<InterfaceSample> = stmt
            .query_map(params![device_id, limit as i64], |row| {
                Ok(InterfaceSample {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    if_index: row.get(2)?,
                    if_name: row.get(3)?,
                    link_status: row.get(4)?,
                    in_errors: row.get::<_, i64>(5)? as u64,
                    out_errors: row.get::<_, i64>(6)? as u64,
                    in_discards: row.get::<_, i64>(7)? as u64,
                    out_discards: row.get::<_, i64>(8)? as u64,
                    late_collisions: row.get::<_, i64>(9)? as u64,
                    in_octets: row.get::<_, i64>(10)? as u64,
                    out_octets: row.get::<_, i64>(11)? as u64,
                    bandwidth_utilization: row.get(12)?,
                    sampled_at: row.get(13)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        rows.reverse();
        Ok(rows)
    }

    /// 最新インターフェーススナップショット（if_index ごとに最新1件）
    pub fn get_latest_interfaces(&self, device_id: i64) -> Result<Vec<InterfaceSample>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, device_id, if_index, if_name, link_status,
                    in_errors, out_errors, in_discards, out_discards, late_collisions,
                    in_octets, out_octets,
                    bandwidth_utilization, sampled_at
             FROM interface_samples
             WHERE (device_id, if_index, sampled_at) IN (
                 SELECT device_id, if_index, MAX(sampled_at)
                 FROM interface_samples
                 WHERE device_id = ?1
                 GROUP BY if_index
             )
             ORDER BY if_index",
        )?;
        let rows: Vec<InterfaceSample> = stmt
            .query_map(params![device_id], |row| {
                Ok(InterfaceSample {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    if_index: row.get(2)?,
                    if_name: row.get(3)?,
                    link_status: row.get(4)?,
                    in_errors: row.get::<_, i64>(5)? as u64,
                    out_errors: row.get::<_, i64>(6)? as u64,
                    in_discards: row.get::<_, i64>(7)? as u64,
                    out_discards: row.get::<_, i64>(8)? as u64,
                    late_collisions: row.get::<_, i64>(9)? as u64,
                    in_octets: row.get::<_, i64>(10)? as u64,
                    out_octets: row.get::<_, i64>(11)? as u64,
                    bandwidth_utilization: row.get(12)?,
                    sampled_at: row.get(13)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn get_latest_interface_sample(
        &self,
        device_id: i64,
        if_index: i32,
    ) -> Result<Option<InterfaceSample>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, device_id, if_index, if_name, link_status,
                    in_errors, out_errors, in_discards, out_discards, late_collisions,
                    in_octets, out_octets, bandwidth_utilization, sampled_at
             FROM interface_samples
             WHERE device_id = ?1 AND if_index = ?2
             ORDER BY sampled_at DESC, id DESC
             LIMIT 1",
        )?;

        match stmt.query_row(params![device_id, if_index], |row| {
            Ok(InterfaceSample {
                id: row.get(0)?,
                device_id: row.get(1)?,
                if_index: row.get(2)?,
                if_name: row.get(3)?,
                link_status: row.get(4)?,
                in_errors: row.get::<_, i64>(5)? as u64,
                out_errors: row.get::<_, i64>(6)? as u64,
                in_discards: row.get::<_, i64>(7)? as u64,
                out_discards: row.get::<_, i64>(8)? as u64,
                late_collisions: row.get::<_, i64>(9)? as u64,
                in_octets: row.get::<_, i64>(10)? as u64,
                out_octets: row.get::<_, i64>(11)? as u64,
                bandwidth_utilization: row.get(12)?,
                sampled_at: row.get(13)?,
            })
        }) {
            Ok(sample) => Ok(Some(sample)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    /// 指定インターフェースの直近サンプルを新しい順で返す。
    pub fn get_recent_interface_samples(
        &self,
        device_id: i64,
        if_index: i32,
        limit: usize,
    ) -> Result<Vec<InterfaceSample>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, device_id, if_index, if_name, link_status,
                    in_errors, out_errors, in_discards, out_discards, late_collisions,
                    in_octets, out_octets, bandwidth_utilization, sampled_at
             FROM interface_samples
             WHERE device_id = ?1 AND if_index = ?2
             ORDER BY sampled_at DESC, id DESC
             LIMIT ?3",
        )?;

        let rows: Vec<InterfaceSample> = stmt
            .query_map(params![device_id, if_index, limit as i64], |row| {
                Ok(InterfaceSample {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    if_index: row.get(2)?,
                    if_name: row.get(3)?,
                    link_status: row.get(4)?,
                    in_errors: row.get::<_, i64>(5)? as u64,
                    out_errors: row.get::<_, i64>(6)? as u64,
                    in_discards: row.get::<_, i64>(7)? as u64,
                    out_discards: row.get::<_, i64>(8)? as u64,
                    late_collisions: row.get::<_, i64>(9)? as u64,
                    in_octets: row.get::<_, i64>(10)? as u64,
                    out_octets: row.get::<_, i64>(11)? as u64,
                    bandwidth_utilization: row.get(12)?,
                    sampled_at: row.get(13)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    // ─── Alert History ────────────────────────────────────────────────────

    pub fn get_alert_history(
        &self,
        device_id: i64,
        limit: usize,
    ) -> Result<Vec<AlertEvent>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, device_id, alert_type, severity, details, created_at
             FROM alert_events
             WHERE device_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let rows: Vec<AlertEvent> = stmt
            .query_map(params![device_id, limit as i64], |row| {
                Ok(AlertEvent {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    alert_type: row.get(2)?,
                    severity: row.get(3)?,
                    details: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn get_recent_alerts(&self, limit: usize) -> Result<Vec<RecentAlert>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT a.id, a.device_id, d.name, d.ip, a.alert_type, a.severity, a.details, a.created_at
             FROM alert_events a
             JOIN devices d ON d.id = a.device_id
             ORDER BY a.created_at DESC, a.id DESC
             LIMIT ?1",
        )?;

        let rows: Vec<RecentAlert> = stmt
            .query_map(params![limit as i64], |row| {
                Ok(RecentAlert {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    device_name: row.get(2)?,
                    device_ip: row.get(3)?,
                    alert_type: row.get(4)?,
                    severity: row.get(5)?,
                    details: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// 保存済みの interface_error_spike アラートから直近10分のスパイク一覧を返す。
    pub fn get_recent_interface_spikes(
        &self,
        device_id: i64,
        limit: usize,
    ) -> Result<Vec<InterfaceSpike>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, device_id, alert_type, severity, details, created_at
             FROM alert_events
             WHERE device_id = ?1
               AND alert_type = 'interface_error_spike'
               AND created_at >= datetime('now', '-10 minutes')
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let alerts: Vec<AlertEvent> = stmt
            .query_map(params![device_id, limit as i64], |row| {
                Ok(AlertEvent {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    alert_type: row.get(2)?,
                    severity: row.get(3)?,
                    details: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        let mut spikes = Vec::new();

        for alert in alerts {
            let Some((
                if_name,
                if_index,
                total_delta,
                in_errors_delta,
                out_errors_delta,
                in_discards_delta,
                out_discards_delta,
            )) = parse_interface_spike_details(&alert.details)
            else {
                continue;
            };

            let link_status = self
                .get_latest_interface_sample(device_id, if_index)
                .ok()
                .flatten()
                .map(|s| s.link_status)
                .unwrap_or_else(|| "unknown".to_string());

            spikes.push(InterfaceSpike {
                if_index,
                if_name,
                link_status,
                in_errors_delta,
                out_errors_delta,
                in_discards_delta,
                out_discards_delta,
                late_collisions_delta: 0,
                total_delta,
                latest_sampled_at: alert.created_at.unwrap_or_default(),
                previous_sampled_at: String::new(),
            });
        }

        spikes.sort_by(|a, b| {
            b.latest_sampled_at
                .cmp(&a.latest_sampled_at)
                .then(b.total_delta.cmp(&a.total_delta))
        });
        Ok(spikes)
    }

    /// 各インターフェースの直近2サンプルからカウンタ差分を算出する（3パターン障害検知の入力値）。
    pub fn get_interface_port_deltas(
        &self,
        device_id: i64,
    ) -> Result<HashMap<i32, InterfacePortDelta>, AppError> {
        let mut stmt = self.connection.prepare(
            "SELECT if_index, in_errors, out_errors, in_discards, out_discards, late_collisions
             FROM interface_samples
             WHERE device_id = ?1
             ORDER BY if_index ASC, sampled_at DESC, id DESC",
        )?;

        struct Counters {
            in_errors: u64,
            out_errors: u64,
            in_discards: u64,
            out_discards: u64,
            late_collisions: u64,
        }

        let mut by_if: HashMap<i32, Vec<Counters>> = HashMap::new();
        let rows = stmt.query_map(params![device_id], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                Counters {
                    in_errors: row.get::<_, i64>(1)? as u64,
                    out_errors: row.get::<_, i64>(2)? as u64,
                    in_discards: row.get::<_, i64>(3)? as u64,
                    out_discards: row.get::<_, i64>(4)? as u64,
                    late_collisions: row.get::<_, i64>(5)? as u64,
                },
            ))
        })?;

        for (if_index, counters) in rows.filter_map(|r| r.ok()) {
            let entry = by_if.entry(if_index).or_default();
            if entry.len() < 2 {
                entry.push(counters);
            }
        }

        let mut deltas = HashMap::new();
        for (if_index, samples) in by_if {
            let delta = if samples.len() < 2 {
                InterfacePortDelta::default()
            } else {
                let latest = &samples[0];
                let previous = &samples[1];
                InterfacePortDelta {
                    in_errors_delta: counter32_delta(previous.in_errors, latest.in_errors),
                    out_errors_delta: counter32_delta(previous.out_errors, latest.out_errors),
                    in_discards_delta: counter32_delta(previous.in_discards, latest.in_discards),
                    out_discards_delta: counter32_delta(
                        previous.out_discards,
                        latest.out_discards,
                    ),
                    late_collisions_delta: counter32_delta(
                        previous.late_collisions,
                        latest.late_collisions,
                    ),
                }
            };
            deltas.insert(if_index, delta);
        }

        Ok(deltas)
    }
}

/// SNMP Counter32 は 2^32 を超えるとラップアラウンドするため、減少時はラップを考慮して差分を算出する。
pub(crate) fn counter32_delta(previous: u64, current: u64) -> u64 {
    if current >= previous {
        current - previous
    } else {
        (u32::MAX as u64 + 1 - previous) + current
    }
}

fn parse_interface_spike_details(details: &str) -> Option<(String, i32, u64, u64, u64, u64, u64)> {
    let rest = details.strip_prefix("Interface ")?;
    let (if_name, rest) = rest.split_once(" (if-")?;
    let (if_index_str, rest) = rest.split_once(") spiked by ")?;
    let (total_delta_str, rest) = rest.split_once(": in_errors +")?;
    let (in_errors_str, rest) = rest.split_once(", out_errors +")?;
    let (out_errors_str, rest) = rest.split_once(", in_discards +")?;
    let (in_discards_str, rest) = rest.split_once(", out_discards +")?;
    let (out_discards_str, _) = rest.split_once(" (threshold: ")?;

    Some((
        if_name.to_string(),
        if_index_str.parse().ok()?,
        total_delta_str.parse().ok()?,
        in_errors_str.parse().ok()?,
        out_errors_str.parse().ok()?,
        in_discards_str.parse().ok()?,
        out_discards_str.parse().ok()?,
    ))
}
