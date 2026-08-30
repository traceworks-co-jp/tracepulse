use crate::config::SnmpConfig;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::monitor::interface::InterfaceMonitor;
use snmp::{SyncSession, Value};
use std::convert::TryFrom;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SnmpClient {
    community: String,
    overrides: Option<SnmpConfig>,
}

impl SnmpClient {
    pub fn new(community: impl Into<String>) -> Self {
        Self {
            community: community.into(),
            overrides: None,
        }
    }

    pub fn with_snmp_config(community: impl Into<String>, overrides: SnmpConfig) -> Self {
        Self {
            community: community.into(),
            overrides: Some(overrides),
        }
    }

    pub fn query_device(&self, device: &DeviceConfig) -> Result<SnmpDeviceInfo, AppError> {
        if device.ip.trim().is_empty() {
            return Err(AppError::Validation("SNMP device IP is empty".to_string()));
        }

        let community = if device.community.trim().is_empty() {
            self.community.clone()
        } else {
            device.community.clone()
        };

        let session_addr = format!("{}:161", device.ip);
        let mut session = SyncSession::new(
            session_addr.as_str(),
            community.as_bytes(),
            Some(Duration::from_secs(2)),
            1,
        )
        .map_err(|err| AppError::Validation(format!("SNMP session error for {}: {}", device.ip, err)))?;

        let sys_name = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 5, 0])
            .unwrap_or_else(|_| format!("{}-snmp", device.ip.replace('.', "-")));

        let sys_descr = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 1, 0])
            .unwrap_or_else(|_| format!("reachable via SNMP v2c community '{}'", community));

        let sys_object_id = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 2, 0]).ok();

        let vendor_enterprise_id = sys_object_id.as_deref().and_then(parse_enterprise_id_from_sys_object_id);

        let cpu_usage = self.query_cpu_usage(&mut session, vendor_enterprise_id)?;

        let memory_usage = None;
        let hardware = self.query_hardware_inventory_with_session(&mut session)?;

        Ok(SnmpDeviceInfo {
            sys_name,
            sys_descr,
            cpu_usage,
            memory_usage,
            hardware_sensors: hardware.sensors,
        })
    }

    pub fn query_hardware_sensors(&self, device: &DeviceConfig) -> Result<Vec<SnmpHardwareSensor>, AppError> {
        let mut session = self.open_session(device)?;
        Ok(self.query_hardware_inventory_with_session(&mut session)?.sensors)
    }

    pub fn diagnose_device(&self, device: &DeviceConfig) -> Result<SnmpDiagnostics, AppError> {
        if device.ip.trim().is_empty() {
            return Err(AppError::Validation("SNMP device IP is empty".to_string()));
        }

        let community = if device.community.trim().is_empty() {
            self.community.clone()
        } else {
            device.community.clone()
        };

        let session_addr = format!("{}:161", device.ip);
        let mut session = SyncSession::new(
            session_addr.as_str(),
            community.as_bytes(),
            Some(Duration::from_secs(2)),
            1,
        )
        .map_err(|err| AppError::Validation(format!("SNMP session error for {}: {}", device.ip, err)))?;

        let sys_name = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 5, 0])
            .unwrap_or_else(|_| format!("{}-snmp", device.ip.replace('.', "-")));
        let sys_descr = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 1, 0])
            .unwrap_or_else(|_| format!("reachable via SNMP v2c community '{}'", community));
        let sys_object_id = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 2, 0]).ok();
        let vendor_enterprise_id = sys_object_id.as_deref().and_then(parse_enterprise_id_from_sys_object_id);
        let vendor_name = vendor_enterprise_id.and_then(vendor_name_from_enterprise_id).map(|s| s.to_string());

        let mut cpu_probes = Vec::new();
        let cpu_override = self.overrides.as_ref().and_then(|o| parse_oid_string(&o.cpu_oid_override));
        let cpu_override_value = cpu_override
            .as_ref()
            .and_then(|oid| query_u32_with_table_fallback(&mut session, oid).ok().flatten())
            .filter(|v| *v > 0);
        if let Some(ref oid) = cpu_override {
            cpu_probes.push(SnmpOidProbe {
                label: "Manual CPU Override".to_string(),
                oid: oid_to_string(oid),
                value: cpu_override_value,
                selected: cpu_override_value.is_some(),
                status: if cpu_override_value.is_some() { "ok" } else { "n/a" }.to_string(),
            });
        }

        let (vendor_cpu_value, vendor_cpu_rows) = self.diagnose_cpu_vendor_candidates(&mut session, vendor_enterprise_id, vendor_name.as_deref())?;
        cpu_probes.extend(vendor_cpu_rows);

        let cpu_standard = query_table_average_u32(&mut session, &[1, 3, 6, 1, 2, 1, 25, 3, 3, 1, 2]).unwrap_or(None);
        cpu_probes.push(SnmpOidProbe {
            label: "Standard hrProcessorLoad".to_string(),
            oid: "1.3.6.1.2.1.25.3.3.1.2".to_string(),
            value: cpu_standard,
            selected: false,
            status: if cpu_standard.filter(|v| *v > 0).is_some() { "ok" } else { "n/a" }.to_string(),
        });

        let cpu_selected = cpu_override_value
            .or(vendor_cpu_value)
            .or(cpu_standard.filter(|v| *v > 0));

        let memory_usage = None;
        let memory_probes = Vec::new();

        let interface_indexes = self.discover_interface_indexes(device)?;
        let mut interfaces = Vec::new();
        for if_index in interface_indexes {
            if let Ok(iface) = self.query_interface(device, if_index, None) {
                interfaces.push(SnmpInterfaceDiagnostic {
                    if_index: iface.if_index as u32,
                    if_name: iface.if_name,
                    link_status: iface.link_status,
                });
            }
        }

        let hardware = self.query_hardware_inventory(device).unwrap_or_else(|_| HardwareInventory { sensors: Vec::new(), probes: Vec::new() });

        Ok(SnmpDiagnostics {
            sys_name,
            sys_descr,
            sys_object_id,
            enterprise_id: vendor_enterprise_id,
            vendor_name,
            cpu_usage: cpu_selected,
            memory_usage,
            cpu_probes,
            memory_probes,
            interfaces,
            hardware_sensors: hardware.sensors,
            hardware_probes: hardware.probes,
        })
    }

    /// Discovery専用の軽量プローブ。sysName (OID 1.3.6.1.2.1.1.5.0) 1つだけ取得する。
    /// タイムアウトは 500ms、アプリレベルで最大3回リトライ。
    /// 並列スキャン時のUDPパケットロスによる取りこぼしを防ぐ。
    pub fn probe_device(&self, device: &DeviceConfig) -> Result<String, AppError> {
        if device.ip.trim().is_empty() {
            return Err(AppError::Validation("SNMP device IP is empty".to_string()));
        }

        let community = if device.community.trim().is_empty() {
            self.community.clone()
        } else {
            device.community.clone()
        };

        const MAX_RETRIES: u32 = 3;
        let session_addr = format!("{}:161", device.ip);
        let mut last_err = String::new();

        for attempt in 0..MAX_RETRIES {
            // 試行ごとに新しいセッション（UDPソケット）を作成して再送
            let mut session = match SyncSession::new(
                session_addr.as_str(),
                community.as_bytes(),
                Some(Duration::from_millis(500)),
                0,
            ) {
                Ok(s) => s,
                Err(err) => {
                    last_err = format!("session error on attempt {}: {}", attempt + 1, err);
                    continue;
                }
            };

            match query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 5, 0]) {
                Ok(sys_name) => return Ok(sys_name),
                Err(err) => {
                    last_err = format!("no response on attempt {}: {}", attempt + 1, err);
                }
            }
        }

        Err(AppError::Validation(format!(
            "SNMP probe failed for {} after {} attempts: {}",
            device.ip, MAX_RETRIES, last_err
        )))
    }

    pub fn discover_interface_indexes(&self, device: &DeviceConfig) -> Result<Vec<u32>, AppError> {
        let mut session = self.open_session(device)?;
        // ifIndex テーブル (1.3.6.1.2.1.2.2.1.1) を GETNEXT でウォーク
        let prefix = [1u32, 3, 6, 1, 2, 1, 2, 2, 1, 1];
        let mut current = prefix.to_vec();
        let mut indexes = Vec::new();

        for _ in 0..64 {
            let mut response = match session.getnext(&current) {
                Ok(r) => r,
                Err(_) => break,
            };

            let Some((name, value)) = response.varbinds.next() else {
                break;
            };

            let mut oid_buf = [0u32; 128];
            let oid = match name.read_name(&mut oid_buf) {
                Ok(o) => o,
                Err(_) => break,
            };

            // prefix を外れたら終了
            if !oid.starts_with(&prefix) {
                break;
            }

            let index = match value {
                Value::Integer(v)    => v as u32,
                Value::Unsigned32(v) => v,
                Value::Counter32(v)  => v,
                _ => {
                    current = oid.to_vec();
                    continue;
                }
            };

            if !indexes.contains(&index) {
                indexes.push(index);
            }
            current = oid.to_vec();
        }

        // GETNEXT ウォークで何も取れない機器は ifIndex 1〜8 を直接試す
        if indexes.is_empty() {
            for i in 1u32..=8 {
                let oid = [1u32, 3, 6, 1, 2, 1, 2, 2, 1, 1, i];
                if let Ok(mut r) = session.get(&oid) {
                    if r.varbinds.next().is_some() {
                        indexes.push(i);
                    }
                }
            }
        }

        Ok(indexes)
    }

    pub fn query_interface(
        &self,
        device: &DeviceConfig,
        if_index: u32,
        if_name: Option<&str>,
    ) -> Result<InterfaceMonitor, AppError> {
        if device.ip.trim().is_empty() {
            return Err(AppError::Validation("SNMP device IP is empty".to_string()));
        }

        let community = if device.community.trim().is_empty() {
            self.community.clone()
        } else {
            device.community.clone()
        };

        let session_addr = format!("{}:161", device.ip);
        let mut session = SyncSession::new(
            session_addr.as_str(),
            community.as_bytes(),
            Some(Duration::from_secs(2)),
            1,
        )
        .map_err(|err| AppError::Validation(format!("SNMP session error for {}: {}", device.ip, err)))?;

        let oid_prefix = [1, 3, 6, 1, 2, 1, 2, 2, 1];
        let link_status = query_u32(&mut session, &oid_prefix.iter().copied().chain([8, if_index]).collect::<Vec<_>>())
            .unwrap_or(1);
        let in_errors = query_u64(&mut session, &oid_prefix.iter().copied().chain([14, if_index]).collect::<Vec<_>>())
            .unwrap_or(0);
        let out_errors = query_u64(&mut session, &oid_prefix.iter().copied().chain([20, if_index]).collect::<Vec<_>>())
            .unwrap_or(0);
        let in_discards = query_u64(&mut session, &oid_prefix.iter().copied().chain([13, if_index]).collect::<Vec<_>>())
            .unwrap_or(0);
        let out_discards = query_u64(&mut session, &oid_prefix.iter().copied().chain([19, if_index]).collect::<Vec<_>>())
            .unwrap_or(0);
        let in_octets = query_u64(&mut session, &oid_prefix.iter().copied().chain([10, if_index]).collect::<Vec<_>>())
            .unwrap_or(0);
        let out_octets = query_u64(&mut session, &oid_prefix.iter().copied().chain([16, if_index]).collect::<Vec<_>>())
            .unwrap_or(0);

        let resolved_name = match if_name {
            Some(name) => name.to_string(),
            None => {
                let name_oid = [1, 3, 6, 1, 2, 1, 2, 2, 1, 2, if_index];
                query_string(&mut session, &name_oid).unwrap_or_else(|_| format!("if-{}", if_index))
            }
        };

        Ok(InterfaceMonitor::from_snmp_snapshot(
            if_index as i32,
            &resolved_name,
            link_status,
            in_errors,
            out_errors,
            in_discards,
            out_discards,
            in_octets,
            out_octets,
        ))
    }

    fn open_session(&self, device: &DeviceConfig) -> Result<SyncSession, AppError> {
        let community = if device.community.trim().is_empty() {
            self.community.clone()
        } else {
            device.community.clone()
        };

        let session_addr = format!("{}:161", device.ip);
        SyncSession::new(
            session_addr.as_str(),
            community.as_bytes(),
            Some(Duration::from_secs(2)),
            1,
        )
        .map_err(|err| AppError::Validation(format!("SNMP session error for {}: {}", device.ip, err)))
    }
}

