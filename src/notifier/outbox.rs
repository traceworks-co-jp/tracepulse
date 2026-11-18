//! Durable flap batches and independent delivery state for each channel.
use super::{AlertBatch, AlertNotification, NotificationChannel, NotifierError};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Outbox(Arc<Mutex<Connection>>);

pub(super) struct Work {
    id: i64,
    lease: i64,
    pub channel: NotificationChannel,
    pub batch: AlertBatch,
    pub attempts: u32,
}

fn error(err: impl std::fmt::Display) -> NotifierError {
    NotifierError::Io(format!("notification outbox: {err}"))
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

impl Outbox {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, NotifierError> {
        let db = Connection::open(path).map_err(error)?;
        db.busy_timeout(Duration::from_secs(3)).map_err(error)?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS notification_intake(
                 event_id TEXT PRIMARY KEY, created_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS notification_outbox(
                 id INTEGER PRIMARY KEY,
                 channel TEXT NOT NULL, flap_key TEXT NOT NULL,
                 first_alert TEXT NOT NULL, last_alert TEXT NOT NULL,
                 count INTEGER NOT NULL, due_at INTEGER NOT NULL, attempts INTEGER NOT NULL DEFAULT 0,
                 lease_until INTEGER NOT NULL DEFAULT 0, buffered INTEGER NOT NULL,
                 status TEXT NOT NULL DEFAULT 'pending');
             CREATE INDEX IF NOT EXISTS notification_outbox_ready
                 ON notification_outbox(status,due_at,lease_until);
             CREATE INDEX IF NOT EXISTS notification_outbox_flap
                 ON notification_outbox(status,channel,flap_key,buffered);",
        ).map_err(error)?;
        Ok(Self(Arc::new(Mutex::new(db))))
    }

    fn db(&self) -> Result<MutexGuard<'_, Connection>, NotifierError> {
        self.0.lock().map_err(error)
    }

    pub(super) fn enqueue(
        &self,
        alert: &AlertNotification,
        channels: &[NotificationChannel],
        window: Duration,
    ) -> Result<(), NotifierError> {
        let mut db = self.db()?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        enqueue_on(&tx, alert, channels, window)?;
        tx.commit().map_err(error)
    }

    pub(super) fn ready(&self, flush_buffered: bool) -> Result<Vec<i64>, NotifierError> {
        let db = self.db()?;
        let now = now();
        let retention = now - 7 * 86400 * 1000;
        // Keep terminal receipts for diagnostics and bounded replay protection.
        db.execute(
            "DELETE FROM notification_outbox WHERE status<>'pending' AND due_at<?1",
            [retention],
        )
        .map_err(error)?;
        db.execute(
            "DELETE FROM notification_intake WHERE created_at<?1",
            [retention],
        )
        .map_err(error)?;
        if flush_buffered {
            db.execute(
                "UPDATE notification_outbox SET due_at=MIN(due_at,?1),buffered=0 WHERE buffered=1",
                [now],
            )
            .map_err(error)?;
        }
        let mut query = db.prepare("SELECT id FROM notification_outbox WHERE status='pending' AND due_at<=?1 AND lease_until<=?1 ORDER BY due_at,id LIMIT 32").map_err(error)?;
        query
            .query_map([now], |row| row.get(0))
            .map_err(error)?
            .collect::<Result<_, _>>()
            .map_err(error)
    }

    pub(super) fn claim(&self, id: i64) -> Result<Option<Work>, NotifierError> {
        let db = self.db()?;
        let now = now();
        let lease = now + 120_000;
        let row: Option<(String, String, String, usize, u32)> = db
            .query_row(
                "UPDATE notification_outbox SET lease_until=?2,buffered=0
                 WHERE id=?1 AND status='pending' AND due_at<=?3 AND lease_until<=?3
                 RETURNING channel,first_alert,last_alert,count,attempts",
                params![id, lease, now],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()
            .map_err(error)?;
        row.map(|(channel, first, last, count, attempts)| {
            let first_alert: AlertNotification = serde_json::from_str(&first).map_err(error)?;
            let last_alert = serde_json::from_str(&last).map_err(error)?;
            let seen = Instant::now();
            Ok(Work {
                id,
                lease,
                channel: serde_json::from_str(&channel).map_err(error)?,
                attempts,
                batch: AlertBatch {
                    key: first_alert.flap_key(),
                    first_alert,
                    last_alert,
                    count,
                    first_seen: seen,
                    last_seen: seen,
                },
            })
        })
        .transpose()
    }

    pub(super) fn finish(&self, work: &Work, status: &str) -> Result<(), NotifierError> {
        self.db()?.execute("UPDATE notification_outbox SET status=?3,lease_until=0,due_at=?4 WHERE id=?1 AND lease_until=?2",
            params![work.id, work.lease, status, now()]).map_err(error)?;
        Ok(())
    }

    pub(super) fn defer(&self, work: &Work, attempted: bool) -> Result<(), NotifierError> {
        let delay = if attempted {
            (300_i64 << work.attempts.min(5)).min(5000)
        } else {
            5000
        };
        self.db()?.execute("UPDATE notification_outbox SET attempts=attempts+?3,lease_until=0,due_at=?4 WHERE id=?1 AND lease_until=?2",
            params![work.id, work.lease, attempted, now()+delay]).map_err(error)?;
        Ok(())
    }
}

/// Caller owns the transaction, so release bookkeeping and enqueue commit together.
pub(super) fn enqueue_on(
    db: &Connection,
    alert: &AlertNotification,
    channels: &[NotificationChannel],
    window: Duration,
) -> Result<(), NotifierError> {
    if channels.is_empty() {
        return Ok(());
    }
    let now = now();
    if db
        .execute(
            "INSERT OR IGNORE INTO notification_intake(event_id,created_at) VALUES(?1,?2)",
            params![alert.id, now],
        )
        .map_err(error)?
        == 0
    {
        return Ok(());
    }
    let buffered = !alert.is_immediate() && !window.is_zero();
    let due = now.saturating_add(i64::try_from(window.as_millis()).unwrap_or(i64::MAX));
    let key = serde_json::to_string(&(
        alert.device_id,
        alert.interface_id,
        &alert.alert_kind,
        alert.severity,
    ))
    .map_err(error)?;
    let payload = serde_json::to_string(alert).map_err(error)?;
    for channel in channels {
        let channel = serde_json::to_string(channel).map_err(error)?;
        if buffered && db.execute(
            "UPDATE notification_outbox SET last_alert=?3,count=count+1 WHERE id=(
              SELECT id FROM notification_outbox WHERE status='pending' AND channel=?1 AND flap_key=?2 AND buffered=1 AND due_at>=?4 ORDER BY id DESC LIMIT 1)",
            params![channel,key,payload,now],
        ).map_err(error)? != 0 { continue; }
        db.execute(
            "INSERT INTO notification_outbox(channel,flap_key,first_alert,last_alert,count,due_at,buffered)
                VALUES(?1,?2,?3,?3,1,?4,?5)",
            params![channel,key,payload,if buffered {due} else {now},buffered],
        ).map_err(error)?;
    }
    Ok(())
}
