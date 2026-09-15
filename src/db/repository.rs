use crate::db::models::{
    AlertEvent, Device, DeviceMetrics, FlowApplicationShare, FlowEndpointShare, FlowRecord,
    FlowSummary, FlowTimeseries, InterfacePortDelta, InterfaceSample, InterfaceSpike,
    ProtocolShare, RecentAlert, TopTalker,
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
            "INSERT INTO interface_samples (device_id, if_index, if_name, link_status, in_errors, out_errors, in_packets, out_packets, in_discards, out_discards, late_collisions, fcs_errors, alignment_errors, frame_too_longs, internal_mac_receive_errors, rx_optical_power_dbm, in_octets, out_octets, bandwidth_utilization, sampled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                sample.device_id,
                sample.if_index,
                sample.if_name,
                sample.link_status,
                sample.in_errors,
                sample.out_errors,
                sample.in_packets,
                sample.out_packets,
                sample.in_discards,
                sample.out_discards,
                sample.late_collisions,
                sample.fcs_errors,
                sample.alignment_errors,
                sample.frame_too_longs,
                sample.internal_mac_receive_errors,
                sample.rx_optical_power_dbm,
                sample.in_octets,
                sample.out_octets,
                sample.bandwidth_utilization,
                sample.sampled_at,
            ],
        )?;

        Ok(id as i64)
    }

    pub fn save_flow_record(&self, flow: &FlowRecord) -> Result<i64, AppError> {
        let id = self.connection.execute(
            "INSERT INTO flow_records (exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol, bytes, packets, ingress_if_index, egress_if_index, tcp_flags, sampling_rate, dscp, bgp_next_hop, observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                flow.exporter_ip,
                flow.source_ip,
                flow.destination_ip,
                flow.source_port,
                flow.destination_port,
                flow.protocol,
                flow.bytes,
                flow.packets,
                flow.ingress_if_index,
                flow.egress_if_index,
                flow.tcp_flags,
                flow.sampling_rate,
                flow.dscp,
                flow.bgp_next_hop.map(|value| value.to_string()),
                flow.observed_at,
            ],
        )?;
        let bucket = flow
            .observed_at
            .get(..16)
            .map(|value| format!("{value}:00Z"))
            .unwrap_or_else(|| flow.observed_at.clone());
        self.connection.execute(
            "INSERT INTO aggregated_conversations_1m_v2 (bucket_start, exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol, bytes, packets, tcp_flags, ingress_if_index, egress_if_index)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(bucket_start, exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol) DO UPDATE SET
                 bytes = bytes + excluded.bytes,
                 packets = packets + excluded.packets,
                 tcp_flags = tcp_flags | excluded.tcp_flags,
                 ingress_if_index = COALESCE(excluded.ingress_if_index, ingress_if_index),
                 egress_if_index = COALESCE(excluded.egress_if_index, egress_if_index)",
            params![bucket, flow.exporter_ip, flow.source_ip, flow.destination_ip, flow.source_port, flow.destination_port, flow.protocol, flow.bytes, flow.packets, flow.tcp_flags, flow.ingress_if_index, flow.egress_if_index],
        )?;
        Ok(id as i64)
    }

    pub fn aggregate_flow_records_1m(&self, records: &[FlowRecord]) -> Result<(), AppError> {
        let mut buckets: HashMap<(String, String), (u64, u64, u64)> = HashMap::new();
        for flow in records {
            let bucket = flow
                .observed_at
                .get(..16)
                .map(|value| format!("{value}:00Z"))
                .unwrap_or_else(|| flow.observed_at.clone());
            let entry = buckets.entry((bucket, flow.protocol.clone())).or_default();
            entry.0 = entry.0.saturating_add(flow.bytes);
            entry.1 = entry.1.saturating_add(flow.packets);
            entry.2 = entry.2.saturating_add(1);
        }

        for ((bucket, protocol), (bytes, packets, flow_count)) in buckets {
            self.connection.execute(
                "INSERT INTO aggregated_flows_1m (bucket_start, protocol, bytes, packets, flow_count)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(bucket_start, protocol) DO UPDATE SET
                     bytes = bytes + excluded.bytes,
                     packets = packets + excluded.packets,
                     flow_count = flow_count + excluded.flow_count",
                params![bucket, protocol, bytes, packets, flow_count],
            )?;
        }
        Ok(())
    }

    pub fn protocol_shares(&self, limit_seconds: i64) -> Result<Vec<ProtocolShare>, AppError> {
        if limit_seconds > 300 {
            let mut stmt = self.connection.prepare(
                "SELECT protocol, SUM(bytes), SUM(packets), SUM(bytes) * 8.0 / MAX(1, ?1), SUM(packets) * 1.0 / MAX(1, ?1), SUM(bytes) * 100.0 / MAX(1, (SELECT SUM(bytes) FROM aggregated_flows_1m WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds'))) FROM aggregated_flows_1m WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds') GROUP BY protocol ORDER BY SUM(bytes) DESC",
            )?;
            let rows = stmt
                .query_map(params![limit_seconds], |row| {
                    Ok(ProtocolShare {
                        protocol: row.get(0)?,
                        bytes: row.get::<_, i64>(1)?.max(0) as u64,
                        bps: row.get(3)?,
                        pps: row.get(4)?,
                        percentage: row.get(5)?,
                    })
                })
                .map_err(AppError::from)?
                .filter_map(|row| row.ok())
                .collect();
            return Ok(rows);
        }
        let mut stmt = self.connection.prepare(
                "SELECT protocol, SUM(bytes), SUM(packets), SUM(bytes) * 8.0 / MAX(1, ?1),
                    SUM(packets) * 1.0 / MAX(1, ?1),
                    SUM(bytes) * 100.0 / MAX(1, (SELECT SUM(bytes) FROM flow_records WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')))
             FROM flow_records
             WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')
             GROUP BY protocol ORDER BY SUM(bytes) DESC",
        )?;
        let rows = stmt
            .query_map(params![limit_seconds], |row| {
                Ok(ProtocolShare {
                    protocol: row.get(0)?,
                    bytes: row.get::<_, i64>(1)? as u64,
                    bps: row.get(3)?,
                    pps: row.get(4)?,
                    percentage: row.get(5)?,
                })
            })?
            .filter_map(|row| row.ok())
            .collect();
        Ok(rows)
    }

    pub fn protocol_shares_for_context(
        &self,
        limit_seconds: i64,
        exporter_ip: Option<&str>,
        if_index: Option<i32>,
    ) -> Result<Vec<ProtocolShare>, AppError> {
        if limit_seconds > 300 {
            let mut stmt = self.connection.prepare(
                "SELECT protocol, SUM(bytes), SUM(packets), SUM(bytes) * 8.0 / MAX(1, ?1),
                        SUM(packets) * 1.0 / MAX(1, ?1),
                        SUM(bytes) * 100.0 / MAX(1, (SELECT SUM(bytes) FROM aggregated_conversations_1m_v2
                            WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')
                              AND (?2 IS NULL OR exporter_ip = ?2)
                              AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3)))
                 FROM aggregated_conversations_1m_v2
                 WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')
                   AND (?2 IS NULL OR exporter_ip = ?2)
                   AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3)
                 GROUP BY protocol ORDER BY SUM(bytes) DESC",
            )?;
            let rows = stmt
                .query_map(params![limit_seconds, exporter_ip, if_index], |row| {
                    Ok(ProtocolShare {
                        protocol: row.get(0)?,
                        bytes: row.get::<_, i64>(1)?.max(0) as u64,
                        bps: row.get(3)?,
                        pps: row.get(4)?,
                        percentage: row.get(5)?,
                    })
                })?
                .filter_map(|row| row.ok())
                .collect();
            return Ok(rows);
        }
        let mut stmt = self.connection.prepare(
            "SELECT protocol, SUM(bytes), SUM(packets), SUM(bytes) * 8.0 / MAX(1, ?1),
                    SUM(packets) * 1.0 / MAX(1, ?1),
                    SUM(bytes) * 100.0 / MAX(1, (SELECT SUM(bytes) FROM flow_records
                        WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')
                          AND (?2 IS NULL OR exporter_ip = ?2)
                          AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3)))
             FROM flow_records
             WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')
               AND (?2 IS NULL OR exporter_ip = ?2)
               AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3)
             GROUP BY protocol ORDER BY SUM(bytes) DESC",
        )?;
        let rows = stmt
            .query_map(params![limit_seconds, exporter_ip, if_index], |row| {
                Ok(ProtocolShare {
                    protocol: row.get(0)?,
                    bytes: row.get::<_, i64>(1)?.max(0) as u64,
                    bps: row.get(3)?,
                    pps: row.get(4)?,
                    percentage: row.get(5)?,
                })
            })?
            .filter_map(|row| row.ok())
            .collect();
        Ok(rows)
    }

    pub fn top_talkers(
        &self,
        limit_seconds: i64,
        limit: usize,
    ) -> Result<Vec<TopTalker>, AppError> {
        if limit_seconds > 300 {
            let mut stmt = self.connection.prepare(
                "WITH grouped AS (
                    SELECT exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol,
                           SUM(bytes) AS total_bytes, SUM(packets) AS total_packets,
                           MAX(ingress_if_index) AS ingress_if_index,
                           MAX(egress_if_index) AS egress_if_index,
                           MAX(tcp_flags) AS tcp_flags
                    FROM aggregated_conversations_1m_v2
                    WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')
                    GROUP BY exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol
                )
                SELECT g.exporter_ip, g.source_ip, g.destination_ip, g.source_port, g.destination_port, g.protocol,
                       g.total_bytes, g.total_packets, g.total_bytes * 8 / MAX(1, ?1),
                       g.ingress_if_index, g.egress_if_index, g.tcp_flags,
                       (SELECT i.if_name FROM interface_samples i JOIN devices d ON d.id = i.device_id
                        WHERE d.ip = g.exporter_ip AND i.if_index = g.ingress_if_index
                        ORDER BY i.sampled_at DESC LIMIT 1),
                       (SELECT i.if_name FROM interface_samples i JOIN devices d ON d.id = i.device_id
                        WHERE d.ip = g.exporter_ip AND i.if_index = g.egress_if_index
                        ORDER BY i.sampled_at DESC LIMIT 1)
                FROM grouped g
                ORDER BY g.total_bytes DESC LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(params![limit_seconds, limit as i64], |row| {
                    let protocol: String = row.get(5)?;
                    let destination_port = row.get::<_, i64>(4)? as u16;
                    let packets = row.get::<_, i64>(7)?.max(0) as u64;
                    Ok(TopTalker {
                        source_ip: row.get(1)?,
                        destination_ip: row.get(2)?,
                        source_port: row.get::<_, i64>(3)? as u16,
                        destination_port,
                        app_name: application_name(protocol.clone(), destination_port),
                        protocol,
                        bytes: row.get::<_, i64>(6)?.max(0) as u64,
                        packets,
                        pps: packets / limit_seconds.max(1) as u64,
                        bps: row.get::<_, i64>(8)?.max(0) as u64,
                        ingress_if_index: row.get::<_, Option<i64>>(9)?.map(|value| value as u32),
                        egress_if_index: row.get::<_, Option<i64>>(10)?.map(|value| value as u32),
                        tcp_flags: row.get::<_, i64>(11)?.min(u8::MAX as i64) as u8,
                        ingress_if_name: row.get(12)?,
                        egress_if_name: row.get(13)?,
                    })
                })?
                .filter_map(|row| row.ok())
                .collect();
            return Ok(rows);
        }
        let mut stmt = self.connection.prepare(
                "SELECT source_ip, destination_ip, source_port, destination_port, protocol,
                    SUM(bytes), SUM(packets), SUM(bytes) * 8 / MAX(1, ?1),
                    MAX(ingress_if_index), MAX(egress_if_index), MAX(tcp_flags),
                    NULL, NULL
             FROM flow_records
             WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')
             GROUP BY exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol
             ORDER BY SUM(bytes) DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![limit_seconds, limit as i64], |row| {
                Ok(TopTalker {
                    source_ip: row.get(0)?,
                    destination_ip: row.get(1)?,
                    source_port: row.get::<_, i64>(2)? as u16,
                    destination_port: row.get::<_, i64>(3)? as u16,
                    protocol: row.get(4)?,
                    bytes: row.get::<_, i64>(5)? as u64,
                    packets: row.get::<_, i64>(6)? as u64,
                    pps: row.get::<_, i64>(6)? as u64 / limit_seconds.max(1) as u64,
                    bps: row.get::<_, i64>(7)? as u64,
                    app_name: application_name(row.get(4)?, row.get::<_, i64>(3)? as u16),
                    ingress_if_index: row.get::<_, Option<i64>>(8)?.map(|value| value as u32),
                    egress_if_index: row.get::<_, Option<i64>>(9)?.map(|value| value as u32),
                    tcp_flags: row.get::<_, i64>(10)?.min(u8::MAX as i64) as u8,
                    ingress_if_name: row.get(11)?,
                    egress_if_name: row.get(12)?,
                })
            })?
            .filter_map(|row| row.ok())
            .collect();
        Ok(rows)
    }

    pub fn top_talkers_for_context(
        &self,
        limit_seconds: i64,
        limit: usize,
        exporter_ip: Option<&str>,
        if_index: Option<i32>,
        sort_key: &str,
    ) -> Result<Vec<TopTalker>, AppError> {
        let order_by = match sort_key {
            "pps" => "total_packets",
            "bytes" => "total_bytes",
            _ => "total_bytes",
        };
        if limit_seconds > 300 {
            let sql = format!(
                "WITH grouped AS (
                    SELECT exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol,
                           SUM(bytes) AS total_bytes, SUM(packets) AS total_packets,
                           MAX(ingress_if_index) AS ingress_if_index, MAX(egress_if_index) AS egress_if_index,
                           MAX(tcp_flags) AS tcp_flags
                    FROM aggregated_conversations_1m_v2
                    WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')
                      AND (?3 IS NULL OR exporter_ip = ?3)
                      AND (?4 IS NULL OR ingress_if_index = ?4 OR egress_if_index = ?4)
                    GROUP BY exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol
                )
                SELECT g.exporter_ip, g.source_ip, g.destination_ip, g.source_port, g.destination_port, g.protocol,
                       g.total_bytes, g.total_packets, g.total_bytes * 8 / MAX(1, ?1),
                       g.ingress_if_index, g.egress_if_index, g.tcp_flags,
                       (SELECT i.if_name FROM interface_samples i JOIN devices d ON d.id = i.device_id
                        WHERE d.ip = g.exporter_ip AND i.if_index = g.ingress_if_index
                        ORDER BY i.sampled_at DESC LIMIT 1),
                       (SELECT i.if_name FROM interface_samples i JOIN devices d ON d.id = i.device_id
                        WHERE d.ip = g.exporter_ip AND i.if_index = g.egress_if_index
                        ORDER BY i.sampled_at DESC LIMIT 1)
                FROM grouped g ORDER BY g.{order_by} DESC LIMIT ?2"
            );
            let mut stmt = self.connection.prepare(&sql)?;
            let rows = stmt
                .query_map(
                    params![limit_seconds, limit as i64, exporter_ip, if_index],
                    |row| {
                        let protocol: String = row.get(5)?;
                        let destination_port = row.get::<_, i64>(4)? as u16;
                        let packets = row.get::<_, i64>(7)?.max(0) as u64;
                        Ok(TopTalker {
                            source_ip: row.get(1)?,
                            destination_ip: row.get(2)?,
                            source_port: row.get::<_, i64>(3)? as u16,
                            destination_port,
                            app_name: application_name(protocol.clone(), destination_port),
                            protocol,
                            bytes: row.get::<_, i64>(6)?.max(0) as u64,
                            packets,
                            pps: packets / limit_seconds.max(1) as u64,
                            bps: row.get::<_, i64>(8)?.max(0) as u64,
                            ingress_if_index: row
                                .get::<_, Option<i64>>(9)?
                                .map(|value| value as u32),
                            egress_if_index: row
                                .get::<_, Option<i64>>(10)?
                                .map(|value| value as u32),
                            tcp_flags: row.get::<_, i64>(11)?.min(u8::MAX as i64) as u8,
                            ingress_if_name: row.get(12)?,
                            egress_if_name: row.get(13)?,
                        })
                    },
                )?
                .filter_map(|row| row.ok())
                .collect();
            return Ok(rows);
        }
        let sql = format!(
                        "WITH grouped AS (
                                        SELECT exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol,
                                                     SUM(bytes) AS total_bytes, SUM(packets) AS total_packets,
                                                     MAX(ingress_if_index) AS ingress_if_index,
                                                     MAX(egress_if_index) AS egress_if_index,
                                                     MAX(tcp_flags) AS tcp_flags
                                        FROM flow_records
                                        WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')
                                            AND (?3 IS NULL OR exporter_ip = ?3)
                                            AND (?4 IS NULL OR ingress_if_index = ?4 OR egress_if_index = ?4)
                                        GROUP BY exporter_ip, source_ip, destination_ip, source_port, destination_port, protocol
                                )
                                SELECT g.exporter_ip, g.source_ip, g.destination_ip, g.source_port, g.destination_port, g.protocol,
                                             g.total_bytes, g.total_packets, g.total_bytes * 8 / MAX(1, ?1),
                                             g.ingress_if_index, g.egress_if_index, g.tcp_flags,
                                             (SELECT i.if_name FROM interface_samples i JOIN devices d ON d.id = i.device_id
                                                WHERE d.ip = g.exporter_ip AND i.if_index = g.ingress_if_index
                                                ORDER BY i.sampled_at DESC LIMIT 1),
                                             (SELECT i.if_name FROM interface_samples i JOIN devices d ON d.id = i.device_id
                                                WHERE d.ip = g.exporter_ip AND i.if_index = g.egress_if_index
                                                ORDER BY i.sampled_at DESC LIMIT 1)
                                FROM grouped g
                                ORDER BY g.{order_by} DESC LIMIT ?2"
        );
        let mut stmt = self.connection.prepare(&sql)?;
        let rows = stmt
            .query_map(
                params![limit_seconds, limit as i64, exporter_ip, if_index],
                |row| {
                    Ok(TopTalker {
                        source_ip: row.get(1)?,
                        destination_ip: row.get(2)?,
                        source_port: row.get::<_, i64>(3)? as u16,
                        destination_port: row.get::<_, i64>(4)? as u16,
                        protocol: row.get(5)?,
                        bytes: row.get::<_, i64>(6)?.max(0) as u64,
                        packets: row.get::<_, i64>(7)?.max(0) as u64,
                        pps: row.get::<_, i64>(7)?.max(0) as u64 / limit_seconds.max(1) as u64,
                        bps: row.get::<_, i64>(8)?.max(0) as u64,
                        app_name: application_name(row.get(5)?, row.get::<_, i64>(4)? as u16),
                        ingress_if_index: row.get::<_, Option<i64>>(9)?.map(|value| value as u32),
                        egress_if_index: row.get::<_, Option<i64>>(10)?.map(|value| value as u32),
                        tcp_flags: row.get::<_, i64>(11)?.min(u8::MAX as i64) as u8,
                        ingress_if_name: row.get(12)?,
                        egress_if_name: row.get(13)?,
                    })
                },
            )?
            .filter_map(|row| row.ok())
            .collect();
        Ok(rows)
    }

    pub fn flow_applications_for_context(
        &self,
        limit_seconds: i64,
        limit: usize,
        exporter_ip: Option<&str>,
        if_index: Option<i32>,
    ) -> Result<Vec<FlowApplicationShare>, AppError> {
        if limit_seconds > 300 {
            let total: i64 = self.connection.query_row(
                "SELECT COALESCE(SUM(bytes), 0) FROM aggregated_conversations_1m_v2
                 WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')
                   AND (?2 IS NULL OR exporter_ip = ?2)
                   AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3)",
                params![limit_seconds, exporter_ip, if_index],
                |row| row.get(0),
            )?;
            let mut stmt = self.connection.prepare(
                "SELECT protocol, destination_port, SUM(bytes) FROM aggregated_conversations_1m_v2
                 WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')
                   AND (?2 IS NULL OR exporter_ip = ?2)
                   AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3)
                 GROUP BY protocol, destination_port ORDER BY SUM(bytes) DESC LIMIT ?4",
            )?;
            let rows = stmt
                .query_map(
                    params![limit_seconds, exporter_ip, if_index, limit as i64],
                    |row| {
                        let protocol: String = row.get(0)?;
                        let port = row.get::<_, i64>(1)? as u16;
                        let bytes = row.get::<_, i64>(2)?.max(0) as u64;
                        Ok(FlowApplicationShare {
                            app_name: application_name(protocol, port),
                            bytes,
                            percentage: bytes as f64 * 100.0 / total.max(1) as f64,
                            bps: bytes as f64 * 8.0 / limit_seconds.max(1) as f64,
                        })
                    },
                )?
                .filter_map(|row| row.ok())
                .collect();
            return Ok(rows);
        }
        let mut stmt = self.connection.prepare(
            "SELECT CASE WHEN destination_port = 443 THEN 'HTTPS' WHEN destination_port = 53 THEN 'DNS'
                    WHEN destination_port = 161 THEN 'SNMP' ELSE protocol END,
                    SUM(bytes), SUM(bytes) * 100.0 / MAX(1, (SELECT SUM(bytes) FROM flow_records
                        WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')
                          AND (?2 IS NULL OR exporter_ip = ?2)
                          AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3))),
                    SUM(bytes) * 8.0 / MAX(1, ?1), destination_port, protocol
             FROM flow_records
             WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')
               AND (?2 IS NULL OR exporter_ip = ?2)
               AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3)
             GROUP BY protocol, destination_port
             ORDER BY SUM(bytes) DESC LIMIT ?4",
        )?;
        let rows = stmt
            .query_map(
                params![limit_seconds, exporter_ip, if_index, limit as i64],
                |row| {
                    Ok(FlowApplicationShare {
                        app_name: row.get(0)?,
                        bytes: row.get::<_, i64>(1)?.max(0) as u64,
                        percentage: row.get(2)?,
                        bps: row.get(3)?,
                    })
                },
            )?
            .filter_map(|row| row.ok())
            .collect();
        Ok(rows)
    }

    pub fn flow_summary(&self, limit_seconds: i64) -> Result<FlowSummary, AppError> {
        if limit_seconds > 300 {
            let (bytes, packets): (i64, i64) = self.connection.query_row(
                "SELECT COALESCE(SUM(bytes),0), COALESCE(SUM(packets),0) FROM aggregated_flows_1m WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')",
                params![limit_seconds], |row| Ok((row.get(0)?, row.get(1)?))
            )?;
            let top_protocol = self.connection.query_row(
                "SELECT protocol FROM aggregated_flows_1m WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds') GROUP BY protocol ORDER BY SUM(bytes) DESC LIMIT 1",
                params![limit_seconds], |row| row.get::<_, String>(0)
            ).unwrap_or_else(|_| "-".to_string());
            return Ok(FlowSummary {
                total_bps: bytes.max(0) as f64 * 8.0 / limit_seconds as f64,
                total_pps: packets.max(0) as f64 / limit_seconds as f64,
                active_flows: packets.max(0) as u64,
                top_protocol,
            });
        }
        let (bytes, packets, active): (i64, i64, i64) = self.connection.query_row(
            "SELECT COALESCE(SUM(bytes),0), COALESCE(SUM(packets),0), COUNT(*) FROM flow_records WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')",
            params![limit_seconds],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let top_protocol = self.connection.query_row(
            "SELECT protocol FROM flow_records WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds') GROUP BY protocol ORDER BY SUM(bytes) DESC LIMIT 1",
            params![limit_seconds],
            |row| row.get::<_, String>(0),
        ).unwrap_or_else(|_| "-".to_string());
        Ok(FlowSummary {
            total_bps: bytes.max(0) as f64 * 8.0 / limit_seconds.max(1) as f64,
            total_pps: packets.max(0) as f64 / limit_seconds.max(1) as f64,
            active_flows: active.max(0) as u64,
            top_protocol,
        })
    }

    pub fn flow_summary_for_context(
        &self,
        limit_seconds: i64,
        exporter_ip: Option<&str>,
        if_index: Option<i32>,
    ) -> Result<(FlowSummary, u64), AppError> {
        if limit_seconds > 300 {
            let (bytes, packets, active, exporters): (i64, i64, i64, i64) = self.connection.query_row(
                "SELECT COALESCE(SUM(bytes),0), COALESCE(SUM(packets),0), COUNT(*), COUNT(DISTINCT exporter_ip)
                 FROM aggregated_conversations_1m_v2
                 WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')
                   AND (?2 IS NULL OR exporter_ip = ?2)
                   AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3)",
                params![limit_seconds, exporter_ip, if_index],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?),)
            )?;
            return Ok((
                FlowSummary {
                    total_bps: bytes.max(0) as f64 * 8.0 / limit_seconds.max(1) as f64,
                    total_pps: packets.max(0) as f64 / limit_seconds.max(1) as f64,
                    active_flows: active.max(0) as u64,
                    top_protocol: "-".to_string(),
                },
                exporters.max(0) as u64,
            ));
        }
        let (bytes, packets, active, exporters): (i64, i64, i64, i64) = self.connection.query_row(
            "SELECT COALESCE(SUM(bytes),0), COALESCE(SUM(packets),0), COUNT(*), COUNT(DISTINCT exporter_ip)
             FROM flow_records
             WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')
               AND (?2 IS NULL OR exporter_ip = ?2)
               AND (?3 IS NULL OR ingress_if_index = ?3 OR egress_if_index = ?3)",
            params![limit_seconds, exporter_ip, if_index],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?),)
        )?;
        Ok((
            FlowSummary {
                total_bps: bytes.max(0) as f64 * 8.0 / limit_seconds.max(1) as f64,
                total_pps: packets.max(0) as f64 / limit_seconds.max(1) as f64,
                active_flows: active.max(0) as u64,
                top_protocol: "-".to_string(),
            },
            exporters.max(0) as u64,
        ))
    }

    pub fn flow_applications(
        &self,
        limit_seconds: i64,
        limit: usize,
    ) -> Result<Vec<FlowApplicationShare>, AppError> {
        if limit_seconds > 300 {
            let total: i64 = self.connection.query_row("SELECT COALESCE(SUM(bytes),0) FROM aggregated_conversations_1m_v2 WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')", params![limit_seconds], |row| row.get(0))?;
            let mut stmt = self.connection.prepare("SELECT protocol, destination_port, SUM(bytes) FROM aggregated_conversations_1m_v2 WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds') GROUP BY protocol, destination_port ORDER BY SUM(bytes) DESC LIMIT ?2")?;
            let rows = stmt
                .query_map(params![limit_seconds, limit as i64], |row| {
                    let protocol: String = row.get(0)?;
                    let port = row.get::<_, i64>(1)? as u16;
                    let bytes = row.get::<_, i64>(2)?.max(0) as u64;
                    Ok(FlowApplicationShare {
                        app_name: application_name(protocol, port),
                        bytes,
                        percentage: bytes as f64 * 100.0 / total.max(1) as f64,
                        bps: bytes as f64 * 8.0 / limit_seconds as f64,
                    })
                })?
                .filter_map(|row| row.ok())
                .collect();
            return Ok(rows);
        }
        let total: i64 = self.connection.query_row(
            "SELECT COALESCE(SUM(bytes),0) FROM flow_records WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')",
            params![limit_seconds], |row| row.get(0))?;
        let mut stmt = self.connection.prepare(
            "SELECT protocol, destination_port, SUM(bytes) FROM flow_records WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds') GROUP BY protocol, destination_port ORDER BY SUM(bytes) DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![limit_seconds, limit as i64], |row| {
                let protocol: String = row.get(0)?;
                let port: u16 = row.get::<_, i64>(1)? as u16;
                let bytes = row.get::<_, i64>(2)?.max(0) as u64;
                Ok(FlowApplicationShare {
                    app_name: application_name(protocol, port),
                    bytes,
                    percentage: bytes as f64 * 100.0 / total.max(1) as f64,
                    bps: bytes as f64 * 8.0 / limit_seconds.max(1) as f64,
                })
            })?
            .filter_map(|row| row.ok())
            .collect();
        Ok(rows)
    }

    pub fn flow_endpoints(
        &self,
        limit_seconds: i64,
        source: bool,
        limit: usize,
    ) -> Result<Vec<FlowEndpointShare>, AppError> {
        if limit_seconds > 300 {
            let total: i64 = self.connection.query_row("SELECT COALESCE(SUM(bytes),0) FROM aggregated_conversations_1m_v2 WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds')", params![limit_seconds], |row| row.get(0))?;
            let column = if source {
                "source_ip"
            } else {
                "destination_ip"
            };
            let sql = format!(
                "SELECT {column}, SUM(bytes) FROM aggregated_conversations_1m_v2 WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds') GROUP BY {column} ORDER BY SUM(bytes) DESC LIMIT ?2"
            );
            let mut stmt = self.connection.prepare(&sql)?;
            let rows = stmt
                .query_map(params![limit_seconds, limit as i64], |row| {
                    let bytes = row.get::<_, i64>(1)?.max(0) as u64;
                    Ok(FlowEndpointShare {
                        ip: row.get(0)?,
                        bps: bytes as f64 * 8.0 / limit_seconds as f64,
                        percentage: bytes as f64 * 100.0 / total.max(1) as f64,
                    })
                })?
                .filter_map(|row| row.ok())
                .collect();
            return Ok(rows);
        }
        let total: i64 = self.connection.query_row("SELECT COALESCE(SUM(bytes),0) FROM flow_records WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds')", params![limit_seconds], |row| row.get(0))?;
        let column = if source {
            "source_ip"
        } else {
            "destination_ip"
        };
        let sql = format!(
            "SELECT {column}, SUM(bytes) FROM flow_records WHERE observed_at >= datetime('now', '-' || ?1 || ' seconds') GROUP BY {column} ORDER BY SUM(bytes) DESC LIMIT ?2"
        );
        let mut stmt = self.connection.prepare(&sql)?;
        let rows = stmt
            .query_map(params![limit_seconds, limit as i64], |row| {
                let bytes = row.get::<_, i64>(1)?.max(0) as u64;
                Ok(FlowEndpointShare {
                    ip: row.get(0)?,
                    bps: bytes as f64 * 8.0 / limit_seconds.max(1) as f64,
                    percentage: bytes as f64 * 100.0 / total.max(1) as f64,
                })
            })?
            .filter_map(|row| row.ok())
            .collect();
        Ok(rows)
    }

    pub fn flow_timeseries(&self, limit_seconds: i64) -> Result<Vec<FlowTimeseries>, AppError> {
        let mut stmt = self.connection.prepare("SELECT bucket_start, protocol, SUM(bytes) FROM aggregated_flows_1m WHERE bucket_start >= datetime('now', '-' || ?1 || ' seconds') GROUP BY bucket_start, protocol ORDER BY bucket_start")?;
        let mut by_time: HashMap<String, (f64, f64, f64)> = HashMap::new();
        for row in stmt
            .query_map(params![limit_seconds], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?.max(0) as f64,
                ))
            })?
            .filter_map(|row| row.ok())
        {
            let entry = by_time.entry(row.0).or_default();
            match row.1.as_str() {
                "UDP" => entry.0 += row.2 * 8.0 / 60.0,
                "TCP" => entry.1 += row.2 * 8.0 / 60.0,
                "ICMP" => entry.2 += row.2 * 8.0 / 60.0,
                _ => {}
            }
        }
        Ok(by_time
            .into_iter()
            .map(|(timestamp, (udp_bps, tcp_bps, icmp_bps))| FlowTimeseries {
                timestamp,
                udp_bps,
                tcp_bps,
                icmp_bps,
            })
            .collect())
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
        let _ = self.connection.execute(
            "DELETE FROM flow_records WHERE observed_at < datetime('now', '-5 minutes')",
            [],
        )?;
        let _ = self.connection.execute(
            "DELETE FROM aggregated_flows_1m WHERE bucket_start < datetime('now', '-24 hours')",
            [],
        )?;
        let _ = self.connection.execute(
            "DELETE FROM aggregated_conversations_1m_v2 WHERE bucket_start < datetime('now', '-24 hours')",
            [],
        )?;
        self.enforce_flow_database_limit()?;

        Ok(deleted)
    }

    fn enforce_flow_database_limit(&self) -> Result<(), AppError> {
        const MAX_DATABASE_BYTES: i64 = 2 * 1024 * 1024 * 1024;
        let page_size: i64 = self
            .connection
            .query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let page_count: i64 = self
            .connection
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        if page_size.saturating_mul(page_count) <= MAX_DATABASE_BYTES {
            return Ok(());
        }

        self.connection.execute(
            "DELETE FROM aggregated_flows_1m WHERE bucket_start IN (SELECT bucket_start FROM aggregated_flows_1m ORDER BY bucket_start ASC LIMIT 1440)",
            [],
        )?;
        self.connection.execute(
            "DELETE FROM flow_records WHERE id IN (SELECT id FROM flow_records ORDER BY observed_at ASC LIMIT 10000)",
            [],
        )?;
        self.connection.execute_batch("PRAGMA incremental_vacuum")?;
        Ok(())
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
                in_packets: 0,
                out_packets: 0,
                in_discards: row.get::<_, i64>(5)? as u64,
                out_discards: row.get::<_, i64>(6)? as u64,
                late_collisions: row.get::<_, i64>(7)? as u64,
                fcs_errors: 0,
                alignment_errors: 0,
                frame_too_longs: 0,
                internal_mac_receive_errors: 0,
                rx_optical_power_dbm: None,
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
                    in_errors, out_errors, in_packets, out_packets, in_discards, out_discards, late_collisions,
                    fcs_errors, alignment_errors, frame_too_longs, internal_mac_receive_errors, rx_optical_power_dbm,
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
                    in_packets: row.get::<_, i64>(7)? as u64,
                    out_packets: row.get::<_, i64>(8)? as u64,
                    in_discards: row.get::<_, i64>(9)? as u64,
                    out_discards: row.get::<_, i64>(10)? as u64,
                    late_collisions: row.get::<_, i64>(11)? as u64,
                    fcs_errors: row.get::<_, i64>(12)? as u64,
                    alignment_errors: row.get::<_, i64>(13)? as u64,
                    frame_too_longs: row.get::<_, i64>(14)? as u64,
                    internal_mac_receive_errors: row.get::<_, i64>(15)? as u64,
                    rx_optical_power_dbm: row.get(16)?,
                    in_octets: row.get::<_, i64>(17)? as u64,
                    out_octets: row.get::<_, i64>(18)? as u64,
                    bandwidth_utilization: row.get(19)?,
                    sampled_at: row.get(20)?,
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
                    in_errors, out_errors, in_packets, out_packets, in_discards, out_discards, late_collisions,
                    fcs_errors, alignment_errors, frame_too_longs, internal_mac_receive_errors, rx_optical_power_dbm,
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
                    in_packets: row.get::<_, i64>(7)? as u64,
                    out_packets: row.get::<_, i64>(8)? as u64,
                    in_discards: row.get::<_, i64>(9)? as u64,
                    out_discards: row.get::<_, i64>(10)? as u64,
                    late_collisions: row.get::<_, i64>(11)? as u64,
                    fcs_errors: row.get::<_, i64>(12)? as u64,
                    alignment_errors: row.get::<_, i64>(13)? as u64,
                    frame_too_longs: row.get::<_, i64>(14)? as u64,
                    internal_mac_receive_errors: row.get::<_, i64>(15)? as u64,
                    rx_optical_power_dbm: row.get(16)?,
                    in_octets: row.get::<_, i64>(17)? as u64,
                    out_octets: row.get::<_, i64>(18)? as u64,
                    bandwidth_utilization: row.get(19)?,
                    sampled_at: row.get(20)?,
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
                    in_errors, out_errors, in_packets, out_packets, in_discards, out_discards, late_collisions,
                    fcs_errors, alignment_errors, frame_too_longs, internal_mac_receive_errors, rx_optical_power_dbm,
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
                in_packets: row.get::<_, i64>(7)? as u64,
                out_packets: row.get::<_, i64>(8)? as u64,
                in_discards: row.get::<_, i64>(9)? as u64,
                out_discards: row.get::<_, i64>(10)? as u64,
                late_collisions: row.get::<_, i64>(11)? as u64,
                fcs_errors: row.get::<_, i64>(12)? as u64,
                alignment_errors: row.get::<_, i64>(13)? as u64,
                frame_too_longs: row.get::<_, i64>(14)? as u64,
                internal_mac_receive_errors: row.get::<_, i64>(15)? as u64,
                rx_optical_power_dbm: row.get(16)?,
                in_octets: row.get::<_, i64>(17)? as u64,
                out_octets: row.get::<_, i64>(18)? as u64,
                bandwidth_utilization: row.get(19)?,
                sampled_at: row.get(20)?,
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
                    in_errors, out_errors, in_packets, out_packets, in_discards, out_discards, late_collisions,
                    fcs_errors, alignment_errors, frame_too_longs, internal_mac_receive_errors, rx_optical_power_dbm,
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
                    in_packets: row.get::<_, i64>(7)? as u64,
                    out_packets: row.get::<_, i64>(8)? as u64,
                    in_discards: row.get::<_, i64>(9)? as u64,
                    out_discards: row.get::<_, i64>(10)? as u64,
                    late_collisions: row.get::<_, i64>(11)? as u64,
                    fcs_errors: row.get::<_, i64>(12)? as u64,
                    alignment_errors: row.get::<_, i64>(13)? as u64,
                    frame_too_longs: row.get::<_, i64>(14)? as u64,
                    internal_mac_receive_errors: row.get::<_, i64>(15)? as u64,
                    rx_optical_power_dbm: row.get(16)?,
                    in_octets: row.get::<_, i64>(17)? as u64,
                    out_octets: row.get::<_, i64>(18)? as u64,
                    bandwidth_utilization: row.get(19)?,
                    sampled_at: row.get(20)?,
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
            "SELECT if_index, in_errors, out_errors, in_discards, out_discards, late_collisions,
                    fcs_errors, alignment_errors, frame_too_longs, internal_mac_receive_errors
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
            fcs_errors: u64,
            alignment_errors: u64,
            frame_too_longs: u64,
            internal_mac_receive_errors: u64,
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
                    fcs_errors: row.get::<_, i64>(6)? as u64,
                    alignment_errors: row.get::<_, i64>(7)? as u64,
                    frame_too_longs: row.get::<_, i64>(8)? as u64,
                    internal_mac_receive_errors: row.get::<_, i64>(9)? as u64,
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
                    out_discards_delta: counter32_delta(previous.out_discards, latest.out_discards),
                    late_collisions_delta: counter32_delta(
                        previous.late_collisions,
                        latest.late_collisions,
                    ),
                    fcs_errors_delta: counter32_delta(previous.fcs_errors, latest.fcs_errors),
                    alignment_errors_delta: counter32_delta(
                        previous.alignment_errors,
                        latest.alignment_errors,
                    ),
                    frame_too_longs_delta: counter32_delta(
                        previous.frame_too_longs,
                        latest.frame_too_longs,
                    ),
                    internal_mac_receive_errors_delta: counter32_delta(
                        previous.internal_mac_receive_errors,
                        latest.internal_mac_receive_errors,
                    ),
                }
            };
            deltas.insert(if_index, delta);
        }

        Ok(deltas)
    }
}

fn application_name(protocol: String, destination_port: u16) -> String {
    let custom_name = crate::exe_dir().join("config").join("services.toml");
    if let Ok(content) = std::fs::read_to_string(custom_name)
        && let Ok(value) = content.parse::<toml::Value>()
        && let Some(name) = value
            .get("services")
            .and_then(|services| services.get(destination_port.to_string()))
            .and_then(toml::Value::as_str)
    {
        return format!("{name} ({protocol}/{destination_port})");
    }
    let name = match destination_port {
        22 => "SSH",
        53 => "DNS",
        80 => "HTTP",
        123 => "NTP",
        161 => "SNMP",
        443 => "HTTPS",
        3306 => "MySQL",
        5432 => "PostgreSQL",
        8080 => "HTTP-Alt",
        _ => return format!("{protocol}/{destination_port}"),
    };
    format!("{name} ({protocol}/{destination_port})")
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