fn query_string(session: &mut SyncSession, oid: &[u32]) -> Result<String, AppError> {
    let mut response = session
        .get(oid)
        .map_err(|err| AppError::Validation(format!("SNMP GET failed for OID {:?}: {:?}", oid, err)))?;

    let value = response
        .varbinds
        .find_map(|(name, value)| {
            if name == oid {
                match value {
                    Value::OctetString(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
                    Value::ObjectIdentifier(oid_name) => Some(oid_name.to_string()),
                    Value::Integer(value) => Some(value.to_string()),
                    Value::Unsigned32(value) => Some(value.to_string()),
                    _ => None,
                }
            } else {
                None
            }
        })
        .ok_or_else(|| AppError::Validation(format!("OID {:?} was not returned", oid)))?;

    Ok(value)
}

fn query_u32(session: &mut SyncSession, oid: &[u32]) -> Result<u32, AppError> {
    let mut response = session
        .get(oid)
        .map_err(|err| AppError::Validation(format!("SNMP GET failed for OID {:?}: {:?}", oid, err)))?;

    let value = response
        .varbinds
        .find_map(|(name, value)| {
            if name == oid {
                match value {
                    Value::Integer(value) => Some(value as u32),
                    Value::Unsigned32(value) => Some(value),
                    Value::Counter32(value) => Some(value),
                    Value::Timeticks(value) => Some(value),
                    _ => None,
                }
            } else {
                None
            }
        })
        .ok_or_else(|| AppError::Validation(format!("OID {:?} was not returned", oid)))?;

    Ok(value)
}

fn query_u64_with_table_fallback(session: &mut SyncSession, oid: &[u32]) -> Result<Option<u64>, AppError> {
    if let Ok(value) = query_u64(session, oid) {
        if value > 0 {
            return Ok(Some(value));
        }
    }

    query_table_first_u64(session, oid)
}

fn query_memory_usage(session: &mut SyncSession, vendor_enterprise_id: Option<u32>) -> Result<Option<u32>, AppError> {
    if vendor_enterprise_id == Some(9) {
        let (usage, _) = query_cisco_memory_candidates(session)?;
        if usage.is_some() {
            return Ok(usage);
        }
    }

    let (usage, _) = query_hr_storage_memory_usage(session)?;
    Ok(usage)
}

fn query_u64(session: &mut SyncSession, oid: &[u32]) -> Result<u64, AppError> {
    let mut response = session
        .get(oid)
        .map_err(|err| AppError::Validation(format!("SNMP GET failed for OID {:?}: {:?}", oid, err)))?;

    let value = response
        .varbinds
        .find_map(|(name, value)| {
            if name == oid {
                match value {
                    Value::Integer(value) => Some(value as u64),
                    Value::Unsigned32(value) => Some(value as u64),
                    Value::Counter32(value) => Some(value as u64),
                    Value::Counter64(value) => Some(value),
                    Value::Timeticks(value) => Some(value as u64),
                    _ => None,
                }
            } else {
                None
            }
        })
        .ok_or_else(|| AppError::Validation(format!("OID {:?} was not returned", oid)))?;

    Ok(value)
}

fn query_cpu_usage(session: &mut SyncSession, vendor_enterprise_id: Option<u32>) -> Result<Option<u32>, AppError> {
    if vendor_enterprise_id == Some(9) {
        let (usage, _) = query_cisco_cpu_candidates(session, Some("Cisco"))?;
        if usage.is_some() {
            return Ok(usage);
        }
    }

    let standard = query_table_average_u32(session, &[1, 3, 6, 1, 2, 1, 25, 3, 3, 1, 2]).unwrap_or(None);
    if let Some(value) = standard.filter(|value| *value > 0) {
        return Ok(Some(value));
    }

    if let Some(vendor_oid) = vendor_cpu_oid(vendor_enterprise_id) {
        let vendor_value = query_u32(session, vendor_oid).ok().filter(|v| *v > 0);
        if vendor_value.is_some() {
            return Ok(vendor_value);
        }
    }

    Ok(None)
}

fn query_cisco_cpu_candidates(session: &mut SyncSession, vendor_name: Option<&str>) -> Result<(Option<u32>, Vec<SnmpOidProbe>), AppError> {
    let mut probes = Vec::new();
    let cpu_1m_oid = [1, 3, 6, 1, 4, 1, 9, 2, 1, 57, 0];
    let cpu_5m_oid = [1, 3, 6, 1, 4, 1, 9, 2, 1, 56, 0];
    let cpm_oid = [1, 3, 6, 1, 4, 1, 9, 9, 109, 1, 1, 1, 1, 5];

    let cpu_1m = query_u32(session, &cpu_1m_oid).ok().filter(|v| *v > 0);
    let cpu_5m = query_u32(session, &cpu_5m_oid).ok().filter(|v| *v > 0);
    let cpm = query_u32_with_table_fallback(session, &cpm_oid).ok().flatten().filter(|v| *v > 0);
    let selected = cpu_1m.or(cpu_5m).or(cpm);

    probes.push(SnmpOidProbe {
        label: format!("{} avgBusy1", vendor_name.unwrap_or("Cisco")),
        oid: oid_to_string(&cpu_1m_oid),
        value: cpu_1m,
        selected: selected == cpu_1m,
        status: if cpu_1m.is_some() { "ok" } else { "n/a" }.to_string(),
    });
    probes.push(SnmpOidProbe {
        label: format!("{} avgBusy5", vendor_name.unwrap_or("Cisco")),
        oid: oid_to_string(&cpu_5m_oid),
        value: cpu_5m,
        selected: selected.is_none() && cpu_5m.is_some(),
        status: if cpu_5m.is_some() { "ok" } else { "n/a" }.to_string(),
    });
    probes.push(SnmpOidProbe {
        label: format!("{} cpmCPUTotal", vendor_name.unwrap_or("Cisco")),
        oid: oid_to_string(&cpm_oid),
        value: cpm,
        selected: selected.is_none() && cpm.is_some(),
        status: if cpm.is_some() { "ok" } else { "n/a" }.to_string(),
    });

    Ok((selected, probes))
}

fn query_table_average_u32(session: &mut SyncSession, prefix: &[u32]) -> Result<Option<u32>, AppError> {
    let mut current = prefix.to_vec();
    let mut values = Vec::new();

    for _ in 0..64 {
        let mut response = match session.getnext(&current) {
            Ok(response) => response,
            Err(_) => break,
        };

        let Some((name, value)) = response.varbinds.next() else {
            break;
        };

        let mut oid_buf = [0u32; 128];
        let oid = match name.read_name(&mut oid_buf) {
            Ok(oid) => oid,
            Err(_) => break,
        };

        if !oid.starts_with(prefix) {
            break;
        }

        let parsed = match value {
            Value::Integer(value) => Some(value as u32),
            Value::Unsigned32(value) => Some(value),
            Value::Counter32(value) => Some(value),
            Value::Timeticks(value) => Some(value),
            _ => None,
        };

        if let Some(value) = parsed {
            values.push(value);
        }

        current = oid.to_vec();
    }

    if values.is_empty() {
        return Ok(None);
    }

    let total: u64 = values.iter().map(|v| *v as u64).sum();
    Ok(Some((total / values.len() as u64) as u32))
}

fn parse_enterprise_id_from_sys_object_id(sys_object_id: &str) -> Option<u32> {
    let parts: Vec<u32> = sys_object_id
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect();

    if parts.len() >= 7 && parts[..6] == [1, 3, 6, 1, 4, 1] {
        parts.get(6).copied()
    } else {
        None
    }
}

fn vendor_cpu_oid(enterprise_id: Option<u32>) -> Option<&'static [u32]> {
    match enterprise_id {
        Some(9) => Some(&[1, 3, 6, 1, 4, 1, 9, 9, 109, 1, 1, 1, 1, 5][..]),   // Cisco
        Some(1182) => Some(&[1, 3, 6, 1, 4, 1, 1182, 2, 1, 4][..]),           // Yamaha
        Some(2078) => Some(&[1, 3, 6, 1, 4, 1, 2078, 4, 4, 3, 3, 2][..]),     // Allied Telesis
        _ => None,
    }
}

