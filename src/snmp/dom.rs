use super::{
    client::{SnmpClient, parse_oid_string, query_template_column},
    template::DomTableTemplate,
    walk::SnmpValue,
};
use crate::{device::types::DeviceConfig, error::AppError};
use snmp::SyncSession;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OpticalPower {
    pub rx_power_dbm: Option<f64>,
    pub tx_power_dbm: Option<f64>,
}

type Column = BTreeMap<Vec<u32>, SnmpValue>;

impl SnmpClient {
    pub fn query_dom(&self, device: &DeviceConfig) -> Result<HashMap<u32, OpticalPower>, AppError> {
        let mut session = self.open_session(device)?;
        let template = self.interface_dom_template(&mut session)?;
        match template {
            Some(table) => collect(&mut session, &table),
            None => Ok(HashMap::new()),
        }
    }
}

pub(super) fn collect(
    session: &mut SyncSession,
    table: &DomTableTemplate,
) -> Result<HashMap<u32, OpticalPower>, AppError> {
    let mut columns = HashMap::<String, Column>::new();
    let mut oids: Vec<&str> = table
        .metrics
        .iter()
        .map(|metric| metric.oid.as_str())
        .collect();
    if let Some(oid) = &table.index_oid {
        oids.push(oid);
    }
    if let Some(entity) = &table.entity {
        oids.extend([
            entity.name_oid.as_str(),
            &entity.contained_in_oid,
            &entity.alias_oid,
            &entity.type_oid,
            &entity.scale_oid,
            &entity.precision_oid,
            &entity.status_oid,
        ]);
    }
    for oid in oids {
        if columns.contains_key(oid) {
            continue;
        }
        let prefix = parse_oid_string(oid)
            .ok_or_else(|| AppError::Validation(format!("invalid DOM OID '{oid}'")))?;
        columns.insert(oid.to_string(), query_template_column(session, &prefix)?);
    }
    Ok(join(table, &columns))
}

fn number(value: &SnmpValue) -> Option<f64> {
    match value {
        SnmpValue::Integer(value) => Some(*value as f64),
        value => value.as_u64().map(|value| value as f64),
    }
}

fn integer(column: &Column, index: &[u32]) -> Option<i64> {
    match column.get(index)? {
        SnmpValue::Integer(value) => Some(*value),
        value => value.as_u64().and_then(|value| i64::try_from(value).ok()),
    }
}

fn entity_port(entity: u32, aliases: &HashMap<u32, Option<u32>>, parents: &Column) -> Option<u32> {
    let mut current = entity;
    let mut visited = HashSet::new();
    for _ in 0..32 {
        if !visited.insert(current) {
            return None;
        }
        if let Some(port) = aliases.get(&current) {
            return *port;
        }
        current = u32::try_from(integer(parents, &[current])?).ok()?;
        if current == 0 {
            return None;
        }
    }
    None
}

