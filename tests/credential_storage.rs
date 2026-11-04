use std::{fs, path::Path};
use tracepulse::{
    config::AppConfig,
    crypto::PREFIX,
    db::{repository::Repository, sqlite::initialize_database},
    device::DeviceConfig,
};

fn repository(path: &Path) -> Repository {
    Repository::new(initialize_database(path).unwrap()).unwrap()
}

fn stored(path: &Path, ip: &str) -> String {
    rusqlite::Connection::open(path)
        .unwrap()
        .query_row("SELECT community FROM devices WHERE ip=?1", [ip], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn device_credentials_inherit_defaults_and_override_only_configured_fields() {
    use tracepulse::snmp::credentials::SnmpCredentials;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    let repo = repository(&path);
    let mut global = AppConfig::default().snmp;
    global.default_community = "global-zone-secret".into();
    global.credentials = SnmpCredentials {
        version: Some("3".into()),
        v3_username: Some("global-user".into()),
        v3_auth_password: Some("global-auth-secret".into()),
        v3_priv_password: Some("global-privacy-secret".into()),
        ..Default::default()
    };
    let device = DeviceConfig::new("192.0.2.51", "");
    let id = repo.save_device_config(&device).unwrap();
    let resolved = repo.resolved_snmp_config(&device.ip, &global, "").unwrap();
    assert_eq!(resolved.default_community, global.default_community);
    assert_eq!(
        resolved.credentials.v3_auth_password,
        global.credentials.v3_auth_password
    );
    repo.set_device_snmp_credentials(
        &[id],
        SnmpCredentials {
            v3_auth_password: Some("device-auth-secret".into()),
            community: Some("device-zone-secret".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let resolved = repo.resolved_snmp_config(&device.ip, &global, "").unwrap();
    assert_eq!(resolved.default_community, "device-zone-secret");
    assert_eq!(
        resolved.credentials.v3_auth_password.as_deref(),
        Some("device-auth-secret")
    );
    assert_eq!(
        resolved.credentials.v3_priv_password,
        global.credentials.v3_priv_password
    );
    global.default_community = "updated-global-secret".into();
    assert_eq!(
        repo.resolved_snmp_config(&device.ip, &global, "")
            .unwrap()
            .default_community,
        "device-zone-secret"
    );
    repo.set_device_snmp_credentials(&[id], SnmpCredentials::default())
        .unwrap();
    assert_eq!(
        repo.find_device_by_ip(&device.ip)
            .unwrap()
            .unwrap()
            .community,
        ""
    );
    assert_eq!(
        repo.resolved_snmp_config(&device.ip, &global, "")
            .unwrap()
            .default_community,
        "updated-global-secret"
    );
}

#[test]
fn device_v3_credentials_are_encrypted_and_bulk_updates_are_atomic() {
    use tracepulse::snmp::credentials::SnmpCredentials;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    let repo = repository(&path);
    let first = repo
        .save_device_config(&DeviceConfig::new("192.0.2.52", "legacy-secret"))
        .unwrap();
    let second = repo
        .save_device_config(&DeviceConfig::new("192.0.2.53", ""))
        .unwrap();
    let credentials = SnmpCredentials {
        version: Some("3".into()),
        community: Some("individual-community-marker".into()),
        v3_username: Some("individual-user-marker".into()),
        v3_auth_password: Some("individual-auth-marker".into()),
        v3_priv_password: Some("individual-privacy-marker".into()),
        ..Default::default()
    };
    repo.set_device_snmp_credentials(&[first, second], credentials.clone())
        .unwrap();
    assert_eq!(
        repo.device_snmp_credentials("192.0.2.52").unwrap(),
        credentials
    );
    assert!(!format!("{credentials:?}").contains("individual"));
    assert!(
        repo.set_device_snmp_credentials(&[first, 99999], Default::default())
            .is_err()
    );
    assert_eq!(
        repo.device_snmp_credentials("192.0.2.52").unwrap(),
        credentials
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    let envelopes: Vec<String> = connection
        .prepare("SELECT envelope FROM device_snmp_credentials")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(envelopes.iter().all(|value| value.starts_with(PREFIX)));
    assert_ne!(envelopes[0], envelopes[1]);
    drop(connection);
    drop(repo);
    for file in [
        path.clone(),
        path.with_file_name("data.db-wal"),
        path.with_file_name("data.db-journal"),
    ] {
        if file.exists() {
            let data = fs::read(file).unwrap();
            for secret in [
                "individual-community-marker",
                "individual-user-marker",
                "individual-auth-marker",
                "individual-privacy-marker",
            ] {
                assert!(
                    !data
                        .windows(secret.len())
                        .any(|window| window == secret.as_bytes())
                );
            }
        }
    }
    let repo = repository(&path);
    assert_eq!(
        repo.device_snmp_credentials("192.0.2.53").unwrap(),
        credentials
    );
    repo.remove_device_by_ip("192.0.2.52").unwrap();
    assert_eq!(
        repo.device_snmp_credentials("192.0.2.52").unwrap(),
        Default::default()
    );
}

#[test]
fn v3_config_migration_encrypts_secrets_and_preserves_other_sections() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    fs::write(&path, "[snmp]\ndefault_community='config-community-marker'\n[snmp.credentials]\nversion='3'\nv3_username='config-user-marker'\nv3_auth_password='config-auth-marker'\nv3_priv_password='config-privacy-marker'\n[extension]\nretained=true\n").unwrap();
    let config = AppConfig::load(&path).unwrap();
    let encrypted = fs::read_to_string(&path).unwrap();
    for secret in [
        "config-community-marker",
        "config-user-marker",
        "config-auth-marker",
        "config-privacy-marker",
    ] {
        assert!(!encrypted.contains(secret));
        assert!(!format!("{config:?}").contains(secret));
        assert!(!serde_json::to_string(&config).unwrap().contains(secret));
    }
    assert!(encrypted.contains("retained = true"));
    assert_eq!(
        AppConfig::load(&path).unwrap().snmp.credentials,
        config.snmp.credentials
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), encrypted);
    fs::remove_file(directory.path().join("secret.key")).unwrap();
    assert!(AppConfig::load(&path).is_err());
    assert!(!directory.path().join("secret.key").exists());
}

#[test]
fn invalid_or_masked_credentials_never_replace_saved_credentials() {
    use tracepulse::snmp::credentials::SnmpCredentials;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    let repo = repository(&path);
    let id = repo
        .save_device_config(&DeviceConfig::new("192.0.2.54", "original"))
        .unwrap();
    for credentials in [
        SnmpCredentials {
            community: Some("********".into()),
            ..Default::default()
        },
        SnmpCredentials {
            version: Some("invalid".into()),
            ..Default::default()
        },
    ] {
        assert!(
            repo.set_device_snmp_credentials(&[id], credentials)
                .is_err()
        );
    }
    assert_eq!(
        repo.find_device_by_ip("192.0.2.54")
            .unwrap()
            .unwrap()
            .community,
        "original"
    );
    let mut credentials = SnmpCredentials {
        version: Some("3".into()),
        ..Default::default()
    }
    .resolve(&AppConfig::default().snmp, "");
    assert!(credentials.validate_resolved().is_err());
    credentials.v3_username = Some("user".into());
    credentials.v3_auth_protocol = Some("none".into());
    credentials.v3_priv_protocol = Some("none".into());
    assert!(credentials.validate_resolved().is_ok());
    credentials.v3_priv_protocol = Some("aes128".into());
    assert!(credentials.validate_resolved().is_err());
}

#[test]
fn communities_are_envelopes_on_disk_but_plaintext_only_for_snmp_in_memory() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    let repo = repository(&path);
    let mut device = DeviceConfig::new("192.0.2.1", "credential-unique-private-value");
    repo.save_device_config(&device).unwrap();
    let envelope = stored(&path, &device.ip);
    assert!(envelope.starts_with(PREFIX));
    assert!(!envelope.contains(&device.community));
    let read = repo.find_device_by_ip(&device.ip).unwrap().unwrap();
    assert_eq!(read.community, device.community);
    assert!(
        !serde_json::to_string(&read)
            .unwrap()
            .contains(&device.community)
    );
    assert!(!format!("{read:?} {device:?}").contains(&device.community));
    device.community.clear();
    repo.save_device_config(&device).unwrap();
    assert_eq!(stored(&path, &device.ip), envelope);
    let mut model = read;
    model.community.clear();
    repo.save_device(&model).unwrap();
    assert_eq!(stored(&path, &device.ip), envelope);
    model.community = "replacement-community".into();
    repo.save_device(&model).unwrap();
    assert_ne!(stored(&path, &device.ip), envelope);
    drop(repo);
    assert_eq!(
        repository(&path).list_devices().unwrap()[0].community,
        model.community
    );
}

#[test]
fn migration_is_idempotent_and_scrubs_old_sqlite_pages_and_wal() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    let connection = initialize_database(&path).unwrap();
    let secret = "legacy-private-marker-23518279";
    connection.execute(
        "INSERT INTO devices (name,ip,community,device_type) VALUES ('old','192.0.2.1',?1,'switch')", [secret],
    ).unwrap();
    drop(connection);
    let repo = repository(&path);
    let envelope = stored(&path, "192.0.2.1");
    assert_eq!(repo.list_devices().unwrap()[0].community, secret);
    drop(repo);
    drop(repository(&path));
    assert_eq!(stored(&path, "192.0.2.1"), envelope);
    for path in [
        path.clone(),
        path.with_file_name("data.db-wal"),
        path.with_file_name("data.db-journal"),
    ] {
        if path.exists() {
            let bytes = fs::read(&path).unwrap();
            assert!(
                !bytes
                    .windows(secret.len())
                    .any(|window| window == secret.as_bytes()),
                "plaintext remains in {}",
                path.display()
            );
        }
    }
}

#[test]
fn missing_wrong_and_corrupt_keys_fail_closed_without_changing_ciphertext() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    let repo = repository(&path);
    repo.save_device_config(&DeviceConfig::new("192.0.2.1", "private"))
        .unwrap();
    drop(repo);
    let envelope = stored(&path, "192.0.2.1");
    let key_path = directory.path().join("secret.key");
    let key = fs::read(&key_path).unwrap();
    fs::remove_file(&key_path).unwrap();
    assert!(Repository::new(initialize_database(&path).unwrap()).is_err());
    assert!(!key_path.exists());
    fs::write(&key_path, [1; 32]).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    assert!(Repository::new(initialize_database(&path).unwrap()).is_err());
    assert_eq!(stored(&path, "192.0.2.1"), envelope);
    fs::write(&key_path, key).unwrap();
    assert_eq!(
        repository(&path).list_devices().unwrap()[0].community,
        "private"
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute("UPDATE devices SET community='tpenc:v1:invalid'", [])
        .unwrap();
    drop(connection);
    assert!(Repository::new(initialize_database(&path).unwrap()).is_err());
    assert_eq!(stored(&path, "192.0.2.1"), "tpenc:v1:invalid");
}

#[test]
fn plaintext_migration_rolls_back_when_any_envelope_is_invalid() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    let repo = repository(&path);
    drop(repo);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch(
        "INSERT INTO devices(name,ip,community,device_type) VALUES('old','192.0.2.1','original-secret','switch');
         INSERT INTO devices(name,ip,community,device_type) VALUES('bad','192.0.2.2','tpenc:v1:bad','switch');",
    ).unwrap();
    drop(connection);
    assert!(Repository::new(initialize_database(&path).unwrap()).is_err());
    assert_eq!(stored(&path, "192.0.2.1"), "original-secret");
}

#[test]
fn config_community_migrates_without_losing_other_sections_or_leaking_debug_values() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    fs::write(
        &path,
        "[snmp]\ndefault_community='private-config-secret'\n[extension]\nretained=true\n",
    )
    .unwrap();
    let config = AppConfig::load(&path).unwrap();
    assert_eq!(config.snmp.default_community, "private-config-secret");
    assert!(!format!("{config:?}").contains("private-config-secret"));
    assert!(
        !serde_json::to_string(&config)
            .unwrap()
            .contains("private-config-secret")
    );
    let encrypted = fs::read_to_string(&path).unwrap();
    assert!(!encrypted.contains("private-config-secret"));
    assert!(encrypted.contains(PREFIX));
    assert!(encrypted.contains("retained = true"));
    assert_eq!(
        AppConfig::load(&path).unwrap().snmp.default_community,
        "private-config-secret"
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), encrypted);
    fs::remove_file(directory.path().join("secret.key")).unwrap();
    assert!(AppConfig::load(&path).is_err());
    assert!(!directory.path().join("secret.key").exists());
    assert_eq!(fs::read_to_string(&path).unwrap(), encrypted);
}