impl SnmpClient {
    fn query_cpu_usage(&self, session: &mut SyncSession, vendor_enterprise_id: Option<u32>) -> Result<Option<u32>, AppError> {
        if let Some(overrides) = &self.overrides {
            if let Some(cpu_oid) = parse_oid_string(&overrides.cpu_oid_override) {
                if let Ok(value) = query_u32(session, &cpu_oid) {
                    if value > 0 {
                        return Ok(Some(value));
                    }
                }
            }
        }
        query_cpu_usage(session, vendor_enterprise_id)
    }

    fn query_memory_usage(&self, session: &mut SyncSession, vendor_enterprise_id: Option<u32>) -> Result<Option<u32>, AppError> {
        let _ = (session, vendor_enterprise_id);
        Ok(None)
    }

    fn diagnose_cpu_vendor_candidates(
        &self,
        session: &mut SyncSession,
        vendor_enterprise_id: Option<u32>,
        vendor_name: Option<&str>,
    ) -> Result<(Option<u32>, Vec<SnmpOidProbe>), AppError> {
        if vendor_enterprise_id == Some(9) {
            return query_cisco_cpu_candidates(session, vendor_name);
        }

        let mut probes = Vec::new();
        if let Some(vendor_oid) = vendor_cpu_oid(vendor_enterprise_id) {
            let vendor_value = query_u32_with_table_fallback(session, vendor_oid).ok().flatten().filter(|v| *v > 0);
            probes.push(SnmpOidProbe {
                label: format!("{} preset", vendor_name.unwrap_or("Vendor")),
                oid: oid_to_string(vendor_oid),
                value: vendor_value,
                selected: vendor_value.is_some(),
                status: if vendor_value.is_some() { "ok" } else { "n/a" }.to_string(),
            });
            return Ok((vendor_value, probes));
        }

        Ok((None, probes))
    }

