use crate::snmp::{SnmpValue, SnmpVarBind};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub const LLDP_REM_TABLE: [u32; 10] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1];
pub const LLDP_REM_CHASSIS_SUBTYPE: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 4];
pub const LLDP_REM_CHASSIS_ID: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 5];
pub const LLDP_REM_PORT_SUBTYPE: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 6];
pub const LLDP_REM_PORT_ID: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 7];
pub const LLDP_REM_SYS_NAME: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 9];
pub const LLDP_REM_MAN_ADDR: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 2, 1, 4];
pub const LLDP_LOC_PORT_TABLE: [u32; 10] = [1, 0, 8802, 1, 1, 2, 1, 3, 7, 1];
pub const LLDP_LOC_PORT_SUBTYPE: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 3, 7, 1, 2];
pub const LLDP_LOC_PORT_ID: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 3, 7, 1, 3];
pub const LLDP_LOC_PORT_DESC: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 3, 7, 1, 4];
pub const IF_DESCR: [u32; 10] = [1, 3, 6, 1, 2, 1, 2, 2, 1, 2];
pub const IF_PHYS_ADDRESS: [u32; 10] = [1, 3, 6, 1, 2, 1, 2, 2, 1, 6];
pub const IF_NAME: [u32; 11] = [1, 3, 6, 1, 2, 1, 31, 1, 1, 1, 1];
pub const IF_ALIAS: [u32; 11] = [1, 3, 6, 1, 2, 1, 31, 1, 1, 1, 18];

