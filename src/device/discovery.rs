use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::snmp::SnmpClient;
use std::net::Ipv4Addr;

pub fn parse_cidr(cidr: &str) -> Result<(Ipv4Addr, u8), AppError> {
    let Some((network, prefix)) = cidr.split_once('/') else {
        return Err(AppError::Validation(format!("invalid CIDR: {cidr}")));
    };

    let network_ip: Ipv4Addr = network
        .parse()
        .map_err(|_| AppError::Validation(format!("invalid IPv4 address: {network}")))?;
    let prefix: u8 = prefix
        .parse()
        .map_err(|_| AppError::Validation(format!("invalid prefix length: {prefix}")))?;

    if prefix > 32 {
        return Err(AppError::Validation(format!(
            "prefix out of range: {prefix}"
        )));
    }

    Ok((network_ip, prefix))
}

/// Community版: CIDR Discovery は単一サブネット（/24 以上に絞った範囲）のみ許可する。
/// IP Sweep 方式の走査対象を1サブネット分に限定するための制約。
/// 有償版では別リポジトリでこの制限を解放する想定。
pub const COMMUNITY_MIN_SCAN_PREFIX: u8 = 24;

pub fn validate_community_scan_range(cidr: &str) -> Result<(), AppError> {
    let (_, prefix) = parse_cidr(cidr)?;
    if prefix < COMMUNITY_MIN_SCAN_PREFIX {
        return Err(AppError::Validation(format!(
            "Community版では CIDR /{} 以上（単一サブネット、最大 /{}）のみスキャン可能です",
            COMMUNITY_MIN_SCAN_PREFIX, COMMUNITY_MIN_SCAN_PREFIX
        )));
    }
    Ok(())
}

pub fn enumerate_cidr_hosts(cidr: &str) -> Result<Vec<Ipv4Addr>, AppError> {
    let (network_ip, prefix) = parse_cidr(cidr)?;
    let mask = if prefix == 0 {
        0u32
    } else {
        u32::MAX << (32 - prefix)
    };
    let network = u32::from(network_ip) & mask;
    let max_hosts = 1u32 << (32 - prefix);
    let mut hosts = Vec::new();

    for offset in 1..max_hosts.saturating_sub(1) {
        let ip_value = network + offset;
        let ip = Ipv4Addr::from(ip_value);
        hosts.push(ip);
    }

    Ok(hosts)
}

pub fn discover_devices(cidr: &str, community: &str) -> Result<Vec<DeviceConfig>, AppError> {
    let hosts = enumerate_cidr_hosts(cidr)?;
    let client = SnmpClient::new(community);
    let mut devices = Vec::new();

    for ip in hosts.into_iter().take(64) {
        let mut device = DeviceConfig::new(ip.to_string(), community);
        match client.query_device(&device) {
            Ok(info) => {
                device.name = info.sys_name;
                device.status = "online".to_string();
                devices.push(device);
            }
            Err(_) => {
                // ignore unreachable devices; only return SNMP-responsive ones.
            }
        }
    }

    Ok(devices)
}