    fn diagnose_memory_vendor_candidates(
        &self,
        session: &mut SyncSession,
        vendor_enterprise_id: Option<u32>,
    ) -> Result<(Option<u32>, Vec<SnmpOidProbe>), AppError> {
        if vendor_enterprise_id == Some(9) {
            return query_cisco_memory_candidates(session);
        }

        Ok((None, Vec::new()))
    }
}

fn parse_oid_string(input: &str) -> Option<Vec<u32>> {
    let oid: Vec<u32> = input
        .split('.')
        .filter_map(|part| part.trim().parse::<u32>().ok())
        .collect();
    if oid.is_empty() { None } else { Some(oid) }
}

fn query_u32_with_table_fallback(session: &mut SyncSession, oid: &[u32]) -> Result<Option<u32>, AppError> {
    if let Ok(value) = query_u32(session, oid) {
        if value > 0 {
            return Ok(Some(value));
        }
    }
    query_table_first_u32(session, oid)
}

fn query_cisco_memory_candidates(session: &mut SyncSession) -> Result<(Option<u32>, Vec<SnmpOidProbe>), AppError> {
    let mut probes = Vec::new();
    let pool_name_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 48, 1, 1, 1, 2];
    let used_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 48, 1, 1, 1, 5];
    let free_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 48, 1, 1, 1, 6];
    let old_free_oid = [1, 3, 6, 1, 4, 1, 9, 2, 1, 8, 0];

    let mut pool_candidates = Vec::new();
    for (index, name) in query_table_strings(session, &pool_name_prefix)? {
        let used = query_u64_with_table_fallback(session, &used_prefix.iter().copied().chain([index]).collect::<Vec<_>>())
            .ok()
            .flatten()
            .filter(|v| *v > 0);
        let free = query_u64_with_table_fallback(session, &free_prefix.iter().copied().chain([index]).collect::<Vec<_>>())
            .ok()
            .flatten()
            .filter(|v| *v > 0);
        let usage = match (used, free) {
            (Some(used), Some(free)) => used.checked_add(free).and_then(|total| calculate_percentage_u64(used, total)),
            _ => None,
        };
        pool_candidates.push(CiscoMemoryPoolCandidate {
            index,
            name: Some(name),
            usage,
        });
    }

    let used = query_u64_with_table_fallback(session, &used_prefix.iter().copied().chain([1]).collect::<Vec<_>>())
        .ok()
        .flatten()
        .filter(|v| *v > 0);
    let free = query_u64_with_table_fallback(session, &free_prefix.iter().copied().chain([1]).collect::<Vec<_>>())
        .ok()
        .flatten()
        .filter(|v| *v > 0);
    let old_free = query_u64(session, &old_free_oid).ok().filter(|v| *v > 0);
    let usage = select_cisco_memory_usage(&pool_candidates)
        .or_else(|| match (used, free) {
            (Some(used), Some(free)) => used.checked_add(free).and_then(|total| calculate_percentage_u64(used, total)),
            _ => None,
        });

    probes.push(SnmpOidProbe {
        label: "Cisco Memory Pool (selected)".to_string(),
        oid: pool_candidates
            .iter()
            .max_by_key(|candidate| candidate.usage.unwrap_or(0))
            .map(|candidate| format!("{}.{}", oid_to_string(&used_prefix), candidate.index))
            .unwrap_or_else(|| format!("{}.1", oid_to_string(&used_prefix))),
        value: usage,
        selected: usage.is_some(),
        status: if usage.is_some() { "ok" } else { "n/a" }.to_string(),
    });
    probes.push(SnmpOidProbe {
        label: "Cisco freeMem (old)".to_string(),
        oid: oid_to_string(&old_free_oid),
        value: old_free.and_then(|v| u32::try_from(v).ok()),
        selected: false,
        status: if old_free.is_some() { "ok" } else { "n/a" }.to_string(),
    });

    if probes.len() == 1 && !pool_candidates.is_empty() {
        for candidate in pool_candidates.iter().take(4) {
            probes.push(SnmpOidProbe {
                label: format!(
                    "Cisco pool {} ({})",
                    candidate.name.as_deref().unwrap_or("unknown"),
                    candidate.index
                ),
                oid: format!("{}.{}", oid_to_string(&used_prefix), candidate.index),
                value: candidate.usage,
                selected: candidate.usage == usage,
                status: if candidate.usage.is_some() { "ok" } else { "n/a" }.to_string(),
            });
        }
    }

    Ok((usage, probes))
}