pub const CDP_CACHE_TABLE: [u32; 13] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1];
pub const CDP_CACHE_DEVICE_ID: [u32; 14] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1, 6];
pub const CDP_CACHE_DEVICE_PORT: [u32; 14] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1, 7];
pub const CDP_CACHE_ADDRESS: [u32; 14] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1, 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedTopologyProtocol {
    Lldp,
    Cdp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryIdentifier {
    pub subtype: Option<u32>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedNeighbor {
    pub protocol: NormalizedTopologyProtocol,
    pub local_port_if_index: Option<u32>,
    pub local_port_id: Option<Vec<u8>>,
    pub local_port_name: Option<String>,
    pub remote_chassis_id: Option<BinaryIdentifier>,
    pub remote_hostname: Option<String>,
    pub remote_ip: Option<IpAddr>,
    pub remote_port_id: Option<Vec<u8>>,
    pub remote_port_subtype: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NeighborNormalizationQuality {
    /// Distinct remote row suffixes with an invalid index shape.
    pub rejected_remote_rows: usize,
    /// Distinct indexed remote rows missing or containing invalid required fields.
    pub incomplete_remote_rows: usize,
    pub invalid_remote_identity_rows: usize,
    pub invalid_remote_port_rows: usize,
    pub malformed_address_rows: usize,
    /// Malformed LLDP local-port or IF-MIB rows relevant to a nonempty remote table.
    pub malformed_local_rows: usize,
    pub unresolved_local_ports: usize,
}

impl NeighborNormalizationQuality {
    /// Normalization quality only; callers must also check collection completeness.
    pub fn is_complete(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeighborNormalizationOutcome {
    /// Identical output to the corresponding compatibility normalizer, including partial rows.
    pub neighbors: Vec<NormalizedNeighbor>,
    pub quality: NeighborNormalizationQuality,
}

/// Preserves legacy output and reports data loss that must not replace last-known-good data.
pub fn normalize_lldp_neighbors_with_quality(
    remote_rows: &[SnmpVarBind],
    management_addresses: &[SnmpVarBind],
    local_ports: &[SnmpVarBind],
    interfaces: &[SnmpVarBind],
) -> NeighborNormalizationOutcome {
    let neighbors =
        normalize_lldp_neighbors(remote_rows, management_addresses, local_ports, interfaces);
    let mut quality = remote_row_quality(remote_rows, true);
    if !remote_rows.is_empty() {
        let referenced_ports: BTreeSet<_> = remote_rows
            .iter()
            .filter_map(|row| row.oid.strip_prefix(&LLDP_REM_TABLE))
            .filter(|suffix| suffix.len() == 4)
            .map(|suffix| suffix[2])
            .collect();
        let remote_keys: BTreeSet<_> = remote_rows
            .iter()
            .filter_map(|row| row.oid.strip_prefix(&LLDP_REM_TABLE))
            .filter(|suffix| suffix.len() == 4)
            .map(|suffix| suffix[1..].to_vec())
            .collect();
        let mut bad_addresses = BTreeSet::new();
        for row in management_addresses {
            let Some(suffix) = row.oid.strip_prefix(&LLDP_REM_MAN_ADDR) else {
                continue;
            };
            if suffix.len() >= 3 && !remote_keys.contains(&suffix[..3]) {
                continue;
            }
            let valid = suffix.len() >= 5
                && usize::try_from(suffix[4])
                    .ok()
                    .is_some_and(|length| suffix.len() == 5 + length)
                && indexed_address(&suffix[3..]).is_some()
                && lldp_address(&row.value, &suffix[3..]).is_some();
            if !valid {
                bad_addresses.insert(suffix.to_vec());
            }
        }
        quality.malformed_address_rows += bad_addresses.len();
        quality.malformed_local_rows += malformed_local_rows(
            local_ports,
            &[
                &LLDP_LOC_PORT_SUBTYPE,
                &LLDP_LOC_PORT_ID,
                &LLDP_LOC_PORT_DESC,
            ],
            Some(&referenced_ports),
            true,
        );
        quality.malformed_local_rows += malformed_local_rows(
            interfaces,
            &[&IF_NAME, &IF_DESCR, &IF_ALIAS, &IF_PHYS_ADDRESS],
            None,
            false,
        );
    }
    quality.unresolved_local_ports = neighbors
        .iter()
        .filter(|neighbor| neighbor.local_port_if_index.is_none())
        .count();
    NeighborNormalizationOutcome { neighbors, quality }
}

/// Preserves legacy output and reports malformed or incomplete CDP cache rows.
pub fn normalize_cdp_neighbors_with_quality(
    rows: &[SnmpVarBind],
    interfaces: &[SnmpVarBind],
) -> NeighborNormalizationOutcome {
    let neighbors = normalize_cdp_neighbors(rows, interfaces);
    let mut quality = remote_row_quality(rows, false);
    if !rows.is_empty() {
        quality.malformed_local_rows = malformed_local_rows(
            interfaces,
            &[&IF_NAME, &IF_DESCR, &IF_ALIAS, &IF_PHYS_ADDRESS],
            None,
            false,
        );
    }
    NeighborNormalizationOutcome { neighbors, quality }
}

fn nonempty_bytes(value: &SnmpValue) -> bool {
    value.as_bytes().is_some_and(|bytes| !bytes.is_empty())
}

fn identifier_valid(value: &SnmpValue, subtype: Option<u32>, mac_subtype: u32) -> bool {
    nonempty_bytes(value)
        && subtype.is_some_and(|subtype| {
            (1..=7).contains(&subtype)
                && (subtype != mac_subtype
                    || value.as_bytes().is_some_and(|bytes| bytes.len() == 6))
        })
}

fn remote_row_quality(rows: &[SnmpVarBind], lldp: bool) -> NeighborNormalizationQuality {
    let table = if lldp {
        &LLDP_REM_TABLE[..]
    } else {
        &CDP_CACHE_TABLE[..]
    };
    let key_len = if lldp { 3 } else { 2 };
    let mut indexed: BTreeMap<Vec<u32>, BTreeMap<u32, &SnmpValue>> = BTreeMap::new();
    let mut rejected = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for row in rows {
        let Some(suffix) = row.oid.strip_prefix(table) else {
            continue;
        };
        if suffix.len() != key_len + 1
            || suffix[if lldp { 2 } else { 1 }] == 0
            || suffix[key_len] == 0
        {
            rejected.insert(suffix.get(1..).unwrap_or_default().to_vec());
            continue;
        }
        let key = suffix[1..].to_vec();
        if indexed
            .entry(key.clone())
            .or_default()
            .insert(suffix[0], &row.value)
            .is_some()
        {
            duplicates.insert(key);
        }
    }
    let mut quality = NeighborNormalizationQuality {
        rejected_remote_rows: rejected.len(),
        ..Default::default()
    };
    for (key, columns) in indexed {
        let (identity_ok, port_ok) = if lldp {
            (
                columns.get(&5).is_some_and(|value| {
                    identifier_valid(value, columns.get(&4).and_then(|v| value_u32(v)), 4)
                }),
                columns.get(&7).is_some_and(|value| {
                    identifier_valid(value, columns.get(&6).and_then(|v| value_u32(v)), 3)
                }),
            )
        } else {
            (
                columns.get(&6).is_some_and(|value| {
                    value.as_bytes().is_some_and(|bytes| {
                        std::str::from_utf8(bytes)
                            .is_ok_and(|text| !text.trim_matches('\0').trim().is_empty())
                    })
                }),
                columns.get(&7).is_some_and(|value| nonempty_bytes(value)),
            )
        };
        let address_bad = !lldp
            && columns
                .get(&4)
                .is_some_and(|value| cdp_address(value).is_none());
        let hostname_bad = lldp
            && columns.get(&9).is_some_and(|value| {
                value
                    .as_bytes()
                    .is_none_or(|bytes| std::str::from_utf8(bytes).is_err())
            });
        quality.invalid_remote_identity_rows += usize::from(!identity_ok);
        quality.invalid_remote_port_rows += usize::from(!port_ok);
        quality.malformed_address_rows += usize::from(address_bad);
        quality.incomplete_remote_rows += usize::from(
            !identity_ok || !port_ok || address_bad || hostname_bad || duplicates.contains(&key),
        );
    }
    quality
}

fn malformed_local_rows(
    rows: &[SnmpVarBind],
    columns: &[&[u32]],
    referenced: Option<&BTreeSet<u32>>,
    lldp: bool,
) -> usize {
    let mut malformed = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let local_ports = lldp.then(|| local_port_map(rows));
    for row in rows {
        let Some((column, suffix)) = columns.iter().enumerate().find_map(|(column, prefix)| {
            row.oid.strip_prefix(*prefix).map(|suffix| (column, suffix))
        }) else {
            continue;
        };
        if suffix
            .first()
            .is_some_and(|index| referenced.is_some_and(|ports| !ports.contains(index)))
        {
            continue;
        }
        let valid_value = if lldp && column == 0 {
            value_u32(&row.value).is_some_and(|subtype| (1..=7).contains(&subtype))
        } else if lldp && column == 1 {
            nonempty_bytes(&row.value)
                && local_ports
                    .as_ref()
                    .and_then(|ports| suffix.first().and_then(|index| ports.get(index)))
                    .is_some_and(|port| identifier_valid(&row.value, port.subtype, 3))
        } else {
            row.value.as_bytes().is_some()
        };
        if suffix.len() != 1
            || suffix[0] == 0
            || !valid_value
            || !seen.insert((column, suffix.to_vec()))
        {
            malformed.insert(suffix.to_vec());
        }
    }
    malformed.len()
}

#[derive(Default)]
struct LldpRow {
    chassis_subtype: Option<u32>,
    chassis_id: Option<Vec<u8>>,
    port_subtype: Option<u32>,
    port_id: Option<Vec<u8>>,
    hostname: Option<String>,
}

#[derive(Default)]
struct LocalPort {
    subtype: Option<u32>,
    id: Option<Vec<u8>>,
    description: Option<Vec<u8>>,
}

#[derive(Default)]
struct Interface {
    name: Option<Vec<u8>>,
    description: Option<Vec<u8>>,
    alias: Option<Vec<u8>>,
    physical_address: Option<Vec<u8>>,
}

pub fn normalize_lldp_neighbors(
    remote_rows: &[SnmpVarBind],
    management_addresses: &[SnmpVarBind],
    local_ports: &[SnmpVarBind],
    interfaces: &[SnmpVarBind],
) -> Vec<NormalizedNeighbor> {
    let mut rows: BTreeMap<Vec<u32>, LldpRow> = BTreeMap::new();
    for varbind in remote_rows {
        let Some((column, suffix)) = match_column(
            &varbind.oid,
            &[
                &LLDP_REM_CHASSIS_SUBTYPE,
                &LLDP_REM_CHASSIS_ID,
                &LLDP_REM_PORT_SUBTYPE,
                &LLDP_REM_PORT_ID,
                &LLDP_REM_SYS_NAME,
            ],
        ) else {
            continue;
        };
        if suffix.len() < 3 {
            continue;
        }
        let row = rows.entry(suffix[..3].to_vec()).or_default();
        match column {
            0 => row.chassis_subtype = value_u32(&varbind.value),
            1 => row.chassis_id = varbind.value.as_bytes().map(<[u8]>::to_vec),
            2 => row.port_subtype = value_u32(&varbind.value),
            3 => row.port_id = varbind.value.as_bytes().map(<[u8]>::to_vec),
            4 => row.hostname = value_text(&varbind.value),
            _ => unreachable!(),
        }
    }

    let local_ports = local_port_map(local_ports);
    let interfaces = interface_map(interfaces);
    let mut addresses = BTreeMap::new();
    for varbind in management_addresses {
        let Some(suffix) = varbind.oid.strip_prefix(&LLDP_REM_MAN_ADDR) else {
            continue;
        };
        if suffix.len() < 6 {
            continue;
        }
        let key = suffix[..3].to_vec();
        if let Some(address) = lldp_address(&varbind.value, &suffix[3..]) {
            addresses.entry(key).or_insert(address);
        }
    }

    rows.into_iter()
        .map(|(row_key, row)| {
            let local_port_num = row_key.get(1).copied();
            let local_port = local_port_num.and_then(|port| local_ports.get(&port));
            let local_if_index = local_port.and_then(|port| resolve_local_port(port, &interfaces));
            NormalizedNeighbor {
                protocol: NormalizedTopologyProtocol::Lldp,
                local_port_if_index: local_if_index,
                local_port_id: local_port.and_then(|port| port.id.clone()),
                local_port_name: local_if_index
                    .and_then(|index| interfaces.get(&index))
                    .and_then(interface_name),
                remote_chassis_id: row.chassis_id.map(|value| BinaryIdentifier {
                    subtype: row.chassis_subtype,
                    value,
                }),
                remote_hostname: row.hostname,
                remote_ip: addresses.get(&row_key).copied(),
                remote_port_id: row.port_id,
                remote_port_subtype: row.port_subtype,
            }
        })
        .collect()
}

pub fn normalize_cdp_neighbors(
    rows: &[SnmpVarBind],
    interfaces: &[SnmpVarBind],
) -> Vec<NormalizedNeighbor> {
    let mut neighbors: BTreeMap<Vec<u32>, CdpRow> = BTreeMap::new();
    for varbind in rows {
        let Some((column, suffix)) = match_column(
            &varbind.oid,
            &[
                &CDP_CACHE_DEVICE_ID,
                &CDP_CACHE_DEVICE_PORT,
                &CDP_CACHE_ADDRESS,
            ],
        ) else {
            continue;
        };
        if suffix.len() < 2 {
            continue;
        }
        let row = neighbors.entry(suffix[..2].to_vec()).or_default();
        match column {
            0 => row.hostname = value_text(&varbind.value),
            1 => row.port_id = varbind.value.as_bytes().map(<[u8]>::to_vec),
            2 => row.remote_ip = cdp_address(&varbind.value),
            _ => unreachable!(),
        }
    }
    let interfaces = interface_map(interfaces);
    neighbors
        .into_iter()
        .map(|(key, row)| {
            let local_if_index = key.first().copied();
            NormalizedNeighbor {
                protocol: NormalizedTopologyProtocol::Cdp,
                local_port_if_index: local_if_index,
                local_port_id: None,
                local_port_name: local_if_index
                    .and_then(|index| interfaces.get(&index))
                    .and_then(interface_name),
                remote_chassis_id: None,
                remote_hostname: row.hostname,
                remote_ip: row.remote_ip,
                remote_port_id: row.port_id,
                remote_port_subtype: None,
            }
        })
        .collect()
}

#[derive(Default)]
struct CdpRow {
    hostname: Option<String>,
    port_id: Option<Vec<u8>>,
    remote_ip: Option<IpAddr>,
}

pub fn display_binary_identifier(identifier: &BinaryIdentifier) -> String {
    if identifier.subtype == Some(4) && identifier.value.len() == 6 {
        return format_mac(&identifier.value);
    }
    match std::str::from_utf8(&identifier.value) {
        Ok(text) if !text.as_bytes().contains(&0) => text.to_string(),
        _ => format!("0x{}", hex_bytes(&identifier.value)),
    }
}

pub fn display_port_id(bytes: &[u8], subtype: Option<u32>) -> String {
    if subtype == Some(3) && bytes.len() == 6 {
        return format_mac(bytes);
    }
    match std::str::from_utf8(bytes) {
        Ok(text) if !text.as_bytes().contains(&0) => text.to_string(),
        _ => format!("0x{}", hex_bytes(bytes)),
    }
}

fn local_port_map(varbinds: &[SnmpVarBind]) -> HashMap<u32, LocalPort> {
    let mut result: HashMap<u32, LocalPort> = HashMap::new();
    for varbind in varbinds {
        let Some((column, suffix)) = match_column(
            &varbind.oid,
            &[
                &LLDP_LOC_PORT_SUBTYPE,
                &LLDP_LOC_PORT_ID,
                &LLDP_LOC_PORT_DESC,
            ],
        ) else {
            continue;
        };
        let Some(index) = suffix.first().copied() else {
            continue;
        };
        let port = result.entry(index).or_default();
        match column {
            0 => port.subtype = value_u32(&varbind.value),
            1 => port.id = varbind.value.as_bytes().map(<[u8]>::to_vec),
            2 => port.description = varbind.value.as_bytes().map(<[u8]>::to_vec),
            _ => unreachable!(),
        }
    }
    result
}

fn interface_map(varbinds: &[SnmpVarBind]) -> HashMap<u32, Interface> {
    let mut result: HashMap<u32, Interface> = HashMap::new();
    for varbind in varbinds {
        let Some((column, suffix)) = match_column(
            &varbind.oid,
            &[&IF_NAME, &IF_DESCR, &IF_ALIAS, &IF_PHYS_ADDRESS],
        ) else {
            continue;
        };
        let Some(index) = suffix.first().copied() else {
            continue;
        };
        let interface = result.entry(index).or_default();
        let bytes = varbind.value.as_bytes().map(<[u8]>::to_vec);
        match column {
            0 => interface.name = bytes,
            1 => interface.description = bytes,
            2 => interface.alias = bytes,
            3 => interface.physical_address = bytes,
            _ => unreachable!(),
        }
    }
    result
}

fn resolve_local_port(port: &LocalPort, interfaces: &HashMap<u32, Interface>) -> Option<u32> {
    let mut candidates = BTreeSet::new();
    for (index, interface) in interfaces {
        let matches = match port.subtype {
            Some(1) => equal_bytes(port.id.as_deref(), interface.alias.as_deref()),
            Some(3) => equal_bytes(port.id.as_deref(), interface.physical_address.as_deref()),
            Some(5) => equal_bytes(port.id.as_deref(), interface.name.as_deref()),
            _ => false,
        };
        let description_matches =
            equal_bytes(port.description.as_deref(), interface.name.as_deref())
                || equal_bytes(
                    port.description.as_deref(),
                    interface.description.as_deref(),
                );
        if matches || description_matches {
            candidates.insert(*index);
        }
    }
    (candidates.len() == 1).then(|| *candidates.first().unwrap())
}

fn interface_name(interface: &Interface) -> Option<String> {
    interface
        .name
        .as_deref()
        .or(interface.description.as_deref())
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
}

fn equal_bytes(left: Option<&[u8]>, right: Option<&[u8]>) -> bool {
    left.zip(right)
        .is_some_and(|(left, right)| !left.is_empty() && left == right)
}

fn match_column<'a>(oid: &'a [u32], columns: &[&[u32]]) -> Option<(usize, &'a [u32])> {
    columns.iter().enumerate().find_map(|(index, column)| {
        oid.strip_prefix(*column)
            .filter(|suffix| !suffix.is_empty())
            .map(|suffix| (index, suffix))
    })
}

fn value_u32(value: &SnmpValue) -> Option<u32> {
    value.as_u64().and_then(|value| u32::try_from(value).ok())
}

fn value_text(value: &SnmpValue) -> Option<String> {
    value
        .as_bytes()
        .map(|bytes| {
            String::from_utf8_lossy(bytes)
                .trim_matches(char::from(0))
                .to_string()
        })
        .or_else(|| value.as_string())
        .filter(|value| !value.is_empty())
}

fn lldp_address(value: &SnmpValue, suffix: &[u32]) -> Option<IpAddr> {
    match value {
        SnmpValue::IpAddress(address) => Some(IpAddr::V4(Ipv4Addr::from(*address))),
        SnmpValue::OctetString(bytes) | SnmpValue::Opaque(bytes) => match bytes.len() {
            4 => Some(IpAddr::V4(Ipv4Addr::new(
                bytes[0], bytes[1], bytes[2], bytes[3],
            ))),
            16 => Some(IpAddr::V6(Ipv6Addr::from(
                <[u8; 16]>::try_from(bytes.as_slice()).ok()?,
            ))),
            _ => indexed_address(suffix),
        },
        _ => indexed_address(suffix),
    }
}

fn cdp_address(value: &SnmpValue) -> Option<IpAddr> {
    match value {
        SnmpValue::IpAddress(address) => Some(IpAddr::V4(Ipv4Addr::from(*address))),
        SnmpValue::OctetString(bytes) | SnmpValue::Opaque(bytes) if bytes.len() == 4 => Some(
            IpAddr::V4(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3])),
        ),
        _ => value
            .as_string()
            .and_then(|value| value.parse::<IpAddr>().ok()),
    }
}

fn indexed_address(suffix: &[u32]) -> Option<IpAddr> {
    let address_type = *suffix.first()?;
    let length = usize::try_from(*suffix.get(1)?).ok()?;
    let octets = suffix.get(2..2 + length)?;
    let octets: Vec<u8> = octets
        .iter()
        .map(|octet| u8::try_from(*octet).ok())
        .collect::<Option<_>>()?;
    match (address_type, octets.as_slice()) {
        (1, [a, b, c, d]) => Some(IpAddr::V4(Ipv4Addr::new(*a, *b, *c, *d))),
        (2, bytes) if bytes.len() == 16 => Some(IpAddr::V6(Ipv6Addr::from(
            <[u8; 16]>::try_from(bytes).ok()?,
        ))),
        _ => None,
    }
}

fn format_mac(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_lldp_rows() -> Vec<SnmpVarBind> {
        vec![
            integer(
                &[LLDP_REM_CHASSIS_SUBTYPE.as_slice(), &[0, 24, 3]].concat(),
                4,
            ),
            octets(
                &[LLDP_REM_CHASSIS_ID.as_slice(), &[0, 24, 3]].concat(),
                &[0, 1, 2, 3, 4, 5],
            ),
            integer(&[LLDP_REM_PORT_SUBTYPE.as_slice(), &[0, 24, 3]].concat(), 7),
            octets(
                &[LLDP_REM_PORT_ID.as_slice(), &[0, 24, 3]].concat(),
                &[0, 255, 128],
            ),
        ]
    }

    #[test]
    fn quality_accepts_empty_tables_without_inventing_neighbors() {
        let lldp = normalize_lldp_neighbors_with_quality(&[], &[], &[], &[]);
        let cdp = normalize_cdp_neighbors_with_quality(&[], &[]);
        assert!(lldp.neighbors.is_empty());
        assert!(cdp.neighbors.is_empty());
        assert!(lldp.quality.is_complete());
        assert!(cdp.quality.is_complete());
    }

    #[test]
    fn quality_preserves_lldp_mapping_and_binary_ports() {
        let rows = valid_lldp_rows();
        let local = vec![
            integer(&[LLDP_LOC_PORT_SUBTYPE.as_slice(), &[24]].concat(), 5),
            octets(&[LLDP_LOC_PORT_ID.as_slice(), &[24]].concat(), b"uplink"),
        ];
        let interfaces = vec![octets(&[IF_NAME.as_slice(), &[7]].concat(), b"uplink")];
        let outcome = normalize_lldp_neighbors_with_quality(&rows, &[], &local, &interfaces);
        assert!(outcome.quality.is_complete());
        assert_eq!(
            outcome.neighbors,
            normalize_lldp_neighbors(&rows, &[], &local, &interfaces)
        );
        assert_eq!(outcome.neighbors[0].local_port_if_index, Some(7));
        assert_eq!(outcome.neighbors[0].remote_port_id, Some(vec![0, 255, 128]));
    }

    #[test]
    fn quality_counts_lldp_invalid_fields_once_per_remote_row() {
        let mut rows = valid_lldp_rows();
        rows[0].value = SnmpValue::Integer(99);
        rows[1].value = SnmpValue::Null;
        rows[2].value = SnmpValue::Unsupported("undecodable".into());
        rows[3].value = SnmpValue::OctetString(Vec::new());
        let outcome = normalize_lldp_neighbors_with_quality(&rows, &[], &[], &[]);
        assert_eq!(outcome.quality.incomplete_remote_rows, 1);
        assert_eq!(outcome.quality.invalid_remote_identity_rows, 1);
        assert_eq!(outcome.quality.invalid_remote_port_rows, 1);
        assert_eq!(outcome.quality.unresolved_local_ports, 1);
        assert!(!outcome.quality.is_complete());
        assert_eq!(
            outcome.neighbors,
            normalize_lldp_neighbors(&rows, &[], &[], &[])
        );
    }

    #[test]
    fn quality_detects_missing_fields_unknown_columns_and_malformed_indices() {
        let rows = vec![
            octets(
                &[LLDP_REM_SYS_NAME.as_slice(), &[0, 24, 3]].concat(),
                b"leaf",
            ),
            octets(
                &[LLDP_REM_TABLE.as_slice(), &[8, 0, 24, 4]].concat(),
                b"unknown column",
            ),
            octets(&[LLDP_REM_PORT_ID.as_slice(), &[0, 24]].concat(), b"short"),
            octets(
                &[LLDP_REM_CHASSIS_ID.as_slice(), &[0, 24]].concat(),
                b"same short row",
            ),
            octets(
                &[LLDP_REM_PORT_ID.as_slice(), &[0, 24, 3, 9]].concat(),
                b"long",
            ),
        ];
        let outcome = normalize_lldp_neighbors_with_quality(&rows, &[], &[], &[]);
        assert_eq!(outcome.quality.rejected_remote_rows, 2);
        assert_eq!(outcome.quality.incomplete_remote_rows, 2);
        assert_eq!(outcome.quality.invalid_remote_identity_rows, 2);
        assert_eq!(outcome.quality.invalid_remote_port_rows, 2);
    }

    #[test]
    fn quality_reports_relevant_malformed_management_and_local_rows() {
        let rows = valid_lldp_rows();
        let addresses = vec![
            integer(
                &[LLDP_REM_MAN_ADDR.as_slice(), &[0, 24, 3, 1, 4, 192]].concat(),
                7,
            ),
            integer(
                &[LLDP_REM_MAN_ADDR.as_slice(), &[0, 24, 99, 1, 4, 192]].concat(),
                7,
            ),
        ];
        let local = vec![
            integer(&[LLDP_LOC_PORT_SUBTYPE.as_slice(), &[24]].concat(), 5),
            integer(&[LLDP_LOC_PORT_ID.as_slice(), &[24]].concat(), 123),
            integer(&[LLDP_LOC_PORT_ID.as_slice(), &[99]].concat(), 123),
        ];
        let interfaces = vec![integer(&[IF_NAME.as_slice(), &[7]].concat(), 123)];
        let outcome = normalize_lldp_neighbors_with_quality(&rows, &addresses, &local, &interfaces);
        assert_eq!(outcome.quality.malformed_address_rows, 1);
        assert_eq!(outcome.quality.malformed_local_rows, 2);
        assert!(!outcome.quality.is_complete());
        // No remote neighbors means unrelated auxiliary rows cannot imply neighbor loss.
        assert!(
            normalize_lldp_neighbors_with_quality(&[], &addresses, &local, &interfaces)
                .quality
                .is_complete()
        );
    }

    #[test]
    fn quality_accepts_management_address_ifindex_value_and_valid_index() {
        let rows = valid_lldp_rows();
        let addresses = vec![integer(
            &[
                LLDP_REM_MAN_ADDR.as_slice(),
                &[0, 24, 3, 1, 4, 192, 0, 2, 1],
            ]
            .concat(),
            7,
        )];
        let outcome = normalize_lldp_neighbors_with_quality(&rows, &addresses, &[], &[]);
        assert_eq!(outcome.quality.malformed_address_rows, 0);
        assert_eq!(
            outcome.neighbors[0].remote_ip,
            Some("192.0.2.1".parse().unwrap())
        );
    }

    #[test]
    fn quality_detects_cdp_invalid_identity_port_address_and_row_shape() {
        let rows = vec![
            integer(&[CDP_CACHE_DEVICE_ID.as_slice(), &[7, 1]].concat(), 123),
            integer(&[CDP_CACHE_DEVICE_PORT.as_slice(), &[7, 1]].concat(), 456),
            octets(&[CDP_CACHE_ADDRESS.as_slice(), &[7, 1]].concat(), &[1, 2]),
            octets(&[CDP_CACHE_DEVICE_ID.as_slice(), &[7]].concat(), b"short"),
            octets(
                &[CDP_CACHE_DEVICE_PORT.as_slice(), &[7, 1, 9]].concat(),
                b"long",
            ),
        ];
        let outcome = normalize_cdp_neighbors_with_quality(&rows, &[]);
        assert_eq!(outcome.quality.rejected_remote_rows, 2);
        assert_eq!(outcome.quality.incomplete_remote_rows, 1);
        assert_eq!(outcome.quality.invalid_remote_identity_rows, 1);
        assert_eq!(outcome.quality.invalid_remote_port_rows, 1);
        assert_eq!(outcome.quality.malformed_address_rows, 1);
        assert_eq!(outcome.neighbors, normalize_cdp_neighbors(&rows, &[]));
    }

    #[test]
    fn quality_accepts_cdp_binary_port_without_optional_address_or_ifname() {
        let rows = vec![
            octets(&[CDP_CACHE_DEVICE_ID.as_slice(), &[7, 1]].concat(), b"leaf"),
            octets(
                &[CDP_CACHE_DEVICE_PORT.as_slice(), &[7, 1]].concat(),
                &[0, 255],
            ),
        ];
        let outcome = normalize_cdp_neighbors_with_quality(&rows, &[]);
        assert!(outcome.quality.is_complete());
        assert_eq!(outcome.neighbors[0].remote_port_id, Some(vec![0, 255]));
    }

    #[test]
    fn quality_detects_duplicate_columns_and_undecodable_hostname() {
        let mut rows = valid_lldp_rows();
        rows.push(rows[0].clone());
        rows.push(octets(
            &[LLDP_REM_SYS_NAME.as_slice(), &[0, 24, 3]].concat(),
            &[255],
        ));
        let outcome = normalize_lldp_neighbors_with_quality(&rows, &[], &[], &[]);
        assert_eq!(outcome.quality.incomplete_remote_rows, 1);
        assert_eq!(outcome.quality.invalid_remote_identity_rows, 0);
    }

    fn octets(oid: &[u32], value: &[u8]) -> SnmpVarBind {
        SnmpVarBind {
            oid: oid.to_vec(),
            value: SnmpValue::OctetString(value.to_vec()),
        }
    }

    fn integer(oid: &[u32], value: i64) -> SnmpVarBind {
        SnmpVarBind {
            oid: oid.to_vec(),
            value: SnmpValue::Integer(value),
        }
    }

    #[test]
    fn lldp_joins_full_row_key_and_maps_local_port_without_index_assumption() {
        let remote = vec![
            octets(
                &[LLDP_REM_SYS_NAME.as_slice(), &[0, 24, 3]].concat(),
                b"leaf",
            ),
            integer(
                &[LLDP_REM_CHASSIS_SUBTYPE.as_slice(), &[0, 24, 3]].concat(),
                4,
            ),
            octets(
                &[LLDP_REM_CHASSIS_ID.as_slice(), &[0, 24, 3]].concat(),
                &[0, 1, 2, 3, 4, 5],
            ),
            octets(
                &[LLDP_REM_PORT_ID.as_slice(), &[0, 24, 3]].concat(),
                b"Ethernet1",
            ),
        ];
        let addresses = vec![SnmpVarBind {
            oid: [
                LLDP_REM_MAN_ADDR.as_slice(),
                &[0, 24, 3, 1, 4, 192, 0, 2, 4],
            ]
            .concat(),
            value: SnmpValue::OctetString(vec![192, 0, 2, 4]),
        }];
        let local_ports = vec![
            integer(&[LLDP_LOC_PORT_SUBTYPE.as_slice(), &[24]].concat(), 5),
            octets(&[LLDP_LOC_PORT_ID.as_slice(), &[24]].concat(), b"uplink-1"),
        ];
        let interfaces = vec![
            octets(&[IF_NAME.as_slice(), &[7]].concat(), b"uplink-1"),
            octets(&[IF_DESCR.as_slice(), &[7]].concat(), b"Ethernet 1"),
        ];

        let result = normalize_lldp_neighbors(&remote, &addresses, &local_ports, &interfaces);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].local_port_if_index, Some(7));
        assert_eq!(result[0].local_port_name.as_deref(), Some("uplink-1"));
        assert_eq!(
            result[0].remote_ip,
            Some(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 4)))
        );
        assert_eq!(
            result[0].remote_chassis_id,
            Some(BinaryIdentifier {
                subtype: Some(4),
                value: vec![0, 1, 2, 3, 4, 5],
            })
        );
        assert_eq!(
            display_binary_identifier(result[0].remote_chassis_id.as_ref().unwrap()),
            "00:01:02:03:04:05"
        );
    }

    #[test]
    fn unresolved_or_ambiguous_lldp_port_is_not_guessed() {
        let remote = vec![octets(
            &[LLDP_REM_SYS_NAME.as_slice(), &[1, 99, 1]].concat(),
            b"leaf",
        )];
        let local_ports = vec![
            integer(&[LLDP_LOC_PORT_SUBTYPE.as_slice(), &[99]].concat(), 5),
            octets(&[LLDP_LOC_PORT_ID.as_slice(), &[99]].concat(), b"unknown"),
        ];
        let result = normalize_lldp_neighbors(&remote, &[], &local_ports, &[]);

        assert_eq!(result[0].local_port_if_index, None);
        assert_eq!(result[0].local_port_name, None);
    }

    #[test]
    fn neighbor_normalization_preserves_binary_remote_port_ids() {
        let port_id = [0, 0xff, 0x80, b'/'];
        let lldp = normalize_lldp_neighbors(
            &[octets(
                &[LLDP_REM_PORT_ID.as_slice(), &[1, 24, 3]].concat(),
                &port_id,
            )],
            &[],
            &[],
            &[],
        );
        let cdp = normalize_cdp_neighbors(
            &[octets(
                &[CDP_CACHE_DEVICE_PORT.as_slice(), &[42, 2]].concat(),
                &port_id,
            )],
            &[],
        );

        assert_eq!(lldp[0].remote_port_id.as_deref(), Some(port_id.as_slice()));
        assert_eq!(cdp[0].remote_port_id.as_deref(), Some(port_id.as_slice()));
        assert_eq!(lldp[0].local_port_if_index, None);
        assert_eq!(cdp[0].local_port_if_index, Some(42));
    }

    #[test]
    fn conflicting_lldp_identity_and_description_are_ambiguous() {
        let port = LocalPort {
            subtype: Some(5),
            id: Some(b"uplink".to_vec()),
            description: Some(b"Ethernet 1".to_vec()),
        };
        let mut interfaces = HashMap::from([
            (
                7,
                Interface {
                    name: Some(b"uplink".to_vec()),
                    ..Default::default()
                },
            ),
            (
                8,
                Interface {
                    description: Some(b"Ethernet 1".to_vec()),
                    ..Default::default()
                },
            ),
        ]);

        assert_eq!(resolve_local_port(&port, &interfaces), None);
        interfaces.get_mut(&8).unwrap().description = None;
        assert_eq!(resolve_local_port(&port, &interfaces), Some(7));
        interfaces.get_mut(&8).unwrap().name = Some(b"uplink".to_vec());
        assert_eq!(resolve_local_port(&port, &interfaces), None);
    }

    #[test]
    fn lldp_description_fallback_requires_unique_interface() {
        let port = LocalPort {
            subtype: Some(5),
            id: Some(b"unavailable".to_vec()),
            description: Some(b"Ethernet 1".to_vec()),
        };
        let mut interfaces = HashMap::from([(
            7,
            Interface {
                description: Some(b"Ethernet 1".to_vec()),
                ..Default::default()
            },
        )]);

        assert_eq!(resolve_local_port(&port, &interfaces), Some(7));
        interfaces.insert(
            8,
            Interface {
                description: Some(b"Ethernet 1".to_vec()),
                ..Default::default()
            },
        );
        assert_eq!(resolve_local_port(&port, &interfaces), None);
    }

    #[test]
    fn cdp_uses_first_of_two_indices_as_ifindex_and_joins_address_by_row() {
        let rows = vec![
            octets(
                &[CDP_CACHE_DEVICE_ID.as_slice(), &[42, 2]].concat(),
                b"leaf",
            ),
            octets(
                &[CDP_CACHE_DEVICE_PORT.as_slice(), &[42, 2]].concat(),
                b"Gi1/0/2",
            ),
            octets(
                &[CDP_CACHE_ADDRESS.as_slice(), &[42, 2]].concat(),
                &[192, 0, 2, 42],
            ),
        ];
        let interfaces = vec![octets(
            &[IF_NAME.as_slice(), &[42]].concat(),
            b"GigabitEthernet1/0/1",
        )];

        let result = normalize_cdp_neighbors(&rows, &interfaces);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].local_port_if_index, Some(42));
        assert_eq!(
            result[0].local_port_name.as_deref(),
            Some("GigabitEthernet1/0/1")
        );
        assert_eq!(
            result[0].remote_ip,
            Some(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 42)))
        );
    }
}
