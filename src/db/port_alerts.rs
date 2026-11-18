use crate::alert::{AlertEvent, AlertKind, AlertSeverity};
use crate::db::repository::Repository;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

pub(crate) fn migrate(db: &Connection) -> Result<(), AppError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS interface_alert_settings(
            device_id INTEGER NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
            if_index INTEGER NOT NULL CHECK(if_index>0),
            link_down_alert_enabled INTEGER NOT NULL DEFAULT 0 CHECK(link_down_alert_enabled IN(0,1)),
            topology_link INTEGER NOT NULL DEFAULT 0 CHECK(topology_link IN(0,1)),
            if_name TEXT NOT NULL DEFAULT '',
            if_admin_status INTEGER, if_oper_status INTEGER, observed_at INTEGER,
            active INTEGER NOT NULL DEFAULT 0 CHECK(active IN(0,1)), last_notified_at INTEGER,
            PRIMARY KEY(device_id,if_index));",
    )?;
    Ok(())
}

#[derive(Debug, Default, Serialize)]
pub struct PortAlertState {
    pub link_down_alert_enabled: bool,
    pub topology_link: bool,
    pub if_admin_status: Option<u32>,
    pub if_oper_status: Option<u32>,
    pub observed_at: Option<i64>,
    pub active: bool,
}

impl Repository {
    pub fn interface_alert(
        &self,
        device_id: i64,
        if_index: i32,
    ) -> Result<PortAlertState, AppError> {
        Ok(self.connection.query_row(
            "SELECT link_down_alert_enabled,topology_link,if_admin_status,if_oper_status,observed_at,active
             FROM interface_alert_settings WHERE device_id=?1 AND if_index=?2",
            params![device_id, if_index], |row| Ok(PortAlertState {
                link_down_alert_enabled: row.get(0)?, topology_link: row.get(1)?,
                if_admin_status: row.get(2)?, if_oper_status: row.get(3)?,
                observed_at: row.get(4)?, active: row.get(5)?,
            }),
        ).optional()?.unwrap_or_default())
    }