fn query_table_strings(session: &mut SyncSession, prefix: &[u32]) -> Result<Vec<(u32, String)>, AppError> {
    let mut current = prefix.to_vec();
    let mut rows = Vec::new();

    for _ in 0..64 {
        let mut response = match session.getnext(&current) {
            Ok(response) => response,
            Err(_) => break,
        };

        let Some((name, value)) = response.varbinds.next() else {
            break;
        };

        let mut oid_buf = [0u32; 128];
        let oid = match name.read_name(&mut oid_buf) {
            Ok(oid) => oid,
            Err(_) => break,
        };

        if !oid.starts_with(prefix) {
            break;
        }

        let Some(index) = oid.last().copied() else {
            current = oid.to_vec();
            continue;
        };

        let value = match value {
            Value::OctetString(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
            Value::ObjectIdentifier(oid_name) => Some(oid_name.to_string()),
            Value::Integer(value) => Some(value.to_string()),
            Value::Unsigned32(value) => Some(value.to_string()),
            Value::Counter32(value) => Some(value.to_string()),
            _ => None,
        };

        if let Some(value) = value {
            rows.push((index, value));
        }

        current = oid.to_vec();
    }

    Ok(rows)
}

fn query_table_i64_values(session: &mut SyncSession, prefix: &[u32]) -> Result<Vec<(u32, i64)>, AppError> {
    let mut current = prefix.to_vec();
    let mut rows = Vec::new();

    for _ in 0..64 {
        let mut response = match session.getnext(&current) {
            Ok(response) => response,
            Err(_) => break,
        };

        let Some((name, value)) = response.varbinds.next() else {
            break;
        };

        let mut oid_buf = [0u32; 128];
        let oid = match name.read_name(&mut oid_buf) {
            Ok(oid) => oid,
            Err(_) => break,
        };

        if !oid.starts_with(prefix) {
            break;
        }

        let Some(index) = oid.last().copied() else {
            current = oid.to_vec();
            continue;
        };

        let value = match value {
            Value::Integer(value) => Some(value as i64),
            Value::Unsigned32(value) => Some(value as i64),
            Value::Counter32(value) => Some(value as i64),
            Value::Counter64(value) => Some(value as i64),
            _ => None,
        };

        if let Some(value) = value {
            rows.push((index, value));
        }

        current = oid.to_vec();
    }

    Ok(rows)
}

#[derive(Debug, Clone)]
struct CiscoMemoryPoolCandidate {
    index: u32,
    name: Option<String>,
    usage: Option<u32>,
}

fn select_cisco_memory_usage(candidates: &[CiscoMemoryPoolCandidate]) -> Option<u32> {
    candidates
        .iter()
        .filter(|candidate| candidate.usage.is_some())
        .max_by_key(|candidate| {
            let name = candidate.name.as_deref().unwrap_or("").to_lowercase();
            let priority = if name.contains("processor") {
                300
            } else if name.contains("main") {
                200
            } else if name.contains("io") || name.contains("i/o") {
                100
            } else {
                0
            };
            priority + candidate.usage.unwrap_or(0) as i32
        })
        .and_then(|candidate| candidate.usage)
}

fn query_table_first_u32(session: &mut SyncSession, prefix: &[u32]) -> Result<Option<u32>, AppError> {
    let mut current = prefix.to_vec();

    for _ in 0..64 {
        let mut response = match session.getnext(&current) {
            Ok(response) => response,
            Err(_) => break,
        };

        let Some((name, value)) = response.varbinds.next() else {
            break;
        };

        let mut oid_buf = [0u32; 128];
        let oid = match name.read_name(&mut oid_buf) {
            Ok(oid) => oid,
            Err(_) => break,
        };

        if !oid.starts_with(prefix) {
            break;
        }

        let parsed = match value {
            Value::Integer(value) => Some(value as u32),
            Value::Unsigned32(value) => Some(value),
            Value::Counter32(value) => Some(value),
            Value::Timeticks(value) => Some(value),
            _ => None,
        };

        if let Some(value) = parsed {
            return Ok(Some(value));
        }

        current = oid.to_vec();
    }

    Ok(None)
}

fn query_table_first_u64(session: &mut SyncSession, prefix: &[u32]) -> Result<Option<u64>, AppError> {
    let mut current = prefix.to_vec();

    for _ in 0..64 {
        let mut response = match session.getnext(&current) {
            Ok(response) => response,
            Err(_) => break,
        };

        let Some((name, value)) = response.varbinds.next() else {
            break;
        };

        let mut oid_buf = [0u32; 128];
        let oid = match name.read_name(&mut oid_buf) {
            Ok(oid) => oid,
            Err(_) => break,
        };

        if !oid.starts_with(prefix) {
            break;
        }

        let parsed = match value {
            Value::Integer(value) => Some(value as u64),
            Value::Unsigned32(value) => Some(value as u64),
            Value::Counter32(value) => Some(value as u64),
            Value::Counter64(value) => Some(value),
            Value::Timeticks(value) => Some(value as u64),
            _ => None,
        };

        if let Some(value) = parsed {
            return Ok(Some(value));
        }

        current = oid.to_vec();
    }

    Ok(None)
}

fn query_hr_storage_memory_usage(session: &mut SyncSession) -> Result<(Option<u32>, Vec<SnmpOidProbe>), AppError> {
    let storage_type_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 2];
    let storage_size_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 5];
    let storage_used_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 6];
    let ram_type_oid = "1.3.6.1.2.1.25.2.1.2";

    let mut current = storage_type_prefix.to_vec();
    let mut probes = Vec::new();

    for _ in 0..64 {
        let mut response = match session.getnext(&current) {
            Ok(response) => response,
            Err(_) => break,
        };

        let Some((name, value)) = response.varbinds.next() else {
            break;
        };

        let mut oid_buf = [0u32; 128];
        let oid = match name.read_name(&mut oid_buf) {
            Ok(oid) => oid,
            Err(_) => break,
        };

        if !oid.starts_with(&storage_type_prefix) {
            break;
        }

        let Some(index) = oid.last().copied() else {
            current = oid.to_vec();
            continue;
        };

        let value_oid = match value {
            Value::ObjectIdentifier(ref obj) => obj.to_string(),
            _ => String::new(),
        };

        if value_oid == ram_type_oid {
            let size_oid = storage_size_prefix.iter().copied().chain([index]).collect::<Vec<_>>();
            let used_oid = storage_used_prefix.iter().copied().chain([index]).collect::<Vec<_>>();
            let size = query_u64(session, &size_oid).ok().filter(|v| *v > 0);
            let used = query_u64(session, &used_oid).ok();
            let usage = match (size, used) {
                (Some(size), Some(used)) if used <= size && size > 0 => calculate_percentage_u64(used, size),
                _ => None,
            };

            probes.push(SnmpOidProbe {
                label: format!("hrStorage RAM (index {})", index),
                oid: format!("{}.{}", storage_used_prefix.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("."), index),
                value: usage,
                selected: usage.is_some(),
                status: if usage.is_some() { "ok" } else { "n/a" }.to_string(),
            });

            return Ok((usage, probes));
        }

        fn calculate_percentage_u64(used: u64, total: u64) -> Option<u32> {
            if total == 0 || used > total {
                return None;
            }

            let percentage = ((used as u128) * 100) / (total as u128);
            u32::try_from(percentage).ok()
        }

        current = oid.to_vec();
    }

    Ok((None, probes))
}

