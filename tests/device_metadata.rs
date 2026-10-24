use std::path::Path;
use tracepulse::{
    db::{repository::Repository, sqlite::initialize_database},
    device::{
        DeviceConfig,
        metadata::{BulkRename, DeviceMetadata, rename_plan},
    },
};

fn repository() -> Repository {
    let repository = Repository::new(initialize_database(Path::new(":memory:")).unwrap());
    for ip in ["192.0.2.10", "192.0.2.2", "2001:db8::10", "2001:db8::2"] {
        repository
            .save_device_config(&DeviceConfig::new(ip, "private"))
            .unwrap();
    }
    repository
}

fn request() -> BulkRename {
    serde_json::from_value(serde_json::json!({
        "device_ids": [1, 2, 3, 4], "prefix": "大阪支店SW-", "padding": 2,
        "suffix": "-本館", "sort_by": "ip_asc"
    }))
    .unwrap()
}

#[test]
fn numeric_ip_order_and_selection_order_have_exact_japanese_names() {
    let repository = repository();
    let mut request = request();
    let plan = rename_plan(&request, &repository.list_devices().unwrap()).unwrap();
    assert_eq!(
        plan.iter().map(|item| item.id).collect::<Vec<_>>(),
        [2, 1, 4, 3]
    );
    assert_eq!(plan[0].display_name, "大阪支店SW-01-本館");
    assert_eq!(plan[3].display_name, "大阪支店SW-04-本館");
    request.sort_by = tracepulse::device::metadata::RenameSort::SelectionOrder;
    request.device_ids = vec![3, 2, 1];
    request.start_number = 99;
    request.padding = 0;
    let plan = rename_plan(&request, &repository.list_devices().unwrap()).unwrap();
    assert_eq!(
        plan.iter().map(|item| item.id).collect::<Vec<_>>(),
        [3, 2, 1]
    );
    assert_eq!(plan[2].display_name, "大阪支店SW-101-本館");
    request.padding = 2;
    assert_eq!(
        rename_plan(&request, &repository.list_devices().unwrap()).unwrap()[2].display_name,
        "大阪支店SW-101-本館"
    );
}

#[test]
fn preview_is_read_only_and_apply_preserves_monitoring_fields_and_tags() {
    let repository = repository();
    repository
        .update_device_metadata(
            "192.0.2.10",
            DeviceMetadata {
                display_name: Some("旧名".into()),
                tags: vec!["東京HQ".into(), "Core".into()],
            },
        )
        .unwrap();
    let before = repository.list_devices().unwrap();
    let plan = repository.bulk_rename(&request(), true).unwrap();
    assert_eq!(
        repository.list_devices().unwrap()[0].display_name,
        before[0].display_name
    );
    assert_eq!(repository.bulk_rename(&request(), false).unwrap(), plan);
    for device in repository.list_devices().unwrap() {
        let previous = before.iter().find(|item| item.id == device.id).unwrap();
        assert_eq!(device.name, previous.name);
        assert_eq!(device.ip, previous.ip);
        assert_eq!(device.community, previous.community);
        assert_eq!(device.status, previous.status);
        assert_eq!(device.last_seen_at, previous.last_seen_at);
        assert_eq!(device.tags, previous.tags);
        assert_eq!(
            device.display_name.as_deref(),
            Some(
                plan.iter()
                    .find(|item| Some(item.id) == device.id)
                    .unwrap()
                    .display_name
                    .as_str()
            )
        );
    }
}

#[test]
fn invalid_bulk_operations_leave_every_device_unchanged() {
    let repository = repository();
    for invalid in [
        serde_json::json!({"device_ids":[1,999]}),
        serde_json::json!({"device_ids":[1,1]}),
        serde_json::json!({"device_ids":[]}),
        serde_json::json!({"device_ids":[0]}),
        serde_json::json!({"padding":11}),
        serde_json::json!({"start_number":4294967295u32}),
        serde_json::json!({"prefix":"bad\nname"}),
        serde_json::json!({"prefix":"あ".repeat(512)}),
        serde_json::json!({"device_ids":vec![1;1001]}),
    ] {
        let mut value = serde_json::json!({
            "device_ids":[1,2], "prefix":"SW-", "padding":2, "sort_by":"ip_asc"
        });
        for (key, value_override) in invalid.as_object().unwrap() {
            value[key] = value_override.clone();
        }
        let request: BulkRename = serde_json::from_value(value).unwrap();
        assert!(repository.bulk_rename(&request, false).is_err());
        assert!(
            repository
                .list_devices()
                .unwrap()
                .iter()
                .all(|device| device.display_name.is_none())
        );
    }
    let mut valid = request();
    valid.device_ids = vec![1];
    valid.start_number = u32::MAX;
    assert!(repository.bulk_rename(&valid, false).is_ok());
}

