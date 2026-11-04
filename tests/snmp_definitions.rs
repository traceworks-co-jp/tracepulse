use serde_json::json;
use tracepulse::config::SnmpConfig;
use tracepulse::db::sqlite::initialize_database;
use tracepulse::snmp::management::{
    Definitions, SaveDefinitions, TemplateState, document, export_document, parse_document,
};
use tracepulse::snmp::template::TemplateRegistry;

fn baseline() -> TemplateState {
    TemplateState::new(TemplateRegistry::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("custom_templates"),
    ))
}

#[test]
fn every_profile_roundtrips_json_and_toml_without_losing_metrics() {
    let state = baseline();
    for template in state.registry.templates() {
        let value = document(template).unwrap();
        let parsed = parse_document(&value).unwrap();
        let text = export_document(&value).unwrap();
        let imported: tracepulse::snmp::VendorOidTemplate = toml::from_str(&text).unwrap();
        assert_eq!(document(&parsed).unwrap(), value);
        assert_eq!(document(&imported).unwrap(), value);
    }
}

#[test]
fn saves_are_atomic_versioned_persistent_and_do_not_change_old_snapshots() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("definitions.db");
    let connection = initialize_database(&path).unwrap();
    let state = baseline()
        .load(&connection, &SnmpConfig::default())
        .unwrap();
    let old = state.registry.clone();
    let mut definitions = Definitions::default();
    let mut generic = document(state.registry.select(None)).unwrap();
    generic["name"] = json!("共通RFC");
    generic["metrics"] = json!([{
        "key": "uptime", "oid": "1.3.6.1.2.1.1.3.0", "unit": "ticks",
        "label": "Uptime", "data_type": "counter"
    }]);
    definitions.profiles.insert("generic".into(), generic);
    definitions.common.cpu_oid = "1.3.6.1.4.1.65000.1.0".into();
    let saved = state
        .save(
            &connection,
            SaveDefinitions {
                generation: 0,
                definitions: definitions.clone(),
            },
        )
        .unwrap();
    assert_eq!(saved.generation, 1);
    assert_eq!(saved.registry.select(None).name, "共通RFC");
    assert_ne!(old.select(None).name, "共通RFC");
    assert!(
        state
            .save(
                &connection,
                SaveDefinitions {
                    generation: 0,
                    definitions: Definitions::default()
                }
            )
            .is_err()
    );
    let mut invalid = definitions.clone();
    invalid.profiles.get_mut("generic").unwrap()["cpu"]["candidates"] = json!(["symbolic::OID"]);
    assert!(
        saved
            .save(
                &connection,
                SaveDefinitions {
                    generation: 1,
                    definitions: invalid
                }
            )
            .is_err()
    );
    drop(connection);
    let reopened = initialize_database(&path).unwrap();
    let restored = baseline().load(&reopened, &SnmpConfig::default()).unwrap();
    assert_eq!(restored.generation, 1);
    assert_eq!(restored.registry.select(None).numeric_metrics.len(), 1);
    assert_eq!(
        restored.definitions.common.cpu_oid,
        definitions.common.cpu_oid
    );
    let reset = restored
        .save(
            &reopened,
            SaveDefinitions {
                generation: 1,
                definitions: Definitions::default(),
            },
        )
        .unwrap();
    assert_eq!(reset.generation, 2);
    assert_eq!(reset.registry.select(None).numeric_metrics.len(), 0);
    assert_eq!(reset.registry.select(None).name, old.select(None).name);
}

#[test]
fn migrates_legacy_common_oids_once_without_reintroducing_removed_settings() {
    let connection = initialize_database(std::path::Path::new(":memory:")).unwrap();
    let legacy = SnmpConfig {
        cpu_oid_override: "1.3.6.1.4.1.65000.1.0".into(),
        memory_oid_override: "1.3.6.1.4.1.65000.2.0".into(),
        hardware_oid_overrides: vec!["temperature|1.3.6.1.4.1.65000.3.0".into()],
        ..SnmpConfig::default()
    };
    let state = baseline().load(&connection, &legacy).unwrap();
    assert_eq!(state.definitions.common.cpu_oid, legacy.cpu_oid_override);
    assert_eq!(
        state.definitions.common.memory_oid,
        legacy.memory_oid_override
    );
    assert_eq!(
        state.definitions.common.hardware_oids,
        legacy.hardware_oid_overrides
    );
    let saved = state
        .save(
            &connection,
            SaveDefinitions {
                generation: 0,
                definitions: Definitions::default(),
            },
        )
        .unwrap();
    let restored = saved.load(&connection, &legacy).unwrap();
    assert!(restored.definitions.common.cpu_oid.is_empty());
    assert!(restored.definitions.common.hardware_oids.is_empty());
}

#[test]
fn user_profiles_use_longest_arc_matching_and_reset_to_byo() {
    let connection = initialize_database(std::path::Path::new(":memory:")).unwrap();
    let state = baseline()
        .load(&connection, &SnmpConfig::default())
        .unwrap();
    let mut definitions = Definitions::default();
    let mut doc = document(state.registry.select(None)).unwrap();
    doc["name"] = json!("Specific model");
    doc["sysObjectID_prefix"] = json!("1.3.6.1.4.1.65000.77");
    definitions
        .profiles
        .insert("1.3.6.1.4.1.65000.77".into(), doc);
    let saved = state
        .save(
            &connection,
            SaveDefinitions {
                generation: 0,
                definitions,
            },
        )
        .unwrap();
    assert_eq!(
        saved.registry.select(Some("1.3.6.1.4.1.65000.77.1")).name,
        "Specific model"
    );
    assert_ne!(
        saved.registry.select(Some("1.3.6.1.4.1.65000.770")).name,
        "Specific model"
    );
    assert_ne!(
        saved.registry.select(Some("1.3.6.1.4.1.9.1")).name,
        "Specific model"
    );
    let mut invalid = saved.definitions.clone();
    invalid.profiles.insert(
        "1.3.6.1.4.1.99".into(),
        document(saved.registry.select(None)).unwrap(),
    );
    assert!(
        saved
            .save(
                &connection,
                SaveDefinitions {
                    generation: 1,
                    definitions: invalid
                }
            )
            .is_err()
    );
}

#[test]
fn corrupt_persisted_definitions_fail_closed_and_failed_writes_preserve_generation() {
    let connection = initialize_database(std::path::Path::new(":memory:")).unwrap();
    let state = baseline()
        .load(&connection, &SnmpConfig::default())
        .unwrap();
    connection.execute_batch(
        "CREATE TRIGGER fail_definition_save BEFORE UPDATE ON snmp_definitions BEGIN SELECT RAISE(ABORT, 'write blocked'); END;"
    ).unwrap();
    assert!(
        state
            .save(
                &connection,
                SaveDefinitions {
                    generation: 0,
                    definitions: Definitions::default()
                }
            )
            .is_err()
    );
    assert_eq!(
        state
            .load(&connection, &SnmpConfig::default())
            .unwrap()
            .generation,
        0
    );
    connection
        .execute_batch(
            "DROP TRIGGER fail_definition_save; UPDATE snmp_definitions SET document = '{bad';",
        )
        .unwrap();
    assert!(state.load(&connection, &SnmpConfig::default()).is_err());
}