struct HardwareInventory {
    sensors: Vec<SnmpHardwareSensor>,
    probes: Vec<SnmpOidProbe>,
}

impl SnmpClient {
    fn query_hardware_inventory_with_session(&self, session: &mut SyncSession) -> Result<HardwareInventory, AppError> {
        query_hardware_inventory_with_session(session)
    }

    fn query_hardware_inventory(&self, device: &DeviceConfig) -> Result<HardwareInventory, AppError> {
        let mut session = self.open_session(device)?;
        query_hardware_inventory_with_session(&mut session)
    }
}

fn query_hardware_inventory_with_session(session: &mut SyncSession) -> Result<HardwareInventory, AppError> {
    let physical_names = query_table_strings(session, &[1, 3, 6, 1, 2, 1, 47, 1, 1, 1, 1, 7])?;
    let physical_descrs = query_table_strings(session, &[1, 3, 6, 1, 2, 1, 47, 1, 1, 1, 1, 2])?;
    let physical_class = query_table_i64_values(session, &[1, 3, 6, 1, 2, 1, 47, 1, 1, 1, 1, 5])?;

    let sensor_names = query_table_strings(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 1, 2])?;
    let sensor_values = query_table_i64_values(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 4])?;
    let sensor_statuses = query_table_i64_values(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 5])?;

    let physical_name_map: HashMap<u32, String> = physical_names.into_iter().collect();
    let physical_descr_map: HashMap<u32, String> = physical_descrs.into_iter().collect();
    let physical_class_map: HashMap<u32, i64> = physical_class.into_iter().collect();

    let sensor_name_map: HashMap<u32, String> = sensor_names.into_iter().collect();
    let sensor_value_map: HashMap<u32, i64> = sensor_values.into_iter().collect();
    let sensor_status_map: HashMap<u32, i64> = sensor_statuses.into_iter().collect();

    let mut probes = vec![
        SnmpOidProbe {
            label: "ENTITY-SENSOR-MIB entPhySensorValue".to_string(),
            oid: "1.3.6.1.2.1.99.1.1.1.4".to_string(),
            value: u32::try_from(sensor_value_map.len()).ok(),
            selected: !sensor_value_map.is_empty(),
            status: if sensor_value_map.is_empty() { "n/a" } else { "ok" }.to_string(),
        },
        SnmpOidProbe {
            label: "ENTITY-SENSOR-MIB entPhySensorStatus".to_string(),
            oid: "1.3.6.1.2.1.99.1.1.1.5".to_string(),
            value: u32::try_from(sensor_status_map.len()).ok(),
            selected: !sensor_status_map.is_empty(),
            status: if sensor_status_map.is_empty() { "n/a" } else { "ok" }.to_string(),
        },
        SnmpOidProbe {
            label: "ENTITY-MIB entPhysicalName".to_string(),
            oid: "1.3.6.1.2.1.47.1.1.1.1.7".to_string(),
            value: u32::try_from(physical_name_map.len()).ok(),
            selected: !physical_name_map.is_empty(),
            status: if physical_name_map.is_empty() { "n/a" } else { "ok" }.to_string(),
        },
        SnmpOidProbe {
            label: "ENTITY-MIB entPhysicalClass".to_string(),
            oid: "1.3.6.1.2.1.47.1.1.1.1.5".to_string(),
            value: u32::try_from(physical_class_map.len()).ok(),
            selected: !physical_class_map.is_empty(),
            status: if physical_class_map.is_empty() { "n/a" } else { "ok" }.to_string(),
        },
    ];

    let mut sensors = Vec::new();
    for index in sensor_value_map.keys().copied().collect::<Vec<_>>() {
        let raw_name = sensor_name_map
            .get(&index)
            .cloned()
            .or_else(|| physical_name_map.get(&index).cloned())
            .or_else(|| physical_descr_map.get(&index).cloned())
            .unwrap_or_else(|| format!("sensor-{}", index));
        let sensor_type = classify_hardware_sensor(&raw_name);
        let unit = sensor_unit_for_type(&sensor_type);
        let value = sensor_value_map.get(&index).copied();
        let status = sensor_status_map.get(&index).copied();
        sensors.push(SnmpHardwareSensor {
            index,
            name: raw_name,
            sensor_type,
            source: "entity-sensor".to_string(),
            oid: format!("1.3.6.1.2.1.99.1.1.1.4.{}", index),
            value,
            unit,
            status,
            is_alarm: matches!(status, Some(3) | Some(4) | Some(5)),
        });
    }

    for index in physical_name_map
        .keys()
        .copied()
        .filter(|idx| !sensor_value_map.contains_key(idx))
        .collect::<Vec<_>>()
    {
        let raw_name = physical_name_map
            .get(&index)
            .cloned()
            .or_else(|| physical_descr_map.get(&index).cloned())
            .unwrap_or_else(|| format!("component-{}", index));
        let class = physical_class_map.get(&index).copied();
        let sensor_type = physical_class_label(class);
        sensors.push(SnmpHardwareSensor {
            index,
            name: raw_name,
            sensor_type,
            source: "entity-physical".to_string(),
            oid: format!("1.3.6.1.2.1.47.1.1.1.1.7.{}", index),
            value: None,
            unit: None,
            status: class,
            is_alarm: false,
        });
    }

    sensors.sort_by(|a, b| a.index.cmp(&b.index).then(a.source.cmp(&b.source)));
    if sensors.is_empty() {
        probes.push(SnmpOidProbe {
            label: "Hardware sensor tables".to_string(),
            oid: "1.3.6.1.2.1.99 / 1.3.6.1.2.1.47".to_string(),
            value: Some(0),
            selected: false,
            status: "n/a".to_string(),
        });
    }

    Ok(HardwareInventory { sensors, probes })
}