#[test]
fn metadata_normalizes_and_survives_both_discovery_upsert_paths() {
    let repository = repository();
    repository
        .update_device_metadata(
            "192.0.2.10",
            DeviceMetadata {
                display_name: Some("  東京本社フロアSW-01  ".into()),
                tags: vec![" 東京HQ ".into(), "Core".into(), "東京HQ".into()],
            },
        )
        .unwrap();
    let before = repository.find_device_by_ip("192.0.2.10").unwrap().unwrap();
    assert_eq!(before.label(), "東京本社フロアSW-01");
    assert_eq!(before.tags, ["東京HQ", "Core"]);
    repository
        .save_device_config(&DeviceConfig::new("192.0.2.10", "updated"))
        .unwrap();
    let mut discovered = before.clone();
    discovered.display_name = None;
    discovered.tags.clear();
    discovered.name = "new-sysName".into();
    repository.save_device(&discovered).unwrap();
    let after = repository.find_device_by_ip("192.0.2.10").unwrap().unwrap();
    assert_eq!(after.display_name, before.display_name);
    assert_eq!(after.tags, before.tags);
    assert_eq!(after.name, "new-sysName");
    repository
        .update_device_metadata(
            &after.ip,
            DeviceMetadata {
                display_name: Some("  ".into()),
                tags: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        repository
            .find_device_by_ip(&after.ip)
            .unwrap()
            .unwrap()
            .label(),
        "new-sysName"
    );
    let mut unnamed = after;
    unnamed.display_name = None;
    unnamed.name = " ".into();
    assert_eq!(unnamed.label(), unnamed.ip);
}

#[test]
fn metadata_validation_rejects_invalid_types_controls_and_limits() {
    for metadata in [
        DeviceMetadata {
            display_name: Some("a".repeat(513)),
            tags: vec![],
        },
        DeviceMetadata {
            display_name: Some("bad\nname".into()),
            tags: vec![],
        },
        DeviceMetadata {
            display_name: None,
            tags: vec![" ".into()],
        },
        DeviceMetadata {
            display_name: None,
            tags: vec!["a".repeat(65)],
        },
        DeviceMetadata {
            display_name: None,
            tags: vec!["tag".into(); 65],
        },
    ] {
        assert!(metadata.normalize().is_err());
    }
    for value in [
        r#"{"display_name":123,"tags":[]}"#,
        r#"{"tags":"Core"}"#,
        r#"{"tags":[],"unknown":1}"#,
    ] {
        assert!(serde_json::from_str::<DeviceMetadata>(value).is_err());
    }
    assert!(
        serde_json::from_str::<BulkRename>(
            r#"{"device_ids":[1],"prefix":"","padding":2,"sort_by":"invalid"}"#
        )
        .is_err()
    );
}

#[test]
fn legacy_database_migrates_idempotently_and_metadata_survives_reopen() {
    let path = std::env::temp_dir().join(format!(
        "tracepulse-metadata-migration-{}.db",
        std::process::id()
    ));
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE devices (
        id INTEGER PRIMARY KEY, name TEXT NOT NULL, ip TEXT NOT NULL UNIQUE,
        community TEXT NOT NULL, device_type TEXT NOT NULL, status TEXT NOT NULL,
        last_seen_at TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
        INSERT INTO devices (id,name,ip,community,device_type,status)
        VALUES (1,'original','192.0.2.1','public','switch','online');",
        )
        .unwrap();
    drop(connection);
    let repository = Repository::new(initialize_database(&path).unwrap());
    let device = repository.list_devices().unwrap().remove(0);
    assert_eq!(device.label(), "original");
    assert!(device.tags.is_empty());
    repository
        .update_device_metadata(
            &device.ip,
            DeviceMetadata {
                display_name: Some("大阪支店SW-01".into()),
                tags: vec!["3F".into()],
            },
        )
        .unwrap();
    drop(repository);
    let repository = Repository::new(initialize_database(&path).unwrap());
    let device = repository.list_devices().unwrap().remove(0);
    assert_eq!(device.label(), "大阪支店SW-01");
    assert_eq!(device.tags, ["3F"]);
    drop(repository);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn database_write_failure_rolls_back_prior_names_in_the_batch() {
    let connection = initialize_database(Path::new(":memory:")).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER reject_second_rename BEFORE UPDATE OF display_name ON devices
         WHEN NEW.id = 2 BEGIN SELECT RAISE(ABORT, 'simulated storage failure'); END;",
        )
        .unwrap();
    let repository = Repository::new(connection);
    for ip in ["192.0.2.1", "192.0.2.2"] {
        repository
            .save_device_config(&DeviceConfig::new(ip, "public"))
            .unwrap();
    }
    let mut request = request();
    request.device_ids = vec![1, 2];
    assert!(repository.bulk_rename(&request, false).is_err());
    assert!(
        repository
            .list_devices()
            .unwrap()
            .iter()
            .all(|device| device.display_name.is_none())
    );
}