    pub fn set_interface_alert(
        &self,
        device_id: i64,
        if_index: i32,
        enabled: bool,
    ) -> Result<(), AppError> {
        let exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM devices WHERE id=?1) AND
             (EXISTS(SELECT 1 FROM interface_samples WHERE device_id=?1 AND if_index=?2) OR
              EXISTS(SELECT 1 FROM interface_alert_settings WHERE device_id=?1 AND if_index=?2))",
            params![device_id, if_index],
            |row| row.get(0),
        )?;
        if if_index <= 0 || !exists {
            return Err(AppError::Validation("interface not found".into()));
        }
        self.connection.execute(
            "INSERT INTO interface_alert_settings(device_id,if_index,link_down_alert_enabled)
             VALUES(?1,?2,?3) ON CONFLICT(device_id,if_index) DO UPDATE SET
             link_down_alert_enabled=excluded.link_down_alert_enabled,
             active=CASE WHEN excluded.link_down_alert_enabled=0 AND topology_link=0 THEN 0 ELSE active END,
             last_notified_at=CASE WHEN excluded.link_down_alert_enabled=0 AND topology_link=0 THEN NULL ELSE last_notified_at END",
            params![device_id, if_index, enabled],
        )?;
        Ok(())
    }

    /// A complete topology snapshot replaces only automatic selections, never manual pins.
    pub fn sync_topology_alert_ports(&self, ports: &[(i64, i32)]) -> Result<(), AppError> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "UPDATE interface_alert_settings SET topology_link=0 WHERE topology_link=1",
            [],
        )?;
        for &(device, index) in ports {
            if device <= 0 || index <= 0 {
                return Err(AppError::Validation(
                    "invalid topology interface identity".into(),
                ));
            }
            tx.execute(
                "INSERT INTO interface_alert_settings(device_id,if_index,topology_link)
                 SELECT id,?2,1 FROM devices WHERE id=?1
                 ON CONFLICT(device_id,if_index) DO UPDATE SET topology_link=1",
                params![device, index],
            )?;
        }
        tx.execute(
            "UPDATE interface_alert_settings SET active=0,last_notified_at=NULL
             WHERE link_down_alert_enabled=0 AND topology_link=0",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn observe_port_link(
        &self,
        device: &DeviceConfig,
        if_index: i32,
        if_name: &str,
        admin: Option<u32>,
        oper: Option<u32>,
        now: i64,
    ) -> Result<Option<AlertEvent>, AppError> {
        let id = device
            .id
            .ok_or_else(|| AppError::Validation("device has no id".into()))?;
        if if_index <= 0 {
            return Err(AppError::Validation("invalid interface index".into()));
        }
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO interface_alert_settings(device_id,if_index) VALUES(?1,?2)",
            params![id, if_index],
        )?;
        let (enabled, active, last, observed): (bool, bool, Option<i64>, Option<i64>) = tx
            .query_row(
            "SELECT link_down_alert_enabled OR topology_link,active,last_notified_at,observed_at
             FROM interface_alert_settings WHERE device_id=?1 AND if_index=?2",
            params![id, if_index],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        if observed.is_some_and(|at| at > now) {
            return Ok(None);
        }
        tx.execute(
            "UPDATE interface_alert_settings SET if_name=?3,if_admin_status=?4,if_oper_status=?5,observed_at=?6
             WHERE device_id=?1 AND if_index=?2",
            params![id,if_index,if_name,admin,oper,now],
        )?;
        let down = enabled && admin == Some(1) && oper == Some(2);
        let notify_down =
            down && (!active || last.is_none_or(|at| now.saturating_sub(at) >= 86400));
        let recovered = active && oper == Some(1);
        let event = if notify_down || recovered {
            let mut event = AlertEvent::new(
                AlertKind::Custom(
                    if recovered {
                        "RESOLVED_PORT_LINK_DOWN"
                    } else {
                        "PORT_LINK_DOWN"
                    }
                    .into(),
                ),
                if recovered {
                    AlertSeverity::Info
                } else {
                    AlertSeverity::Critical
                },
                device,
                format!(
                    "{}{} ({}) {} (if-{}): admin={:?}, oper={:?}",
                    if recovered { "[RESOLVED] " } else { "" },
                    device.name,
                    device.ip,
                    if_name,
                    if_index,
                    admin,
                    oper
                ),
            )
            .with_interface(if_name)
            .with_interface_id(i64::from(if_index));
            event.occurred_at = chrono::DateTime::from_timestamp(now, 0)
                .ok_or_else(|| AppError::Validation("invalid observation time".into()))?;
            tx.execute(
                "UPDATE interface_alert_settings SET active=?3,last_notified_at=?4 WHERE device_id=?1 AND if_index=?2",
                params![id,if_index,!recovered,if recovered {None} else {Some(now)}],
            )?;
            tx.execute(
                "INSERT INTO alert_events(device_id,alert_type,severity,details,created_at) VALUES(?1,?2,?3,?4,?5)",
                params![id,event.kind.as_str(),event.severity.to_string(),event.message,event.occurred_at.to_rfc3339()],
            )?;
            Some(event)
        } else {
            None
        };
        if let Some(event) = event.as_ref() {
            crate::notifications::journal_port_alert(&tx, event)?;
        }
        tx.commit()?;
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, Repository, DeviceConfig) {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::sqlite::initialize_database(&dir.path().join("data.db")).unwrap();
        db.execute("INSERT INTO devices(id,name,ip,community,device_type) VALUES(1,'switch','192.0.2.1','fixture','switch')", []).unwrap();
        let repo = Repository::new(db).unwrap();
        let device = DeviceConfig {
            id: Some(1),
            name: "switch".into(),
            ip: "192.0.2.1".into(),
            community: "fixture".into(),
            device_type: "switch".into(),
            status: "online".into(),
            last_seen_at: None,
        };
        repo.observe_port_link(&device, 1, "Gi1", Some(1), Some(1), 100)
            .unwrap();
        repo.set_interface_alert(1, 1, true).unwrap();
        (dir, repo, device)
    }

    #[test]
    fn important_port_admin_up_oper_down_emits_critical() {
        let (_dir, repo, device) = fixture();
        let event = repo
            .observe_port_link(&device, 1, "Gi1", Some(1), Some(2), 101)
            .unwrap()
            .unwrap();
        assert_eq!(event.kind.as_str(), "PORT_LINK_DOWN");
        assert_eq!(event.severity, AlertSeverity::Critical);
        assert!(repo.interface_alert(1, 1).unwrap().active);
    }

    #[test]
    fn shutdown_unknown_and_unselected_ports_never_emit() {
        let (_dir, repo, device) = fixture();
        for (admin, oper) in [
            (Some(2), Some(2)),
            (None, Some(2)),
            (Some(1), None),
            (Some(1), Some(7)),
        ] {
            assert!(
                repo.observe_port_link(&device, 1, "Gi1", admin, oper, 101)
                    .unwrap()
                    .is_none()
            );
        }
        assert!(
            repo.observe_port_link(&device, 2, "Gi2", Some(1), Some(2), 102)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn persistent_port_throttle_recovery_and_redown_are_independent() {
        let (dir, repo, device) = fixture();
        assert!(
            repo.observe_port_link(&device, 1, "Gi1", Some(1), Some(2), 101)
                .unwrap()
                .is_some()
        );
        drop(repo);
        let repo = Repository::new(
            crate::db::sqlite::initialize_database(&dir.path().join("data.db")).unwrap(),
        )
        .unwrap();
        assert!(
            repo.observe_port_link(&device, 1, "Gi1", Some(1), Some(2), 86500)
                .unwrap()
                .is_none()
        );
        assert!(
            repo.observe_port_link(&device, 1, "Gi1", Some(1), Some(2), 86501)
                .unwrap()
                .is_some()
        );
        let event = repo
            .observe_port_link(&device, 1, "Gi1", Some(1), Some(1), 86502)
            .unwrap()
            .unwrap();
        assert_eq!(event.kind.as_str(), "RESOLVED_PORT_LINK_DOWN");
        assert!(event.message.starts_with("[RESOLVED]"));
        assert!(
            repo.observe_port_link(&device, 1, "Gi1", Some(1), Some(2), 86503)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn topology_selection_preserves_manual_pins_and_rejects_stale_observations() {
        let (_dir, repo, device) = fixture();
        repo.sync_topology_alert_ports(&[(1, 2)]).unwrap();
        assert!(
            repo.observe_port_link(&device, 2, "Gi2", Some(1), Some(2), 200)
                .unwrap()
                .is_some()
        );
        assert!(
            repo.observe_port_link(&device, 2, "Gi2", Some(1), Some(1), 199)
                .unwrap()
                .is_none()
        );
        repo.sync_topology_alert_ports(&[]).unwrap();
        assert!(repo.interface_alert(1, 1).unwrap().link_down_alert_enabled);
        assert!(!repo.interface_alert(1, 2).unwrap().active);
        assert!(
            repo.observe_port_link(&device, 2, "Gi2", Some(1), Some(2), 201)
                .unwrap()
                .is_none()
        );
    }
}
