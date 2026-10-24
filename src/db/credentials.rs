use crate::{crypto::MasterKey, error::AppError};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

pub fn context(ip: &str) -> String {
    format!("TracePulse/devices/{ip}/community")
}

pub fn protect(connection: &mut Connection) -> Result<MasterKey, AppError> {
    let file = connection
        .path()
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from);
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='devices')",
        [],
        |row| row.get(0),
    )?;
    let encrypted: bool = if exists {
        connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM devices WHERE community LIKE 'tpenc:%')",
            [],
            |row| row.get(0),
        )?
    } else {
        false
    };
    let key_state: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='credential_storage')",
        [], |row| row.get(0),
    )?;
    let key = match file {
        Some(path) => MasterKey::load(
            path.parent().unwrap_or(std::path::Path::new(".")),
            !encrypted && !key_state,
        )?,
        None => MasterKey::generate()?,
    };
    if !exists {
        return Ok(key);
    }
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS credential_storage (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            cleanup_pending INTEGER NOT NULL CHECK(cleanup_pending IN (0, 1)),
            key_check TEXT NOT NULL
         );",
    )?;
    let secure_delete: i64 = connection.query_row("PRAGMA secure_delete", [], |row| row.get(0))?;
    connection.execute_batch("PRAGMA secure_delete=ON")?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key_check: Option<String> = transaction
        .query_row(
            "SELECT key_check FROM credential_storage WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match key_check {
        Some(envelope) => {
            let value = key.decrypt(&envelope, b"TracePulse/database/key-check")?;
            if value.as_slice() != b"TracePulse credential storage v1" {
                return Err(AppError::Config(
                    "invalid database key verification value".into(),
                ));
            }
        }
        None => {
            let envelope = key.encrypt(
                b"TracePulse credential storage v1",
                b"TracePulse/database/key-check",
            )?;
            transaction.execute(
                "INSERT INTO credential_storage (id, cleanup_pending, key_check) VALUES (1,0,?1)",
                [envelope],
            )?;
        }
    }
    let values = {
        let mut statement = transaction.prepare("SELECT ip, community FROM devices")?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut migrated = false;
    for (ip, value) in values {
        let context = context(&ip);
        if value.starts_with("tpenc:") {
            key.decrypt(&value, context.as_bytes())?;
        } else {
            let envelope = key.encrypt(value.as_bytes(), context.as_bytes())?;
            transaction.execute(
                "UPDATE devices SET community=?1 WHERE ip=?2",
                params![envelope, ip],
            )?;
            migrated = true;
        }
    }
    if migrated {
        transaction.execute(
            "UPDATE credential_storage SET cleanup_pending=1 WHERE id=1",
            [],
        )?;
    }
    transaction.commit()?;
    let pending = connection.query_row(
        "SELECT COALESCE((SELECT cleanup_pending FROM credential_storage WHERE id=1), 0)",
        [],
        |row| row.get::<_, bool>(0),
    )?;
    if pending {
        checkpoint(connection)?;
        connection.execute_batch("VACUUM")?;
        checkpoint(connection)?;
        connection.execute(
            "UPDATE credential_storage SET cleanup_pending=0 WHERE id=1",
            [],
        )?;
        checkpoint(connection)?;
    }
    connection.pragma_update(None, "secure_delete", secure_delete)?;
    Ok(key)
}

fn checkpoint(connection: &Connection) -> Result<(), AppError> {
    let busy: i64 =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))?;
    if busy != 0 {
        return Err(AppError::Database(
            "credential migration requires exclusive access; close other TracePulse instances and retry".into(),
        ));
    }
    Ok(())
}