fn classify_hardware_sensor(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("temp") {
        "temperature".to_string()
    } else if lower.contains("fan") {
        "fan".to_string()
    } else if lower.contains("psu") || lower.contains("power") {
        "power".to_string()
    } else if lower.contains("battery") {
        "battery".to_string()
    } else {
        "sensor".to_string()
    }
}

fn physical_class_label(class: Option<i64>) -> String {
    match class {
        Some(3) => "chassis".to_string(),
        Some(4) => "backplane".to_string(),
        Some(5) => "container".to_string(),
        Some(6) => "power".to_string(),
        Some(7) => "fan".to_string(),
        Some(8) => "sensor".to_string(),
        Some(9) => "module".to_string(),
        Some(10) => "port".to_string(),
        Some(v) => format!("class-{}", v),
        None => "component".to_string(),
    }
}

fn sensor_unit_for_type(sensor_type: &str) -> Option<String> {
    match sensor_type {
        "temperature" => Some("°C".to_string()),
        "fan" => Some("rpm".to_string()),
        "power" => Some("V".to_string()),
        "battery" => Some("%".to_string()),
        _ => None,
    }
}

fn calculate_percentage_u64(used: u64, total: u64) -> Option<u32> {
    if total == 0 || used > total {
        return None;
    }

    let percentage = ((used as u128) * 100) / (total as u128);
    u32::try_from(percentage).ok()
}