#[test]
fn empty_database_still_remembers_its_key_and_refuses_silent_key_recreation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    drop(repository(&path));
    fs::remove_file(directory.path().join("secret.key")).unwrap();
    assert!(Repository::new(initialize_database(&path).unwrap()).is_err());
    assert!(!directory.path().join("secret.key").exists());
}

#[test]
fn repository_credentials_authenticate_an_actual_loopback_snmp_exchange() {
    use std::{net::UdpSocket, time::Duration};
    fn tlv(tag: u8, bytes: &[u8]) -> Vec<u8> {
        assert!(bytes.len() < 128);
        [vec![tag, bytes.len() as u8], bytes.to_vec()].concat()
    }
    fn take<'a>(bytes: &mut &'a [u8]) -> (u8, &'a [u8]) {
        let tag = bytes[0];
        let size = bytes[1] as usize;
        assert!(size < 128);
        let value = &bytes[2..2 + size];
        *bytes = &bytes[2 + size..];
        (tag, value)
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    let repo = repository(&path);
    let ip = "127.0.0.241";
    repo.save_device_config(&DeviceConfig::new(ip, "")).unwrap();
    let socket = UdpSocket::bind((ip, 0)).unwrap();
    let port = socket.local_addr().unwrap().port();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let worker = std::thread::spawn(move || {
        for expected in [
            b"loopback-global".as_slice(),
            b"loopback-private".as_slice(),
        ] {
            let mut buffer = [0; 1500];
            let (length, peer) = socket.recv_from(&mut buffer).unwrap();
            let mut packet = &buffer[..length];
            let (_, mut message) = take(&mut packet);
            let (_, version) = take(&mut message);
            let (_, community) = take(&mut message);
            assert_eq!(community, expected);
            let (_, mut pdu) = take(&mut message);
            let (_, request_id) = take(&mut pdu);
            take(&mut pdu);
            take(&mut pdu);
            let (_, mut bindings) = take(&mut pdu);
            let (_, mut binding) = take(&mut bindings);
            let (_, oid) = take(&mut binding);
            let binding = tlv(0x30, &[tlv(6, oid), tlv(4, b"loopback-switch")].concat());
            let pdu = tlv(
                0xa2,
                &[
                    tlv(2, request_id),
                    tlv(2, &[0]),
                    tlv(2, &[0]),
                    tlv(0x30, &binding),
                ]
                .concat(),
            );
            let response = tlv(0x30, &[tlv(2, version), tlv(4, community), pdu].concat());
            socket.send_to(&response, peer).unwrap();
        }
    });
    let mut global = AppConfig::default().snmp;
    global.default_community = "loopback-global".into();
    for community in [None, Some("loopback-private")] {
        let device = repo.find_device_by_ip(ip).unwrap().unwrap();
        if let Some(community) = community {
            repo.set_device_snmp_credentials(
                &[device.id.unwrap()],
                tracepulse::snmp::credentials::SnmpCredentials {
                    community: Some(community.into()),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let config = repo
            .resolved_snmp_config(ip, &global, &device.community)
            .unwrap();
        let client = tracepulse::snmp::SnmpClient::with_snmp_config(
            config.default_community.clone(),
            config,
        )
        .with_destination_port(port);
        assert_eq!(
            client
                .probe_device_once(&DeviceConfig::new(ip, ""), Duration::from_secs(3))
                .unwrap(),
            "loopback-switch"
        );
    }
    worker.join().unwrap();
    assert!(stored(&path, ip).starts_with(PREFIX));
}

#[test]
fn startup_checks_encrypted_config_before_creating_a_new_database_key() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(
        &config,
        "[snmp]\ndefault_community='startup-secret-value'\n",
    )
    .unwrap();
    AppConfig::load(&config).unwrap();
    let encrypted = fs::read(&config).unwrap();
    fs::remove_file(directory.path().join("secret.key")).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tracepulse"))
        .args(["--web", "--port=0"])
        .env("TRACEPULSE_DATA_DIR", directory.path())
        .env_remove("TRACEPULSE_MASTER_KEY")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let logs = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(logs.contains("master key is missing"));
    assert!(!logs.contains("startup-secret-value"));
    assert!(!directory.path().join("secret.key").exists());
    assert_eq!(fs::read(config).unwrap(), encrypted);
}

#[test]
fn interrupted_plaintext_cleanup_is_resumed_without_reencrypting_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.db");
    let reader = initialize_database(&path).unwrap();
    reader.execute(
        "INSERT INTO devices(name,ip,community,device_type) VALUES ('old','192.0.2.1','legacy-secret','switch')", [],
    ).unwrap();
    reader
        .execute_batch("BEGIN; SELECT * FROM devices;")
        .unwrap();
    assert!(Repository::new(initialize_database(&path).unwrap()).is_err());
    reader.execute_batch("ROLLBACK").unwrap();
    drop(reader);
    let envelope = stored(&path, "192.0.2.1");
    assert!(envelope.starts_with(PREFIX));
    let repo = repository(&path);
    assert_eq!(stored(&path, "192.0.2.1"), envelope);
    assert_eq!(repo.list_devices().unwrap()[0].community, "legacy-secret");
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT cleanup_pending FROM credential_storage WHERE id=1",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
