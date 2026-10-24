use std::path::Path;
use tracepulse::{
    db::{repository::Repository, sqlite::initialize_database},
    device::{DeviceConfig, MAX_MONITORED_NODES, registry::DeviceRegistry},
};

#[test]
fn repository_enforces_instance_ceiling_on_both_registration_paths() {
    let repo = Repository::new(initialize_database(Path::new(":memory:")).unwrap()).unwrap();
    for index in 1..=MAX_MONITORED_NODES {
        let config = DeviceConfig::new(format!("192.0.2.{index}"), "test");
        if index % 2 == 0 {
            repo.save_device_config(&config).unwrap();
        } else {
            let device = tracepulse::db::models::Device {
                id: None,
                name: "test".into(),
                display_name: None,
                tags: vec![],
                ip: config.ip,
                community: "test".into(),
                device_type: "unknown".into(),
                status: "unknown".into(),
                last_seen_at: None,
                created_at: None,
                updated_at: None,
            };
            repo.save_device(&device).unwrap();
        }
    }
    let mut existing = repo.find_device_by_ip("192.0.2.1").unwrap().unwrap();
    existing.display_name = Some("kept".into());
    repo.save_device(&existing).unwrap();
    repo.save_device_config(&DeviceConfig::new("192.0.2.1", "replacement"))
        .unwrap();
    let mut excess = existing.clone();
    excess.ip = "192.0.2.201".into();
    assert!(
        repo.save_device(&excess)
            .unwrap_err()
            .to_string()
            .contains("200 nodes")
    );
    assert!(
        repo.save_device_config(&DeviceConfig::new("192.0.2.201", "test"))
            .is_err()
    );
    assert_eq!(repo.list_devices().unwrap().len(), 200);
    repo.remove_device_by_ip("192.0.2.1").unwrap();
    repo.save_device(&excess).unwrap();
    assert_eq!(repo.list_devices().unwrap().len(), 200);
}

#[test]
fn registry_accepts_200_nodes_and_reuses_removed_capacity() {
    let mut registry = DeviceRegistry::new();
    for index in 1..=MAX_MONITORED_NODES {
        registry
            .add_device(DeviceConfig::new(format!("192.0.2.{index}"), "test"))
            .unwrap();
    }
    assert!(
        registry
            .add_device(DeviceConfig::new("192.0.2.201", "test"))
            .is_err()
    );
    registry.update_device("192.0.2.1", "online").unwrap();
    registry.remove_device("192.0.2.1").unwrap();
    registry
        .add_device(DeviceConfig::new("192.0.2.201", "test"))
        .unwrap();
    assert_eq!(registry.list_devices().len(), 200);
}
