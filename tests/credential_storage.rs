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
    repo.save_device_config(&DeviceConfig::new("127.0.0.1", "loopback-private"))
        .unwrap();
    let secret = repo
        .find_device_by_ip("127.0.0.1")
        .unwrap()
        .unwrap()
        .community;
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let endpoint = socket.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let mut buffer = [0; 1500];
        let (length, peer) = socket.recv_from(&mut buffer).unwrap();
        let mut packet = &buffer[..length];
        let (_, mut message) = take(&mut packet);
        let (_, version) = take(&mut message);
        let (_, community) = take(&mut message);
        assert_eq!(community, b"loopback-private");
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
    });
    let mut session =
        snmp::SyncSession::new(endpoint, secret.as_bytes(), Some(Duration::from_secs(3)), 1)
            .unwrap();
    let mut response = session.get(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap();
    let (_, value) = response.varbinds.next().unwrap();
    assert!(matches!(
        value,
        snmp::Value::OctetString(b"loopback-switch")
    ));
    worker.join().unwrap();
    assert!(stored(&path, "127.0.0.1").starts_with(PREFIX));
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
