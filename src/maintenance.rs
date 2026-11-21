//! Shared maintenance periods and Community release reconciliation.
use crate::{
    alert::{AlertEvent, AlertKind, AlertSeverity},
    config::AlertConfig,
    notifier::{AlertNotification, MaintenanceGate, Notifier},
};
use chrono::{TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path, sync::Mutex, time::Duration};

#[derive(Debug, Clone, Serialize)]
pub struct MuteState {
    pub device_id: Option<i64>,
    pub device_name: Option<String>,
    pub device_ip: Option<String>,
    pub enabled: bool,
    pub muted_until: Option<i64>,
    pub muted_until_utc: Option<String>,
    pub actor: String,
    pub reason: String,
    pub updated_at: i64,
}

impl MuteState {
    pub fn active(&self, now: i64) -> bool {
        self.enabled && self.muted_until.is_none_or(|end| now < end)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MaintenanceStatus {
    pub server_time: i64,
    pub global: Option<MuteState>,
    pub devices: BTreeMap<i64, MuteState>,
}

impl MaintenanceStatus {
    pub fn effective(&self, id: Option<i64>, now: i64) -> Option<&MuteState> {
        self.global.as_ref().filter(|s| s.active(now)).or_else(|| {
            id.and_then(|id| self.devices.get(&id))
                .filter(|s| s.active(now))
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MaintenanceChange {
    pub id: i64,
    pub device_id: Option<i64>,
    pub enabled: bool,
    pub muted_until: Option<i64>,
    pub actor: String,
    pub reason: String,
    pub changed_at: i64,
}

fn err(error: rusqlite::Error) -> String {
    format!("maintenance database: {error}")
}

type MetricSample = (Option<f64>, Option<f64>);

pub fn migrate_periods(db: &Connection) -> Result<(), String> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS maintenance_state(
            target INTEGER PRIMARY KEY CHECK(target>=0), enabled INTEGER NOT NULL CHECK(enabled IN(0,1)),
            muted_until INTEGER,actor TEXT NOT NULL,reason TEXT NOT NULL,updated_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS maintenance_periods(
            id INTEGER PRIMARY KEY,target INTEGER NOT NULL,started_at INTEGER NOT NULL,ended_at INTEGER,
            actor TEXT NOT NULL,reason TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS maintenance_period_target ON maintenance_periods(target,started_at,ended_at);
         CREATE TABLE IF NOT EXISTS maintenance_changes(
            id INTEGER PRIMARY KEY,target INTEGER NOT NULL,enabled INTEGER NOT NULL,muted_until INTEGER,
            actor TEXT NOT NULL,reason TEXT NOT NULL,changed_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS basic_maintenance_reconciled(
            device_id INTEGER NOT NULL, release_id INTEGER NOT NULL, risk TEXT NOT NULL, checked_at INTEGER NOT NULL,
            PRIMARY KEY(device_id,release_id,risk));",
    ).map_err(err)
}

pub fn is_suppressed(
    db: &Connection,
    id: Option<i64>,
    occurred_at: i64,
    now: i64,
) -> Result<bool, String> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM maintenance_periods WHERE
        (target=0 OR target=?1) AND started_at<=?3 AND (ended_at IS NULL OR ended_at>MIN(?2,?3)))",
        params![id, occurred_at, now],
        |r| r.get(0),
    )
    .map_err(err)
}

/// Call inside a write transaction. Editions supply their own device lookup and pending-risk tracking.
pub fn write_period(
    db: &Connection,
    device_id: Option<i64>,
    enabled: bool,
    duration: Option<i64>,
    reason: &str,
    actor: &str,
    now: i64,
) -> Result<MuteState, String> {
    if device_id.is_some_and(|id| id <= 0)
        || actor.trim().is_empty()
        || reason.chars().count() > 500
        || duration.is_some_and(|seconds| ![3600, 14400, 86400].contains(&seconds))
    {
        return Err("invalid maintenance settings".into());
    }
    let until = if enabled {
        duration
            .map(|seconds| now.checked_add(seconds).ok_or("invalid maintenance expiry"))
            .transpose()?
    } else {
        None
    };
    let target = device_id.unwrap_or(0);
    db.execute("UPDATE maintenance_periods SET ended_at=?1 WHERE target=?2 AND (ended_at IS NULL OR ended_at>?1)",params![now,target]).map_err(err)?;
    if enabled {
        db.execute("INSERT INTO maintenance_periods(target,started_at,ended_at,actor,reason) VALUES(?1,?2,?3,?4,?5)",
            params![target,now,until,actor,reason]).map_err(err)?;
    }
    db.execute("INSERT INTO maintenance_state(target,enabled,muted_until,actor,reason,updated_at) VALUES(?1,?2,?3,?4,?5,?6)
        ON CONFLICT(target) DO UPDATE SET enabled=excluded.enabled,muted_until=excluded.muted_until,
        actor=excluded.actor,reason=excluded.reason,updated_at=excluded.updated_at",params![target,enabled,until,actor,reason,now]).map_err(err)?;
    db.execute("INSERT INTO maintenance_changes(target,enabled,muted_until,actor,reason,changed_at) VALUES(?1,?2,?3,?4,?5,?6)",
        params![target,enabled,until,actor,reason,now]).map_err(err)?;
    Ok(MuteState {
        device_id,
        device_name: None,
        device_ip: None,
        enabled,
        muted_until: until,
        muted_until_utc: timestamp_string(until),
        actor: actor.into(),
        reason: reason.into(),
        updated_at: now,
    })
}

fn timestamp_string(value: Option<i64>) -> Option<String> {
    value
        .and_then(|at| Utc.timestamp_opt(at, 0).single())
        .map(|at| at.to_rfc3339())
}

pub struct MaintenanceStore {
    pub(crate) database: Mutex<Connection>,
}

impl MaintenanceStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let db = Connection::open(path).map_err(err)?;
        db.busy_timeout(Duration::from_secs(3)).map_err(err)?;
        migrate_periods(&db)?;
        Ok(Self {
            database: Mutex::new(db),
        })
    }

    pub fn maintenance_status(&self, now: i64) -> Result<MaintenanceStatus, String> {
        let db = self.database.lock().map_err(|e| e.to_string())?;
        let mut query = db
            .prepare(
                "SELECT s.target,s.enabled,s.muted_until,s.actor,s.reason,s.updated_at,d.name,d.ip
             FROM maintenance_state s LEFT JOIN devices d ON d.id=s.target
             WHERE s.target=0 OR d.id IS NOT NULL",
            )
            .map_err(err)?;
        let mut status = MaintenanceStatus {
            server_time: now,
            global: None,
            devices: BTreeMap::new(),
        };
        for row in query
            .query_map([], |r| {
                let id: i64 = r.get(0)?;
                let until = r.get(2)?;
                Ok(MuteState {
                    device_id: (id != 0).then_some(id),
                    device_name: r.get(6)?,
                    device_ip: r.get(7)?,
                    enabled: r.get(1)?,
                    muted_until: until,
                    muted_until_utc: timestamp_string(until),
                    actor: r.get(3)?,
                    reason: r.get(4)?,
                    updated_at: r.get(5)?,
                })
            })
            .map_err(err)?
        {
            let mut mute = row.map_err(err)?;
            mute.enabled = mute.active(now);
            if let Some(id) = mute.device_id {
                status.devices.insert(id, mute);
            } else {
                status.global = Some(mute);
            }
        }
        Ok(status)
    }

    pub fn set_maintenance(
        &self,
        device_id: Option<i64>,
        enabled: bool,
        duration: Option<i64>,
        reason: &str,
        actor: &str,
        now: i64,
    ) -> Result<MuteState, String> {
        let mut db = self.database.lock().map_err(|e| e.to_string())?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(err)?;
        if let Some(id) = device_id {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM devices WHERE id=?1)",
                    [id],
                    |r| r.get(0),
                )
                .map_err(err)?;
            if !exists {
                return Err("device not found".into());
            }
        }
        let state = write_period(&tx, device_id, enabled, duration, reason, actor, now)?;
        tx.commit().map_err(err)?;
        Ok(state)
    }

    pub fn history(&self, limit: usize) -> Result<Vec<MaintenanceChange>, String> {
        let db = self.database.lock().map_err(|e| e.to_string())?;
        let mut query = db.prepare(
            "SELECT id,target,enabled,muted_until,actor,reason,changed_at FROM maintenance_changes ORDER BY id DESC LIMIT ?1",
        ).map_err(err)?;
        query
            .query_map([limit.clamp(1, 1000)], |r| {
                let id: i64 = r.get(1)?;
                Ok(MaintenanceChange {
                    id: r.get(0)?,
                    device_id: (id != 0).then_some(id),
                    enabled: r.get(2)?,
                    muted_until: r.get(3)?,
                    actor: r.get(4)?,
                    reason: r.get(5)?,
                    changed_at: r.get(6)?,
                })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)
    }

    pub(crate) fn enqueue_release_summaries(
        &self,
        notifier: &Notifier,
        now: i64,
        thresholds: Option<&AlertConfig>,
    ) -> Result<(), String> {
        let mut db = self.database.lock().map_err(|e| e.to_string())?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(err)?;
        reconcile(&tx, &tx, notifier, now, thresholds, false)?;
        tx.commit().map_err(err)
    }

    #[cfg(test)]
    pub(crate) fn test_fixture() -> Self {
        let store = Self::open(":memory:").unwrap();
        store.database.lock().unwrap().execute_batch(
            "CREATE TABLE devices(id INTEGER PRIMARY KEY,name TEXT,ip TEXT,status TEXT,updated_at TEXT);
             INSERT INTO devices VALUES(1,'router','192.0.2.1','warning',CURRENT_TIMESTAMP),
                                       (2,'switch','192.0.2.2','online',CURRENT_TIMESTAMP);",
        ).unwrap();
        store
    }
}

impl MaintenanceGate for MaintenanceStore {
    fn suppress_notification(&self, alert: &AlertNotification, now: i64) -> Result<bool, String> {
        let db = self.database.lock().map_err(|e| e.to_string())?;
        let id = match alert.device_id {
            Some(id) => Some(id),
            None => db
                .query_row(
                    "SELECT id FROM devices WHERE ip=?1",
                    [&alert.device_ip],
                    |r| r.get(0),
                )
                .optional()
                .map_err(err)?,
        };
        let occurred = chrono::DateTime::parse_from_rfc3339(&alert.occurred_at)
            .map_err(|e| format!("invalid alert timestamp: {e}"))?
            .timestamp();
        is_suppressed(&db, id, occurred, now)
    }
}

/// Enqueue only system metric release summaries within the caller's transaction.
pub fn enqueue_system_release_summaries(
    store: &Connection,
    core: &Connection,
    notifier: &Notifier,
    now: i64,
    thresholds: &AlertConfig,
) -> Result<Vec<AlertEvent>, String> {
    reconcile(store, core, notifier, now, Some(thresholds), true)
}

/// Consume the first fresh post-release evaluation of each basic risk.
fn reconcile(
    store: &Connection,
    core: &Connection,
    notifier: &Notifier,
    now: i64,
    thresholds: Option<&AlertConfig>,
    metrics_only: bool,
) -> Result<Vec<AlertEvent>, String> {
    let has_ports: bool = !metrics_only && core.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='interface_alert_settings')",
        [], |r| r.get(0),
    ).map_err(err)?;
    let has_metrics: bool = thresholds.is_some() && core.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='device_metrics')",
        [], |r| r.get(0),
    ).map_err(err)?;
    let mut samples = if has_metrics {
        Some(
            core.prepare(
                "SELECT cpu_usage,memory_usage FROM device_metrics
             WHERE device_id=?1
               AND sampled_at>=date(?2,'unixepoch','-1 day')
               AND sampled_at<date(?3,'unixepoch','+2 days')
               AND CAST(strftime('%s',sampled_at) AS INTEGER) BETWEEN ?2 AND ?3
             ORDER BY julianday(sampled_at) DESC, id DESC LIMIT 1",
            )
            .map_err(err)?,
        )
    } else {
        None
    };
    let mut devices = core
        .prepare("SELECT id,name,ip,status,CAST(strftime('%s',updated_at) AS INTEGER) FROM devices")
        .map_err(err)?;
    let mut release = store
        .prepare(
            "SELECT id,ended_at FROM maintenance_periods WHERE (target=0 OR target=?1)
           AND ended_at<=?2 AND ended_at>=?3 ORDER BY ended_at DESC,id DESC LIMIT 1",
        )
        .map_err(err)?;
    let mut output = Vec::new();
    for row in devices
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<i64>>(4)?,
            ))
        })
        .map_err(err)?
    {
        let (id, name, ip, status, observed) = row.map_err(err)?;
        let Some((release_id, ended)) = release
            .query_row(params![id, now, now - 900], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            })
            .optional()
            .map_err(err)?
        else {
            continue;
        };
        if is_suppressed(store, Some(id), now, now)? {
            continue;
        }

        let mut risks = Vec::new();
        if !metrics_only && observed.is_some_and(|at| at > ended && at <= now && at >= now - 900) {
            let value = match status.as_str() {
                "offline" | "critical" => Some((
                    AlertSeverity::Critical,
                    format!("Current availability: {status}"),
                )),
                "warning" => Some((
                    AlertSeverity::Warning,
                    format!("Current availability: {status}"),
                )),
                _ => None,
            };
            risks.push(("availability".to_string(), value));
        }
        if has_ports {
            let mut ports = core
                .prepare(
                    "SELECT if_index,if_name,if_admin_status,if_oper_status,active
                   FROM interface_alert_settings WHERE device_id=?1
                   AND (link_down_alert_enabled=1 OR topology_link=1)
                   AND observed_at>?2 AND observed_at<=?3 AND observed_at>=?4 ORDER BY if_index",
                )
                .map_err(err)?;
            for row in ports
                .query_map(params![id, ended, now, now - 900], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, Option<i64>>(3)?,
                        r.get::<_, bool>(4)?,
                    ))
                })
                .map_err(err)?
            {
                let (index, port, admin, oper, active) = row.map_err(err)?;
                let value = (admin == Some(1) && oper == Some(2) && active).then(|| {
                    (
                        AlertSeverity::Critical,
                        format!("Important port down: {port} (ifIndex {index})"),
                    )
                });
                // Unknown data is not evidence that the first post-release evaluation completed.
                if admin.is_some() && oper.is_some() {
                    risks.push((format!("port:{index}"), value));
                }
            }
        }
        if let (Some(query), Some(thresholds)) = (&mut samples, thresholds) {
            let mut latest = |start, end| -> Result<Option<MetricSample>, String> {
                query
                    .query_row(params![id, start, end], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .optional()
                    .map_err(err)
            };
            if let Some((cpu, memory)) = latest((ended + 1).max(now - 900), now)?
                && let Some((previous_cpu, previous_memory)) = latest(ended - 900, ended)?
            {
                for (key, label, previous, current, limits) in [
                    ("cpu", "CPU", previous_cpu, cpu, &thresholds.cpu),
                    (
                        "memory",
                        "Memory",
                        previous_memory,
                        memory,
                        &thresholds.memory,
                    ),
                ] {
                    if let (Some(previous), Some(current)) = (previous, current) {
                        let continuing = limits
                            .severity(previous)
                            .and_then(|_| limits.severity(current));
                        let value = continuing.map(|severity| {
                            let threshold = if severity == AlertSeverity::Critical {
                                limits.critical
                            } else {
                                limits.warning
                            };
                            (
                                severity,
                                format!(
                                    "{label} usage remains high: {current:.2}% ≥ {threshold:.2}%"
                                ),
                            )
                        });
                        risks.push((key.into(), value));
                    }
                }
            }
        }
        let mut messages = Vec::new();
        let mut summarized = Vec::new();
        let mut severity = AlertSeverity::Warning;
        for (risk, value) in risks {
            let inserted = store.execute(
                "INSERT OR IGNORE INTO basic_maintenance_reconciled(device_id,release_id,risk,checked_at) VALUES(?1,?2,?3,?4)",
                params![id,release_id,risk,now],
            ).map_err(err)?;
            if inserted != 0
                && let Some((level, message)) = value
            {
                if level == AlertSeverity::Critical {
                    severity = level;
                }
                summarized.push(risk);
                messages.push(message);
            }
        }
        if !messages.is_empty() {
            let event = AlertEvent {
                id: format!(
                    "maintenance-basic-{id}-{release_id}-{}",
                    summarized.join("|")
                ),
                device_id: Some(id),
                device_name: name,
                device_ip: ip,
                interface_id: None,
                interface_name: None,
                kind: AlertKind::Custom(
                    if metrics_only {
                        "MAINTENANCE_RESUMED_SYSTEM_METRICS"
                    } else {
                        "MAINTENANCE_RESUMED_AVAILABILITY"
                    }
                    .into(),
                ),
                severity,
                message: format!(
                    "Maintenance ended; continuing risks: {}",
                    messages.join("; ")
                ),
                observed_value: None,
                threshold: None,
                occurred_at: Utc
                    .timestamp_opt(now, 0)
                    .single()
                    .ok_or("invalid maintenance time")?,
            };
            notifier
                .enqueue_release(store, &AlertNotification::from_event(event.clone()))
                .map_err(|e| e.to_string())?;
            output.push(event);
        }
    }
    store
        .execute(
            "DELETE FROM basic_maintenance_reconciled WHERE checked_at<?1",
            [now - 86400],
        )
        .map_err(err)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifier::{NotifierConfig, WebhookEndpointConfig, outbox::Outbox};

    #[test]
    fn system_metric_release_uses_live_thresholds_and_waits_for_each_fresh_sample() {
        let dir = tempfile::tempdir_in("target").unwrap();
        let path = dir.path().join("metrics.db");
        let config_path = dir.path().join("config.toml");
        let store = MaintenanceStore::open(&path).unwrap();
        let outbox = Outbox::open(&path).unwrap();
        let notifier = Notifier::new(NotifierConfig {
            webhook: WebhookEndpointConfig {
                enabled: true,
                webhook_url: Some("http://127.0.0.1:1".into()),
                webhook_url_env: None,
            },
            ..NotifierConfig::default()
        })
        .unwrap()
        .with_outbox(outbox);
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE devices(id INTEGER PRIMARY KEY,name TEXT,ip TEXT,status TEXT,updated_at TEXT);
             CREATE TABLE device_metrics(id INTEGER PRIMARY KEY,device_id INTEGER,cpu_usage INTEGER,memory_usage INTEGER,sampled_at TEXT);
             INSERT INTO devices VALUES(1,'router','192.0.2.1','online',datetime(105,'unixepoch'));
             INSERT INTO device_metrics VALUES(1,1,76,88,datetime(99,'unixepoch'));"
        ).unwrap();
        store
            .set_maintenance(None, true, None, "work", "test", 90)
            .unwrap();
        store
            .set_maintenance(None, false, None, "done", "test", 100)
            .unwrap();
        let load = || crate::notifications::load_release_alert_config(&config_path).unwrap();
        std::fs::write(
            &config_path,
            "[alert.cpu]\nwarning=85\ncritical=95\n[alert.memory]\nwarning=70\ncritical=85\n",
        )
        .unwrap();
        store
            .enqueue_release_summaries(&notifier, 101, Some(&load()))
            .unwrap();
        let count = || {
            db.query_row("SELECT COUNT(*) FROM notification_outbox", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap()
        };
        assert_eq!(count(), 0);

        std::fs::write(
            &config_path,
            "[alert.cpu]\nwarning=70\ncritical=80\n[alert.memory]\nwarning=70\ncritical=85\n",
        )
        .unwrap();
        db.execute(
            "INSERT INTO device_metrics VALUES(2,1,77,NULL,datetime(102,'unixepoch'))",
            [],
        )
        .unwrap();
        store
            .enqueue_release_summaries(&notifier, 103, Some(&load()))
            .unwrap();
        assert_eq!(count(), 1);
        db.execute(
            "INSERT INTO device_metrics VALUES(3,1,79,90,datetime(104,'unixepoch'))",
            [],
        )
        .unwrap();
        drop(store);
        let restarted = MaintenanceStore::open(&path).unwrap();
        restarted
            .enqueue_release_summaries(&notifier, 105, Some(&load()))
            .unwrap();
        restarted
            .enqueue_release_summaries(&notifier, 106, Some(&load()))
            .unwrap();
        assert_eq!(count(), 2);
        let mut query = db
            .prepare("SELECT last_alert FROM notification_outbox ORDER BY id")
            .unwrap();
        let alerts = query
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| serde_json::from_str::<AlertNotification>(&r.unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert!(
            alerts[0]
                .message
                .contains("CPU usage remains high: 77.00% ≥ 70.00%")
        );
        assert!(!alerts[0].message.contains("Memory"));
        assert!(
            alerts[1]
                .message
                .contains("Memory usage remains high: 90.00% ≥ 85.00%")
        );
        assert!(!alerts[1].message.contains("CPU"));
    }

    #[test]
    fn release_requires_fresh_observations_and_reports_confirmed_risks_once() {
        let dir = tempfile::tempdir_in("target").unwrap();
        let path = dir.path().join("release.db");
        let store = MaintenanceStore::open(&path).unwrap();
        let outbox = Outbox::open(&path).unwrap();
        let notifier = Notifier::new(NotifierConfig {
            webhook: WebhookEndpointConfig {
                enabled: true,
                webhook_url: Some("http://127.0.0.1:1".into()),
                webhook_url_env: None,
            },
            ..NotifierConfig::default()
        })
        .unwrap()
        .with_outbox(outbox);
        {
            let db = store.database.lock().unwrap();
            db.execute_batch(
                "CREATE TABLE devices(id INTEGER PRIMARY KEY,name TEXT,ip TEXT,status TEXT,updated_at TEXT);
                 INSERT INTO devices VALUES(1,'router','192.0.2.1','warning',datetime(99,'unixepoch'));
                 CREATE TABLE interface_alert_settings(
                     device_id INTEGER,if_index INTEGER,if_name TEXT,link_down_alert_enabled INTEGER,
                     topology_link INTEGER,if_admin_status INTEGER,if_oper_status INTEGER,
                     observed_at INTEGER,active INTEGER);
                 INSERT INTO interface_alert_settings VALUES
                     (1,1,'Gi1',0,1,1,2,99,1),
                     (1,2,'AdminDown',1,0,2,2,103,1),
                     (1,3,'Unconfirmed',1,0,1,NULL,103,1),
                     (1,4,'Disabled',0,0,1,2,103,1),
                     (1,5,'Inactive',1,0,1,2,103,0);",
            ).unwrap();
        }
        store
            .set_maintenance(None, true, None, "work", "test", 90)
            .unwrap();
        store
            .set_maintenance(Some(1), true, None, "overlap", "test", 91)
            .unwrap();
        store
            .set_maintenance(None, false, None, "finished", "test", 100)
            .unwrap();
        store
            .enqueue_release_summaries(&notifier, 101, None)
            .unwrap();
        store
            .set_maintenance(Some(1), false, None, "finished", "test", 102)
            .unwrap();
        store
            .enqueue_release_summaries(&notifier, 103, None)
            .unwrap();
        let db = Connection::open(path).unwrap();
        let count = || {
            db.query_row("SELECT COUNT(*) FROM notification_outbox", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap()
        };
        assert_eq!(count(), 0);
        db.execute_batch(
            "UPDATE devices SET updated_at=datetime(104,'unixepoch');
             UPDATE interface_alert_settings SET observed_at=104 WHERE if_index=1;",
        )
        .unwrap();
        store
            .enqueue_release_summaries(&notifier, 105, None)
            .unwrap();
        store
            .enqueue_release_summaries(&notifier, 106, None)
            .unwrap();
        assert_eq!(count(), 1);
        let payload: String = db
            .query_row("SELECT last_alert FROM notification_outbox", [], |r| {
                r.get(0)
            })
            .unwrap();
        let alert: AlertNotification = serde_json::from_str(&payload).unwrap();
        assert_eq!(
            alert.message,
            "Maintenance ended; continuing risks: Current availability: warning; Important port down: Gi1 (ifIndex 1)"
        );
        assert_eq!(store.history(10).unwrap().len(), 4);
    }
}