fn join(table: &DomTableTemplate, columns: &HashMap<String, Column>) -> HashMap<u32, OpticalPower> {
    let mut aliases = HashMap::<u32, Option<u32>>::new();
    if let Some(entity) = &table.entity {
        for (index, value) in &columns[&entity.alias_oid] {
            let ([physical, _logical], SnmpValue::ObjectIdentifier(oid)) =
                (index.as_slice(), value)
            else {
                continue;
            };
            let Some([port]) = oid.strip_prefix(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1][..]) else {
                continue;
            };
            if *port == 0 || *port > i32::MAX as u32 {
                continue;
            }
            aliases
                .entry(*physical)
                .and_modify(|previous| {
                    if *previous != Some(*port) {
                        *previous = None;
                    }
                })
                .or_insert(Some(*port));
        }
    }
    let mut readings = HashMap::<(u32, &str), Vec<f64>>::new();
    for metric in &table.metrics {
        for (index, raw) in &columns[&metric.oid] {
            let Some(raw) = number(raw).filter(|value| !metric.invalid_values.contains(value))
            else {
                continue;
            };
            let (port, value) = if let Some(entity) = &table.entity {
                let [row] = index.as_slice() else {
                    continue;
                };
                let Some(name) = columns[&entity.name_oid]
                    .get(index)
                    .and_then(SnmpValue::as_string)
                else {
                    continue;
                };
                let name = name.to_ascii_lowercase();
                // Lane readings must not be silently collapsed into one port value.
                if name.contains("lane")
                    || name.contains("channel")
                    || !metric
                        .match_names
                        .iter()
                        .any(|pattern| name.contains(&pattern.to_ascii_lowercase()))
                    || integer(&columns[&entity.status_oid], index) != Some(1)
                    || integer(&columns[&entity.type_oid], index) != Some(6)
                {
                    continue;
                }
                if table
                    .metrics
                    .iter()
                    .filter(|candidate| {
                        candidate
                            .match_names
                            .iter()
                            .any(|pattern| name.contains(&pattern.to_ascii_lowercase()))
                    })
                    .count()
                    != 1
                {
                    continue;
                }
                let Some(port) = entity_port(*row, &aliases, &columns[&entity.contained_in_oid])
                else {
                    continue;
                };
                let Some(scale) = integer(&columns[&entity.scale_oid], index) else {
                    continue;
                };
                let exponent = match scale {
                    1..=13 => (scale as i32 - 9) * 3,
                    14 => 18,
                    15 => 15,
                    16 => 21,
                    17 => 24,
                    _ => continue,
                };
                let Some(precision) = integer(&columns[&entity.precision_oid], index)
                    .filter(|value| (-8..=9).contains(value))
                else {
                    continue;
                };
                // Non-positive precision describes accuracy, not fractional digits.
                let watts = raw * metric.scale * 10f64.powi(exponent - precision.max(0) as i32);
                if !watts.is_finite() || watts <= 0.0 {
                    continue;
                }
                (port, 10.0 * (watts * 1000.0).log10())
            } else {
                let port = if let Some(oid) = &table.index_oid {
                    integer(&columns[oid], index).and_then(|value| u32::try_from(value).ok())
                } else {
                    match index.as_slice() {
                        [row] => Some(*row),
                        _ => None,
                    }
                };
                let Some(port) = port.filter(|port| *port > 0 && *port <= i32::MAX as u32) else {
                    continue;
                };
                (port, raw * metric.scale)
            };
            if value.is_finite() {
                readings.entry((port, &metric.key)).or_default().push(value);
            }
        }
    }
    let mut result = HashMap::<u32, OpticalPower>::new();
    for ((port, key), values) in readings {
        let value = if values.len() == 1 {
            Some(values[0])
        } else {
            tracing::warn!(
                if_index = port,
                metric = key,
                "Ambiguous DOM rows; leaving port power unmeasured"
            );
            None
        };
        let power = result.entry(port).or_default();
        match key {
            "rx_power" => power.rx_power_dbm = value,
            "tx_power" => power.tx_power_dbm = value,
            _ => unreachable!("validated DOM key"),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snmp::template::VendorOidTemplate;

    fn juniper() -> DomTableTemplate {
        toml::from_str::<VendorOidTemplate>(include_str!("../../templates/juniper.toml"))
            .unwrap()
            .tables
            .dom
            .unwrap()
    }

    #[test]
    fn direct_rows_join_by_full_index_and_preserve_negative_zero_and_missing_values() {
        let mut table = juniper();
        table.metrics[0].invalid_values = vec![-4000.0];
        let columns = HashMap::from([
            (
                table.metrics[0].oid.clone(),
                BTreeMap::from([
                    (vec![7], SnmpValue::Integer(-350)),
                    (vec![8], SnmpValue::Integer(-4000)),
                    (vec![9], SnmpValue::Null),
                    (vec![10, 1], SnmpValue::Integer(-100)),
                ]),
            ),
            (
                table.metrics[1].oid.clone(),
                BTreeMap::from([
                    (vec![7], SnmpValue::Integer(-420)),
                    (vec![8], SnmpValue::Integer(0)),
                ]),
            ),
        ]);
        let powers = join(&table, &columns);
        assert_eq!(
            powers[&7],
            OpticalPower {
                rx_power_dbm: Some(-3.5),
                tx_power_dbm: Some(-4.2)
            }
        );
        assert_eq!(
            powers[&8],
            OpticalPower {
                rx_power_dbm: None,
                tx_power_dbm: Some(0.0)
            }
        );
        assert!(!powers.contains_key(&9));
        assert!(!powers.contains_key(&10));
        table.metrics[0].invalid_values.clear();
        assert_eq!(join(&table, &columns)[&8].rx_power_dbm, Some(-40.0));
        table.index_oid = Some("1.3.6.1.2.1.2.2.1.1".into());
        let mut columns = columns;
        columns.insert(
            table.index_oid.clone().unwrap(),
            BTreeMap::from([
                (vec![7], SnmpValue::Integer(123)),
                (vec![10, 1], SnmpValue::Integer(124)),
            ]),
        );
        let powers = join(&table, &columns);
        assert_eq!(powers.len(), 2);
        assert_eq!(powers[&123].rx_power_dbm, Some(-3.5));
        assert_eq!(powers[&124].rx_power_dbm, Some(-1.0));
    }

    fn cisco_columns(table: &DomTableTemplate) -> HashMap<String, Column> {
        let entity = table.entity.as_ref().unwrap();
        let mut columns = HashMap::new();
        for oid in [
            &entity.name_oid,
            &entity.contained_in_oid,
            &entity.alias_oid,
            &entity.type_oid,
            &entity.scale_oid,
            &entity.precision_oid,
            &entity.status_oid,
            &table.metrics[0].oid,
        ] {
            columns.insert(oid.clone(), BTreeMap::new());
        }
        for (row, name, value) in [
            (100, "Receive Power Sensor", 500),
            (101, "Transmit Power Sensor", 1000),
        ] {
            columns
                .get_mut(&entity.name_oid)
                .unwrap()
                .insert(vec![row], SnmpValue::OctetString(name.as_bytes().to_vec()));
            for (oid, value) in [
                (&entity.contained_in_oid, 50),
                (&entity.type_oid, 6),
                (&entity.scale_oid, 8),
                (&entity.precision_oid, 3),
                (&entity.status_oid, 1),
                (&table.metrics[0].oid, value),
            ] {
                columns
                    .get_mut(oid)
                    .unwrap()
                    .insert(vec![row], SnmpValue::Integer(value));
            }
        }
        // Sensor -> transceiver -> physical port -> IF-MIB ifIndex.
        columns
            .get_mut(&entity.contained_in_oid)
            .unwrap()
            .insert(vec![50], SnmpValue::Integer(40));
        columns.get_mut(&entity.alias_oid).unwrap().insert(
            vec![40, 0],
            SnmpValue::ObjectIdentifier(vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 1, 7]),
        );
        columns
    }

    #[test]
    fn cisco_joins_entity_hierarchy_and_converts_scaled_watts_to_dbm() {
        let table = toml::from_str::<VendorOidTemplate>(include_str!("../../templates/cisco.toml"))
            .unwrap()
            .tables
            .dom
            .unwrap();
        let columns = cisco_columns(&table);
        let powers = join(&table, &columns);
        assert!((powers[&7].rx_power_dbm.unwrap() - (-3.0102999566)).abs() < 1e-9);
        assert_eq!(powers[&7].tx_power_dbm, Some(0.0));
        assert!(!powers.contains_key(&100));
    }

    #[test]
    fn cisco_rejects_unknown_units_failed_sensors_lanes_bad_scale_and_missing_metadata() {
        let table = toml::from_str::<VendorOidTemplate>(include_str!("../../templates/cisco.toml"))
            .unwrap()
            .tables
            .dom
            .unwrap();
        let entity = table.entity.as_ref().unwrap();
        for (oid, value) in [
            (&entity.type_oid, SnmpValue::Integer(1)),
            (&entity.status_oid, SnmpValue::Integer(2)),
            (&entity.scale_oid, SnmpValue::Integer(99)),
            (&entity.precision_oid, SnmpValue::Integer(10)),
            (&table.metrics[0].oid, SnmpValue::Integer(0)),
            (&table.metrics[0].oid, SnmpValue::Integer(-1000000000)),
            (
                &entity.name_oid,
                SnmpValue::OctetString(b"Receive Power Lane 1".to_vec()),
            ),
        ] {
            let mut columns = cisco_columns(&table);
            columns.get_mut(oid).unwrap().insert(vec![100], value);
            let powers = join(&table, &columns);
            assert_eq!(powers[&7].rx_power_dbm, None, "{oid}");
            assert_eq!(powers[&7].tx_power_dbm, Some(0.0));
        }
        let mut columns = cisco_columns(&table);
        columns
            .get_mut(&entity.precision_oid)
            .unwrap()
            .remove(&vec![100]);
        assert_eq!(join(&table, &columns)[&7].rx_power_dbm, None);
    }

    #[test]
    fn cisco_rejects_ambiguous_aliases_duplicate_sensors_and_parent_cycles() {
        let table = toml::from_str::<VendorOidTemplate>(include_str!("../../templates/cisco.toml"))
            .unwrap()
            .tables
            .dom
            .unwrap();
        let entity = table.entity.as_ref().unwrap();
        let mut columns = cisco_columns(&table);
        columns.get_mut(&entity.alias_oid).unwrap().insert(
            vec![40, 1],
            SnmpValue::ObjectIdentifier(vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 1, 8]),
        );
        assert!(join(&table, &columns).is_empty());
        let mut columns = cisco_columns(&table);
        for column in columns.values_mut() {
            if let Some(value) = column.get(&vec![100]).cloned() {
                column.insert(vec![102], value);
            }
        }
        assert_eq!(join(&table, &columns)[&7].rx_power_dbm, None);
        let mut columns = cisco_columns(&table);
        columns
            .get_mut(&entity.contained_in_oid)
            .unwrap()
            .insert(vec![50], SnmpValue::Integer(100));
        assert!(join(&table, &columns).is_empty());
    }

    #[test]
    fn dom_schema_rejects_invalid_columns_scales_keys_and_metadata() {
        let valid = r#"name = "Test"
enterprise_id = 65000
[tables.dom]
base_oid = "1.3.6.1.4.1.65000.1"
[[tables.dom.metrics]]
key = "rx_power"
oid = "1.3.6.1.4.1.65000.1.5"
unit = "dBm"
label = "Rx Power"
scale = 0.01
"#;
        assert!(toml::from_str::<VendorOidTemplate>(valid).is_ok());
        for invalid in [
            valid.replace("scale = 0.01", "scale = nan"),
            valid.replace("scale = 0.01", "scale = 0"),
            valid.replace("scale = 0.01", "scale = -1"),
            valid.replace("scale = 0.01", "invalid_values = [inf]"),
            valid.replace("rx_power", "temperature"),
            valid.replace("unit = \"dBm\"", "unit = \"watts\""),
            valid.replace("65000.1.5", "65000.2.5"),
            valid.replace("65000.1.5", "65000.x.5"),
            valid.replace("label = \"Rx Power\"", "label = \"\""),
        ] {
            assert!(
                toml::from_str::<VendorOidTemplate>(&invalid).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn optical_samples_roundtrip_all_repository_read_paths_and_migrate_existing_database() {
        use crate::{
            db::{repository::Repository, sqlite::initialize_database},
            monitor::interface::InterfaceMonitor,
        };
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("data.db");
        {
            let repo = Repository::new(initialize_database(&path).unwrap()).unwrap();
            let device = DeviceConfig::new("192.0.2.77", "public");
            let id = repo.save_device_config(&device).unwrap();
            let mut interface = InterfaceMonitor::new(7, "xe-0/0/0");
            interface.rx_optical_power_dbm = Some(-3.5);
            interface.tx_power_dbm = Some(-4.2);
            repo.save_sample(&interface.into_sample(id)).unwrap();
            for samples in [
                repo.get_latest_interfaces(id).unwrap(),
                repo.get_interface_history(id, 10).unwrap(),
                repo.get_recent_interface_samples(id, 7, 10).unwrap(),
                vec![repo.get_latest_interface_sample(id, 7).unwrap().unwrap()],
            ] {
                assert_eq!(samples[0].rx_optical_power_dbm, Some(-3.5));
                assert_eq!(samples[0].tx_power_dbm, Some(-4.2));
            }
            let copper = InterfaceMonitor::new(8, "ge-0/0/1").into_sample(id);
            repo.save_sample(&copper).unwrap();
            assert_eq!(
                repo.get_latest_interface_sample(id, 8)
                    .unwrap()
                    .unwrap()
                    .tx_power_dbm,
                None
            );
        }
        // Reproduce an old schema and verify additive migration preserves the historical Rx value.
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute("ALTER TABLE interface_samples DROP COLUMN tx_power_dbm", [])
                .unwrap();
        }
        {
            let repo = Repository::new(initialize_database(&path).unwrap()).unwrap();
            let device = repo.find_device_by_ip("192.0.2.77").unwrap().unwrap();
            let sample = repo
                .get_latest_interface_sample(device.id.unwrap(), 7)
                .unwrap()
                .unwrap();
            assert_eq!(sample.rx_optical_power_dbm, Some(-3.5));
            assert_eq!(sample.tx_power_dbm, None);
        }
        std::fs::remove_file(path).unwrap();
    }
}