fn vendor_name_from_enterprise_id(enterprise_id: u32) -> Option<&'static str> {
    match enterprise_id {
        9 => Some("Cisco"),
        1182 => Some("Yamaha"),
        2078 => Some("Allied Telesis"),
        _ => None,
    }
}

fn oid_to_string(oid: &[u32]) -> String {
    oid.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(".")
}

#[derive(Debug, Clone)]
pub struct SnmpDeviceInfo {
    pub sys_name: String,
    pub sys_descr: String,
    pub cpu_usage: Option<u32>,
    pub memory_usage: Option<u32>,
    pub hardware_sensors: Vec<SnmpHardwareSensor>,
}

#[derive(Debug, Clone)]
pub struct SnmpHardwareSensor {
    pub index: u32,
    pub name: String,
    pub sensor_type: String,
    pub source: String,
    pub oid: String,
    pub value: Option<i64>,
    pub unit: Option<String>,
    pub status: Option<i64>,
    pub is_alarm: bool,
}

#[derive(Debug, Clone)]
pub struct SnmpOidProbe {
    pub label: String,
    pub oid: String,
    pub value: Option<u32>,
    pub selected: bool,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct SnmpInterfaceDiagnostic {
    pub if_index: u32,
    pub if_name: String,
    pub link_status: String,
}

#[derive(Debug, Clone)]
pub struct SnmpDiagnostics {
    pub sys_name: String,
    pub sys_descr: String,
    pub sys_object_id: Option<String>,
    pub enterprise_id: Option<u32>,
    pub vendor_name: Option<String>,
    pub cpu_usage: Option<u32>,
    pub memory_usage: Option<u32>,
    pub cpu_probes: Vec<SnmpOidProbe>,
    pub memory_probes: Vec<SnmpOidProbe>,
    pub interfaces: Vec<SnmpInterfaceDiagnostic>,
    pub hardware_sensors: Vec<SnmpHardwareSensor>,
    pub hardware_probes: Vec<SnmpOidProbe>,
}

#[cfg(test)]
mod tests {
    use super::calculate_percentage_u64;

    #[test]
    fn calculates_percentage_with_large_values() {
        assert_eq!(calculate_percentage_u64(3_000_000_000, 4_000_000_000), Some(75));
    }

    #[test]
    fn returns_none_for_zero_total() {
        assert_eq!(calculate_percentage_u64(0, 0), None);
    }
}
