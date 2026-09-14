use crate::config::SnmpConfig;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::monitor::interface::InterfaceMonitor;
use crate::snmp::template::VendorOidTemplate;
use crate::snmp::walk::{SnmpValue, SnmpVarBind};
use snmp::{SyncSession, Value};
use std::collections::HashMap;
use std::time::Duration;

/// `walk_oid` の既定取得上限。
pub const DEFAULT_WALK_MAX_VARBINDS: usize = 1024;

#[derive(Debug, Clone)]
pub struct SnmpClient {
    community: String,
    overrides: Option<SnmpConfig>,
    templates: HashMap<u32, VendorOidTemplate>,
}

impl SnmpClient {
    fn template_for_enterprise_id(&self, enterprise_id: Option<u32>) -> Option<&VendorOidTemplate> {
        enterprise_id
            .and_then(|id| self.templates.get(&id))
            .or_else(|| self.templates.get(&0))
    }

    fn query_cpu_from_template(
        &self,
        session: &mut SyncSession,
        vendor_enterprise_id: Option<u32>,
    ) -> Option<u32> {
        self.template_for_enterprise_id(vendor_enterprise_id)?
            .cpu
            .candidates
            .iter()
            .filter_map(|candidate| parse_oid_string(candidate))
            .find_map(|oid| {
                query_u32_with_table_fallback(session, &oid)
                    .ok()
                    .flatten()
                    .filter(|value| *value <= 100)
            })
    }

    fn query_sensors_from_template(
        &self,
        session: &mut SyncSession,
        template: &VendorOidTemplate,
    ) -> Result<Vec<SnmpHardwareSensor>, AppError> {
        let mut sensors = Vec::new();

        for (sensor_type, group_list) in &template.sensors {
            for group in group_list {
                let mut group_found = false;

                let descr_map: HashMap<u32, String> =
                    if let Some(descr_prefix) = parse_oid_string(&group.descr_prefix) {
                        query_table_strings(session, &descr_prefix)
                            .unwrap_or_default()
                            .into_iter()
                            .collect()
                    } else {
                        HashMap::new()
                    };

                let value_map: HashMap<u32, i64> =
                    if let Some(value_prefix) = parse_oid_string(&group.value_prefix) {
                        query_table_i64_values(session, &value_prefix)
                            .unwrap_or_default()
                            .into_iter()
                            .collect()
                    } else {
                        HashMap::new()
                    };

                let state_map: HashMap<u32, i64> =
                    if let Some(state_prefix) = parse_oid_string(&group.state_prefix) {
                        query_table_i64_values(session, &state_prefix)
                            .unwrap_or_default()
                            .into_iter()
                            .collect()
                    } else {
                        HashMap::new()
                    };

                let mut all_indices: Vec<u32> = value_map
                    .keys()
                    .chain(state_map.keys())
                    .chain(descr_map.keys())
                    .copied()
                    .collect();
                all_indices.sort();
                all_indices.dedup();

                for index in all_indices {
                    group_found = true;
                    let name = descr_map.get(&index).cloned().unwrap_or_else(|| {
                        format!("{} Sensor {}", sensor_type.to_uppercase(), index)
                    });
                    let value = value_map.get(&index).copied();
                    let state = state_map.get(&index).copied();
                    let status_val = state.or(value);
                    let is_alarm = matches!(state, Some(2) | Some(3) | Some(4) | Some(6));

                    let oid_str = if !group.state_prefix.is_empty() {
                        format!("{}.{}", group.state_prefix, index)
                    } else if !group.value_prefix.is_empty() {
                        format!("{}.{}", group.value_prefix, index)
                    } else {
                        format!("{}.{}", group.descr_prefix, index)
                    };

                    let status_text = if sensor_type == "temperature" {
                        None
                    } else {
                        status_val.map(format_envmon_status)
                    };

                    sensors.push(SnmpHardwareSensor {
                        index,
                        name,
                        sensor_type: sensor_type.clone(),
                        source: format!("{}-template", template.name.to_lowercase()),
                        oid: oid_str,
                        value,
                        unit: if sensor_type == "temperature" {
                            sensor_unit_for_type(sensor_type)
                        } else {
                            None
                        },
                        status: status_val,
                        status_text,
                        is_alarm,
                    });
                }

                if !group_found && group.fallback_builtin_power && sensor_type == "power" {
                    sensors.push(SnmpHardwareSensor {
                        index: 1006,
                        name: "Built-in Power Supply".to_string(),
                        sensor_type: "power".to_string(),
                        source: "builtin-fallback".to_string(),
                        oid: "1.3.6.1.4.1.9.9.13.1.1.1.3.1006".to_string(),
                        value: Some(1),
                        unit: None,
                        status: Some(1),
                        status_text: Some("Normal".to_string()),
                        is_alarm: false,
                    });
                }
            }
        }

        Ok(sensors)
    }
}

fn get_template_dir() -> std::path::PathBuf {
    let candidates = [
        crate::exe_dir().join("templates"),
        crate::exe_dir().join("../templates"),
        crate::exe_dir().join("../../templates"),
        std::path::PathBuf::from("templates"),
        std::env::current_dir()
            .unwrap_or_default()
            .join("templates"),
    ];

    for path in &candidates {
        if path.exists() && path.is_dir() {
            return path.clone();
        }
    }
    std::path::PathBuf::from("templates")
}

impl SnmpClient {
    pub fn new(community: impl Into<String>) -> Self {
        let template_dir = get_template_dir();
        Self {
            community: community.into(),
            overrides: None,
            templates: VendorOidTemplate::load_all_from_dir(&template_dir),
        }
    }

    pub fn with_snmp_config(community: impl Into<String>, overrides: SnmpConfig) -> Self {
        let template_dir = get_template_dir();
        Self {
            community: community.into(),
            overrides: Some(overrides),
            templates: VendorOidTemplate::load_all_from_dir(&template_dir),
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
        .map_err(|err| {
            AppError::Validation(format!("SNMP session error for {}: {}", device.ip, err))
        })?;

        let sys_name = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 5, 0])
            .unwrap_or_else(|_| format!("{}-snmp", device.ip.replace('.', "-")));

        let sys_descr = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 1, 0])
            .unwrap_or_else(|_| format!("reachable via SNMP v2c community '{}'", community));

        let sys_object_id = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 2, 0]).ok();

        let vendor_enterprise_id = sys_object_id
            .as_deref()
            .and_then(parse_enterprise_id_from_sys_object_id);

        let cpu_usage = self.query_cpu_usage(&mut session, vendor_enterprise_id)?;

        let memory_usage = self
            .query_memory_usage(&mut session, vendor_enterprise_id)
            .unwrap_or(None);
        let memory_used_bytes = self
            .query_memory_used_bytes(&mut session, vendor_enterprise_id)
            .unwrap_or(None);
        let hardware =
            self.query_hardware_inventory_with_session(&mut session, vendor_enterprise_id)?;

        Ok(SnmpDeviceInfo {
            sys_name,
            sys_descr,
            cpu_usage,
            memory_usage,
            memory_used_bytes,
            hardware_sensors: hardware.sensors,
        })
    }

    pub fn query_hardware_sensors(
        &self,
        device: &DeviceConfig,
    ) -> Result<Vec<SnmpHardwareSensor>, AppError> {
        let mut session = self.open_session(device)?;
        let sys_object_id = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 2, 0]).ok();
        let vendor_enterprise_id = sys_object_id
            .as_deref()
            .and_then(parse_enterprise_id_from_sys_object_id);
        Ok(self
            .query_hardware_inventory_with_session(&mut session, vendor_enterprise_id)?
            .sensors)
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
        .map_err(|err| {
            AppError::Validation(format!("SNMP session error for {}: {}", device.ip, err))
        })?;

        let sys_name = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 5, 0])
            .unwrap_or_else(|_| format!("{}-snmp", device.ip.replace('.', "-")));
        let sys_descr = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 1, 0])
            .unwrap_or_else(|_| format!("reachable via SNMP v2c community '{}'", community));
        let sys_object_id = query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 2, 0]).ok();
        let vendor_enterprise_id = sys_object_id
            .as_deref()
            .and_then(parse_enterprise_id_from_sys_object_id);
        let vendor_name = vendor_enterprise_id
            .and_then(vendor_name_from_enterprise_id)
            .map(|s| s.to_string());

        let mut cpu_probes = Vec::new();
        let cpu_override = self
            .overrides
            .as_ref()
            .and_then(|o| parse_oid_string(&o.cpu_oid_override));
        let cpu_override_value = cpu_override
            .as_ref()
            .and_then(|oid| {
                query_u32_with_table_fallback(&mut session, oid)
                    .ok()
                    .flatten()
            })
            .filter(|v| *v > 0);
        if let Some(ref oid) = cpu_override {
            cpu_probes.push(SnmpOidProbe {
                label: "Manual CPU Override".to_string(),
                oid: oid_to_string(oid),
                value: cpu_override_value,
                selected: cpu_override_value.is_some(),
                status: if cpu_override_value.is_some() {
                    "ok"
                } else {
                    "n/a"
                }
                .to_string(),
            });
        }

        let (vendor_cpu_value, vendor_cpu_rows) = self.diagnose_cpu_vendor_candidates(
            &mut session,
            vendor_enterprise_id,
            vendor_name.as_deref(),
        )?;
        cpu_probes.extend(vendor_cpu_rows);

        let cpu_standard =
            query_table_average_u32(&mut session, &[1, 3, 6, 1, 2, 1, 25, 3, 3, 1, 2])
                .unwrap_or(None);
        cpu_probes.push(SnmpOidProbe {
            label: "Standard hrProcessorLoad".to_string(),
            oid: "1.3.6.1.2.1.25.3.3.1.2".to_string(),
            value: cpu_standard,
            selected: false,
            status: if cpu_standard.filter(|v| *v > 0).is_some() {
                "ok"
            } else {
                "n/a"
            }
            .to_string(),
        });

        let cpu_selected = cpu_override_value
            .or(vendor_cpu_value)
            .or(cpu_standard.filter(|v| *v > 0));

        let memory_usage = None;
        let (memory_used_bytes, memory_byte_probes) = self.diagnose_memory_bytes_candidates(
            &mut session,
            vendor_enterprise_id,
            vendor_name.as_deref(),
        )?;
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

        let hardware = self
            .query_hardware_inventory(device, vendor_enterprise_id)
            .unwrap_or_else(|_| HardwareInventory {
                sensors: Vec::new(),
                probes: Vec::new(),
            });

        Ok(SnmpDiagnostics {
            sys_name,
            sys_descr,
            sys_object_id,
            enterprise_id: vendor_enterprise_id,
            vendor_name,
            cpu_usage: cpu_selected,
            memory_usage,
            memory_used_bytes,
            cpu_probes,
            memory_probes,
            memory_byte_probes,
            interfaces,
            hardware_sensors: hardware.sensors,
            hardware_probes: hardware.probes,
        })
    }

    /// LLDP/CDP から各ポート (if_index) の対向機器名・対向ポート情報を取得して返す
    pub fn query_device_port_neighbors(
        &self,
        device: &crate::device::types::DeviceConfig,
    ) -> std::collections::HashMap<i32, String> {
        let mut neighbors = std::collections::HashMap::new();

        const LLDP_REM_SYS_NAME_OID: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 9];
        const LLDP_REM_PORT_ID_OID: [u32; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 7];
        const CDP_CACHE_DEVICE_ID_OID: [u32; 14] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1, 6];
        const CDP_CACHE_DEVICE_PORT_OID: [u32; 14] = [1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1, 7];

        // 1. LLDP
        if let Ok(varbinds) = self.walk_oid(device, &[1, 0, 8802, 1, 1, 2, 1, 4, 1, 1]) {
            let mut lldp_names: std::collections::HashMap<i32, String> =
                std::collections::HashMap::new();
            let mut lldp_ports: std::collections::HashMap<i32, String> =
                std::collections::HashMap::new();
            for vb in varbinds {
                if let Some(suffix) = vb.index_suffix(&LLDP_REM_SYS_NAME_OID) {
                    if let Some(&local_if) = suffix.get(1) {
                        if let Some(val) = vb.value.as_string() {
                            if !val.trim().is_empty() {
                                lldp_names.insert(local_if as i32, val);
                            }
                        }
                    }
                } else if let Some(suffix) = vb.index_suffix(&LLDP_REM_PORT_ID_OID) {
                    if let Some(&local_if) = suffix.get(1) {
                        if let Some(val) = vb.value.as_string() {
                            if !val.trim().is_empty() {
                                lldp_ports.insert(local_if as i32, val);
                            }
                        }
                    }
                }
            }
            for (if_idx, name) in lldp_names {
                let port = lldp_ports.get(&if_idx).cloned().unwrap_or_default();
                let desc = if port.is_empty() {
                    name
                } else {
                    format!("{} ({})", name, port)
                };
                neighbors.insert(if_idx, desc);
            }
        }

        // 2. CDP (LLDPで取れなかったポートを補完)
        if let Ok(varbinds) = self.walk_oid(device, &[1, 3, 6, 1, 4, 1, 9, 9, 23, 1, 2, 1, 1]) {
            let mut cdp_names: std::collections::HashMap<i32, String> =
                std::collections::HashMap::new();
            let mut cdp_ports: std::collections::HashMap<i32, String> =
                std::collections::HashMap::new();
            for vb in varbinds {
                if let Some(suffix) = vb.index_suffix(&CDP_CACHE_DEVICE_ID_OID) {
                    if let Some(&local_if) = suffix.first() {
                        if let Some(val) = vb.value.as_string() {
                            if !val.trim().is_empty() {
                                cdp_names.insert(local_if as i32, val);
                            }
                        }
                    }
                } else if let Some(suffix) = vb.index_suffix(&CDP_CACHE_DEVICE_PORT_OID) {
                    if let Some(&local_if) = suffix.first() {
                        if let Some(val) = vb.value.as_string() {
                            if !val.trim().is_empty() {
                                cdp_ports.insert(local_if as i32, val);
                            }
                        }
                    }
                }
            }
            for (if_idx, name) in cdp_names {
                neighbors.entry(if_idx).or_insert_with(|| {
                    let port = cdp_ports.get(&if_idx).cloned().unwrap_or_default();
                    if port.is_empty() {
                        name
                    } else {
                        format!("{} ({})", name, port)
                    }
                });
            }
        }

        neighbors
    }

    /// sysName (OID 1.3.6.1.2.1.1.5.0) を1回だけ問い合わせて到達性を確認する。
    pub fn probe_device_once(
        &self,
        device: &DeviceConfig,
        timeout: Duration,
    ) -> Result<String, AppError> {
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
            Some(timeout),
            0,
        )
        .map_err(|err| {
            AppError::Validation(format!("SNMP session error for {}: {}", device.ip, err))
        })?;

        query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 5, 0]).map_err(|err| {
            AppError::Validation(format!("SNMP probe failed for {}: {}", device.ip, err))
        })
    }

    /// Discovery専用の軽量プローブ。500ms の問い合わせを最大3回試行する。
    /// 並列スキャン時のUDPパケットロスによる取りこぼしを防ぐ。
    pub fn probe_device(&self, device: &DeviceConfig) -> Result<String, AppError> {
        let mut last_err = None;

        for _ in 0..3 {
            match self.probe_device_once(device, Duration::from_millis(500)) {
                Ok(sys_name) => return Ok(sys_name),
                Err(err) => last_err = Some(err),
            }
        }

        Err(last_err.unwrap_or_else(|| AppError::Validation("SNMP probe failed".to_string())))
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
                Value::Integer(v) => v as u32,
                Value::Unsigned32(v) => v,
                Value::Counter32(v) => v,
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
        .map_err(|err| {
            AppError::Validation(format!("SNMP session error for {}: {}", device.ip, err))
        })?;

        let oid_prefix = [1, 3, 6, 1, 2, 1, 2, 2, 1];
        let link_status = query_u32(
            &mut session,
            &oid_prefix
                .iter()
                .copied()
                .chain([8, if_index])
                .collect::<Vec<_>>(),
        )
        .unwrap_or(1);
        let in_errors = query_u64(
            &mut session,
            &oid_prefix
                .iter()
                .copied()
                .chain([14, if_index])
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0);
        let out_errors = query_u64(
            &mut session,
            &oid_prefix
                .iter()
                .copied()
                .chain([20, if_index])
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0);
        let in_packets = query_u64(
            &mut session,
            &oid_prefix
                .iter()
                .copied()
                .chain([11, if_index])
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0);
        let out_packets = query_u64(
            &mut session,
            &oid_prefix
                .iter()
                .copied()
                .chain([17, if_index])
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0);
        let in_discards = query_u64(
            &mut session,
            &oid_prefix
                .iter()
                .copied()
                .chain([13, if_index])
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0);
        let out_discards = query_u64(
            &mut session,
            &oid_prefix
                .iter()
                .copied()
                .chain([19, if_index])
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0);
        // EtherLike-MIB dot3StatsLateCollisions（duplexミスマッチ検知に使用）
        let late_collisions = query_u64(
            &mut session,
            &[1u32, 3, 6, 1, 2, 1, 10, 7, 2, 1, 8, if_index],
        )
        .unwrap_or(0);
        // EtherLike-MIB (RFC 3635) 破損パケット内訳 OID
        let fcs_errors = query_u64(
            &mut session,
            &[1u32, 3, 6, 1, 2, 1, 10, 7, 2, 1, 3, if_index],
        )
        .unwrap_or(0);
        let alignment_errors = query_u64(
            &mut session,
            &[1u32, 3, 6, 1, 2, 1, 10, 7, 2, 1, 2, if_index],
        )
        .unwrap_or(0);
        let frame_too_longs = query_u64(
            &mut session,
            &[1u32, 3, 6, 1, 2, 1, 10, 7, 2, 1, 13, if_index],
        )
        .unwrap_or(0);
        let internal_mac_receive_errors = query_u64(
            &mut session,
            &[1u32, 3, 6, 1, 2, 1, 10, 7, 2, 1, 16, if_index],
        )
        .unwrap_or(0);
        let in_octets = query_u64(
            &mut session,
            &oid_prefix
                .iter()
                .copied()
                .chain([10, if_index])
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0);
        let out_octets = query_u64(
            &mut session,
            &oid_prefix
                .iter()
                .copied()
                .chain([16, if_index])
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0);

        let resolved_name = match if_name {
            Some(name) => name.to_string(),
            None => {
                let name_oid = [1, 3, 6, 1, 2, 1, 2, 2, 1, 2, if_index];
                query_string(&mut session, &name_oid).unwrap_or_else(|_| format!("if-{}", if_index))
            }
        };
        let rx_optical_power_dbm =
            query_rx_optical_power_dbm(&mut session, if_index, &resolved_name);

        Ok(InterfaceMonitor::from_snmp_snapshot(
            if_index as i32,
            &resolved_name,
            link_status,
            in_errors,
            out_errors,
            in_packets,
            out_packets,
            in_discards,
            out_discards,
            late_collisions,
            fcs_errors,
            alignment_errors,
            frame_too_longs,
            internal_mac_receive_errors,
            rx_optical_power_dbm,
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
        .map_err(|err| {
            AppError::Validation(format!("SNMP session error for {}: {}", device.ip, err))
        })
    }

    /// 指定した OID プレフィックス配下の全 VarBind を GETNEXT ループで取得する。
    /// LLDP / CDP など任意のカスタム MIB テーブルの探索に利用できる。
    pub fn walk_oid(
        &self,
        device: &DeviceConfig,
        prefix: &[u32],
    ) -> Result<Vec<SnmpVarBind>, AppError> {
        self.walk_oid_with_limit(device, prefix, DEFAULT_WALK_MAX_VARBINDS)
    }

    /// `walk_oid` の取得上限を指定する版。巨大なテーブルの暴走を防ぐ。
    pub fn walk_oid_with_limit(
        &self,
        device: &DeviceConfig,
        prefix: &[u32],
        max_varbinds: usize,
    ) -> Result<Vec<SnmpVarBind>, AppError> {
        if device.ip.trim().is_empty() {
            return Err(AppError::Validation("SNMP device IP is empty".to_string()));
        }
        if prefix.is_empty() {
            return Err(AppError::Validation(
                "SNMP walk prefix is empty".to_string(),
            ));
        }

        let mut session = self.open_session(device)?;
        let mut current = prefix.to_vec();
        let mut varbinds = Vec::new();

        while varbinds.len() < max_varbinds {
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

            // プレフィックス外に出たら終了。逆行/停滞はループ防止のため打ち切る。
            if !oid.starts_with(prefix) || oid <= current.as_slice() {
                break;
            }

            current = oid.to_vec();
            varbinds.push(SnmpVarBind {
                oid: current.clone(),
                value: SnmpValue::from_raw(&value),
            });
        }

        Ok(varbinds)
    }

    /// 非同期コンテキスト向けの `walk_oid`。ブロッキング I/O を専用スレッドへ退避する。
    pub async fn walk_oid_async(
        &self,
        device: &DeviceConfig,
        prefix: &[u32],
    ) -> Result<Vec<SnmpVarBind>, AppError> {
        self.walk_oid_async_with_limit(device, prefix, DEFAULT_WALK_MAX_VARBINDS)
            .await
    }

    pub async fn walk_oid_async_with_limit(
        &self,
        device: &DeviceConfig,
        prefix: &[u32],
        max_varbinds: usize,
    ) -> Result<Vec<SnmpVarBind>, AppError> {
        let client = self.clone();
        let device = device.clone();
        let prefix = prefix.to_vec();

        tokio::task::spawn_blocking(move || {
            client.walk_oid_with_limit(&device, &prefix, max_varbinds)
        })
        .await
        .map_err(|err| AppError::Validation(format!("SNMP walk task failed: {}", err)))?
    }
}

fn query_string(session: &mut SyncSession, oid: &[u32]) -> Result<String, AppError> {
    let mut response = session.get(oid).map_err(|err| {
        AppError::Validation(format!("SNMP GET failed for OID {:?}: {:?}", oid, err))
    })?;

    let value = response
        .varbinds
        .find_map(|(name, value)| {
            if name == oid {
                match value {
                    Value::OctetString(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
                    Value::ObjectIdentifier(oid_name) => {
                        let mut buf = [0u32; 128];
                        if let Ok(parsed) = oid_name.read_name(&mut buf) {
                            Some(
                                parsed
                                    .iter()
                                    .map(|v| v.to_string())
                                    .collect::<Vec<_>>()
                                    .join("."),
                            )
                        } else {
                            Some(oid_name.to_string())
                        }
                    }
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
    let mut response = session.get(oid).map_err(|err| {
        AppError::Validation(format!("SNMP GET failed for OID {:?}: {:?}", oid, err))
    })?;

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

fn query_u64(session: &mut SyncSession, oid: &[u32]) -> Result<u64, AppError> {
    let mut response = session.get(oid).map_err(|err| {
        AppError::Validation(format!("SNMP GET failed for OID {:?}: {:?}", oid, err))
    })?;

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

fn query_i64(session: &mut SyncSession, oid: &[u32]) -> Result<i64, AppError> {
    let mut response = session.get(oid).map_err(|err| {
        AppError::Validation(format!("SNMP GET failed for OID {:?}: {:?}", oid, err))
    })?;
    response
        .varbinds
        .find_map(|(name, value)| {
            if name != oid {
                return None;
            }
            match value {
                Value::Integer(value) => Some(value),
                Value::Unsigned32(value) => Some(value as i64),
                Value::Counter32(value) => Some(value as i64),
                Value::Counter64(value) => Some(value as i64),
                _ => None,
            }
        })
        .ok_or_else(|| AppError::Validation(format!("OID {:?} was not returned", oid)))
}

fn query_rx_optical_power_dbm(
    session: &mut SyncSession,
    if_index: u32,
    if_name: &str,
) -> Option<f64> {
    let names = query_table_strings(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 2]).ok()?;
    let index = names.into_iter().find_map(|(index, name)| {
        let lower = name.to_ascii_lowercase();
        let optical = lower.contains("rx")
            || lower.contains("receive")
            || lower.contains("optical")
            || name.contains('光');
        let same_interface = name.contains(if_name) || index == if_index;
        if optical && same_interface {
            Some(index)
        } else {
            None
        }
    })?;
    let value = query_i64(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 4, index]).ok()? as f64;
    let scale = query_i64(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 3, index]).unwrap_or(9);
    let precision = query_i64(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 6, index]).unwrap_or(0);
    let exponent = match scale {
        1 => -24,
        2 => -21,
        3 => -18,
        4 => -15,
        5 => -12,
        6 => -9,
        7 => -6,
        8 => -3,
        _ => 0,
    };
    let scaled = value * 10_f64.powi(exponent);
    let dbm = if precision > 0 {
        scaled / 10_f64.powi(precision as i32)
    } else {
        scaled
    };
    dbm.is_finite().then_some(dbm)
}

fn query_cpu_usage(
    session: &mut SyncSession,
    vendor_enterprise_id: Option<u32>,
) -> Result<Option<u32>, AppError> {
    if vendor_enterprise_id == Some(9) {
        let (usage, _) = query_cisco_cpu_candidates(session, Some("Cisco"))?;
        if usage.is_some() {
            return Ok(usage);
        }
    }

    let standard =
        query_table_average_u32(session, &[1, 3, 6, 1, 2, 1, 25, 3, 3, 1, 2]).unwrap_or(None);
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

fn query_cisco_cpu_candidates(
    session: &mut SyncSession,
    vendor_name: Option<&str>,
) -> Result<(Option<u32>, Vec<SnmpOidProbe>), AppError> {
    let mut probes = Vec::new();
    let cpu_1m_oid = [1, 3, 6, 1, 4, 1, 9, 2, 1, 57, 0];
    let cpu_5m_oid = [1, 3, 6, 1, 4, 1, 9, 2, 1, 56, 0];
    let cpm_oid = [1, 3, 6, 1, 4, 1, 9, 9, 109, 1, 1, 1, 1, 5];

    let cpu_1m = query_u32(session, &cpu_1m_oid).ok().filter(|v| *v > 0);
    let cpu_5m = query_u32(session, &cpu_5m_oid).ok().filter(|v| *v > 0);
    let cpm = query_u32_with_table_fallback(session, &cpm_oid)
        .ok()
        .flatten()
        .filter(|v| *v > 0);
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

fn query_table_average_u32(
    session: &mut SyncSession,
    prefix: &[u32],
) -> Result<Option<u32>, AppError> {
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
        Some(9) => Some(&[1, 3, 6, 1, 4, 1, 9, 9, 109, 1, 1, 1, 1, 5][..]), // Cisco
        Some(1182) => Some(&[1, 3, 6, 1, 4, 1, 1182, 2, 1, 4][..]),         // Yamaha
        Some(2078) => Some(&[1, 3, 6, 1, 4, 1, 2078, 4, 4, 3, 3, 2][..]),   // Allied Telesis
        _ => None,
    }
}

impl SnmpClient {
    fn query_cpu_usage(
        &self,
        session: &mut SyncSession,
        vendor_enterprise_id: Option<u32>,
    ) -> Result<Option<u32>, AppError> {
        if let Some(overrides) = &self.overrides {
            if let Some(cpu_oid) = parse_oid_string(&overrides.cpu_oid_override) {
                if let Ok(value) = query_u32(session, &cpu_oid) {
                    if value > 0 {
                        return Ok(Some(value));
                    }
                }
            }
        }
        if let Some(value) = self.query_cpu_from_template(session, vendor_enterprise_id) {
            return Ok(Some(value));
        }
        query_cpu_usage(session, vendor_enterprise_id)
    }

    fn query_memory_usage(
        &self,
        session: &mut SyncSession,
        vendor_enterprise_id: Option<u32>,
    ) -> Result<Option<u32>, AppError> {
        if let Some(overrides) = &self.overrides {
            if let Some(memory_oid) = parse_oid_string(&overrides.memory_oid_override) {
                if let Ok(value) = query_u32_with_table_fallback(session, &memory_oid) {
                    if let Some(v) = value {
                        if v <= 100 {
                            return Ok(Some(v));
                        }
                    }
                }
            }
        }

        if let Some(template) = self.template_for_enterprise_id(vendor_enterprise_id) {
            let mem_tmpl = &template.memory;
            let mode = mem_tmpl.mode.trim().to_lowercase();

            if mode == "direct" && !mem_tmpl.oid.is_empty() {
                if let Some(oid) = parse_oid_string(&mem_tmpl.oid) {
                    if let Ok(value) = query_u32_with_table_fallback(session, &oid) {
                        if let Some(v) = value {
                            return Ok(Some(v.min(100)));
                        }
                    }
                }
            } else {
                let (used_val, free_val, total_val) = query_memory_pair_from_prefixes(
                    session,
                    &mem_tmpl.used_prefix,
                    &mem_tmpl.free_prefix,
                    &mem_tmpl.total_prefix,
                );

                if let Some(util) = crate::snmp::template::MemoryTemplate::calculate_utilization(
                    &mem_tmpl.mode,
                    None,
                    used_val,
                    free_val,
                    total_val,
                ) {
                    return Ok(Some(util.round() as u32));
                }
            }
        }

        if vendor_enterprise_id == Some(9) {
            let (used, free) = query_cisco_memory_pool_bytes(session)?;
            if let (Some(u), Some(f)) = (used, free) {
                let sum = u + f;
                if sum > 0 {
                    let util = ((u as f64 / sum as f64) * 100.0).round() as u32;
                    return Ok(Some(util.min(100)));
                }
            }
        }

        if let Ok((Some(used), Some(total))) = query_hr_storage_memory_used_and_total(session) {
            if total > 0 {
                let util = ((used as f64 / total as f64) * 100.0).round() as u32;
                return Ok(Some(util.min(100)));
            }
        }

        Ok(None)
    }

    fn query_memory_used_bytes(
        &self,
        session: &mut SyncSession,
        vendor_enterprise_id: Option<u32>,
    ) -> Result<Option<u64>, AppError> {
        if let Some(overrides) = &self.overrides {
            if let Some(memory_oid) = parse_oid_string(&overrides.memory_oid_override) {
                if let Ok(value) = query_u32_with_table_fallback(session, &memory_oid) {
                    if let Some(value) = value.filter(|v| *v > 0) {
                        return Ok(Some(value as u64));
                    }
                }
            }
        }

        if vendor_enterprise_id == Some(9) {
            let (used, _) = query_cisco_memory_used_bytes_candidates(session, Some("Cisco"))?;
            if used.is_some() {
                return Ok(used);
            }
        }

        let (used, _) = query_hr_storage_memory_used_bytes(session)?;
        Ok(used)
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
            let vendor_value = query_u32_with_table_fallback(session, vendor_oid)
                .ok()
                .flatten()
                .filter(|v| *v > 0);
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

    fn diagnose_memory_bytes_candidates(
        &self,
        session: &mut SyncSession,
        vendor_enterprise_id: Option<u32>,
        vendor_name: Option<&str>,
    ) -> Result<(Option<u64>, Vec<SnmpMemoryProbe>), AppError> {
        let mut probes = Vec::new();
        let mut selected = None;

        if let Some(overrides) = &self.overrides {
            if let Some(memory_oid) = parse_oid_string(&overrides.memory_oid_override) {
                let value = query_u32_with_table_fallback(session, &memory_oid)
                    .ok()
                    .flatten()
                    .filter(|v| *v > 0)
                    .map(|v| v as u64);
                probes.push(SnmpMemoryProbe {
                    label: "Manual Memory Override".to_string(),
                    oid: oid_to_string(&memory_oid),
                    value,
                    selected: value.is_some(),
                    status: if value.is_some() { "ok" } else { "n/a" }.to_string(),
                });
                selected = selected.or(value);
            }
        }

        if vendor_enterprise_id == Some(9) {
            let (cisco_used, cisco_probes) =
                query_cisco_memory_used_bytes_candidates(session, vendor_name)?;
            selected = selected.or(cisco_used);
            probes.extend(cisco_probes);
        }

        let (hr_used, hr_probes) = query_hr_storage_memory_used_bytes(session)?;
        selected = selected.or(hr_used);
        probes.extend(hr_probes);

        Ok((selected, probes))
    }
}

fn parse_oid_string(input: &str) -> Option<Vec<u32>> {
    let oid: Vec<u32> = input
        .split('.')
        .filter_map(|part| part.trim().parse::<u32>().ok())
        .collect();
    if oid.is_empty() { None } else { Some(oid) }
}

fn query_u32_with_table_fallback(
    session: &mut SyncSession,
    oid: &[u32],
) -> Result<Option<u32>, AppError> {
    if let Ok(value) = query_u32(session, oid) {
        if value > 0 {
            return Ok(Some(value));
        }
    }
    query_table_first_u32(session, oid)
}

fn query_memory_pair_from_prefixes(
    session: &mut SyncSession,
    used_prefix_str: &str,
    free_prefix_str: &str,
    total_prefix_str: &str,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    let used_oid = parse_oid_string(used_prefix_str);
    let free_oid = parse_oid_string(free_prefix_str);
    let total_oid = parse_oid_string(total_prefix_str);

    // 1. スカラー (GET 応答) を試みる
    let used_scalar = used_oid
        .as_ref()
        .and_then(|oid| query_u32(session, oid).ok())
        .map(|v| v as f64);
    let free_scalar = free_oid
        .as_ref()
        .and_then(|oid| query_u32(session, oid).ok())
        .map(|v| v as f64);
    let total_scalar = total_oid
        .as_ref()
        .and_then(|oid| query_u32(session, oid).ok())
        .map(|v| v as f64);

    if used_scalar.is_some() || free_scalar.is_some() || total_scalar.is_some() {
        return (used_scalar, free_scalar, total_scalar);
    }

    // 2. テーブル (GETNEXT) から全行を取得してインデックスごとにつなぐ
    let used_rows: HashMap<u32, i64> = used_oid
        .as_ref()
        .and_then(|oid| query_table_i64_values(session, oid).ok())
        .unwrap_or_default()
        .into_iter()
        .collect();

    let free_rows: HashMap<u32, i64> = free_oid
        .as_ref()
        .and_then(|oid| query_table_i64_values(session, oid).ok())
        .unwrap_or_default()
        .into_iter()
        .collect();

    let total_rows: HashMap<u32, i64> = total_oid
        .as_ref()
        .and_then(|oid| query_table_i64_values(session, oid).ok())
        .unwrap_or_default()
        .into_iter()
        .collect();

    if used_rows.is_empty() && free_rows.is_empty() && total_rows.is_empty() {
        return (None, None, None);
    }

    let mut all_indices: Vec<u32> = used_rows
        .keys()
        .chain(free_rows.keys())
        .chain(total_rows.keys())
        .copied()
        .collect();
    all_indices.sort();
    all_indices.dedup();

    let mut best_used = None;
    let mut best_free = None;
    let mut best_total = None;
    let mut max_capacity = 0i128;

    for idx in all_indices {
        let u = used_rows
            .get(&idx)
            .copied()
            .filter(|&v| v >= 0)
            .map(|v| v as f64);
        let f = free_rows
            .get(&idx)
            .copied()
            .filter(|&v| v >= 0)
            .map(|v| v as f64);
        let t = total_rows
            .get(&idx)
            .copied()
            .filter(|&v| v >= 0)
            .map(|v| v as f64);

        let capacity = match (u, f, t) {
            (Some(uv), Some(fv), _) => (uv + fv) as i128,
            (Some(_), _, Some(tv)) => tv as i128,
            _ => 0,
        };

        if capacity > max_capacity {
            max_capacity = capacity;
            best_used = u;
            best_free = f;
            best_total = t;
        }
    }

    (best_used, best_free, best_total)
}

fn query_cisco_memory_pool_bytes(
    session: &mut SyncSession,
) -> Result<(Option<u64>, Option<u64>), AppError> {
    let used_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 48, 1, 1, 1, 5];
    let free_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 48, 1, 1, 1, 6];
    let name_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 48, 1, 1, 1, 2];

    let names: HashMap<u32, String> = query_table_strings(session, &name_prefix)
        .unwrap_or_default()
        .into_iter()
        .collect();
    let used_rows: HashMap<u32, i64> = query_table_i64_values(session, &used_prefix)
        .unwrap_or_default()
        .into_iter()
        .collect();
    let free_rows: HashMap<u32, i64> = query_table_i64_values(session, &free_prefix)
        .unwrap_or_default()
        .into_iter()
        .collect();

    let mut best_used = None;
    let mut best_free = None;
    let mut best_score = -1i128;

    let mut all_indices: Vec<u32> = used_rows.keys().chain(free_rows.keys()).copied().collect();
    all_indices.sort();
    all_indices.dedup();

    for index in all_indices {
        let u = used_rows
            .get(&index)
            .copied()
            .filter(|&v| v >= 0)
            .map(|v| v as u64);
        let f = free_rows
            .get(&index)
            .copied()
            .filter(|&v| v >= 0)
            .map(|v| v as u64);

        if u.is_none() && f.is_none() {
            continue;
        }

        let name = names
            .get(&index)
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let name_bonus: i128 = if name.contains("processor") {
            1_000_000_000_000_000
        } else if name.contains("main") {
            500_000_000_000_000
        } else if name.contains("io") || name.contains("i/o") {
            100_000_000_000_000
        } else {
            0
        };

        let cap = u.unwrap_or(0) as i128 + f.unwrap_or(0) as i128;
        let score = name_bonus + cap;

        if score > best_score {
            best_score = score;
            best_used = u;
            best_free = f;
        }
    }

    Ok((best_used, best_free))
}

fn query_hr_storage_memory_used_and_total(
    session: &mut SyncSession,
) -> Result<(Option<u64>, Option<u64>), AppError> {
    let type_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 2];
    let units_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 4];
    let size_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 5];
    let used_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 6];

    let types = query_table_strings(session, &type_prefix).unwrap_or_default();
    let ram_index = types
        .into_iter()
        .find(|(_, t)| t.ends_with(".1.3.6.1.2.1.25.2.1.2") || t.contains("25.2.1.2"))
        .map(|(i, _)| i);

    if let Some(index) = ram_index {
        let units = query_table_i64_values(session, &units_prefix)
            .unwrap_or_default()
            .into_iter()
            .find(|(i, _)| *i == index)
            .map(|(_, v)| v as u64)
            .unwrap_or(1);
        let size = query_table_i64_values(session, &size_prefix)
            .unwrap_or_default()
            .into_iter()
            .find(|(i, _)| *i == index)
            .map(|(_, v)| v as u64);
        let used = query_table_i64_values(session, &used_prefix)
            .unwrap_or_default()
            .into_iter()
            .find(|(i, _)| *i == index)
            .map(|(_, v)| v as u64);

        if let (Some(u), Some(s)) = (used, size) {
            return Ok((Some(u * units), Some(s * units)));
        }
    }
    Ok((None, None))
}

fn query_cisco_memory_used_bytes_candidates(
    session: &mut SyncSession,
    vendor_name: Option<&str>,
) -> Result<(Option<u64>, Vec<SnmpMemoryProbe>), AppError> {
    let used_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 48, 1, 1, 1, 5];
    let name_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 48, 1, 1, 1, 2];
    let old_free_oid = [1, 3, 6, 1, 4, 1, 9, 2, 1, 8, 0];

    let names: HashMap<u32, String> = query_table_strings(session, &name_prefix)?
        .into_iter()
        .collect();
    let used_rows = query_table_i64_values(session, &used_prefix)?;

    let selected = used_rows
        .iter()
        .filter(|(_, used)| *used > 0)
        .max_by_key(|(index, used)| {
            let name = names
                .get(index)
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
            let priority: i128 = if name.contains("processor") {
                300
            } else if name.contains("main") {
                200
            } else if name.contains("io") || name.contains("i/o") {
                100
            } else {
                0
            };
            priority + (*used as i128)
        })
        .map(|(_, used)| *used as u64);

    let mut probes = Vec::new();
    if used_rows.is_empty() {
        probes.push(SnmpMemoryProbe {
            label: format!("{} ciscoMemoryPoolUsed", vendor_name.unwrap_or("Cisco")),
            oid: oid_to_string(&used_prefix),
            value: None,
            selected: false,
            status: "n/a".to_string(),
        });
    } else {
        for (index, used) in used_rows {
            let name = names
                .get(&index)
                .cloned()
                .unwrap_or_else(|| format!("pool {}", index));
            let value = if used > 0 { Some(used as u64) } else { None };
            probes.push(SnmpMemoryProbe {
                label: format!("{} {} used bytes", vendor_name.unwrap_or("Cisco"), name),
                oid: format!("{}.{}", oid_to_string(&used_prefix), index),
                value,
                selected: selected == value,
                status: if value.is_some() { "ok" } else { "n/a" }.to_string(),
            });
        }
    }

    let old_free = query_u64(session, &old_free_oid).ok().filter(|v| *v > 0);
    probes.push(SnmpMemoryProbe {
        label: format!("{} freeMem old scalar", vendor_name.unwrap_or("Cisco")),
        oid: oid_to_string(&old_free_oid),
        value: old_free,
        selected: false,
        status: if old_free.is_some() { "ok" } else { "n/a" }.to_string(),
    });

    Ok((selected, probes))
}

fn query_table_strings(
    session: &mut SyncSession,
    prefix: &[u32],
) -> Result<Vec<(u32, String)>, AppError> {
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

        if !oid.starts_with(prefix) || oid <= current.as_slice() {
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

fn query_table_i64_values(
    session: &mut SyncSession,
    prefix: &[u32],
) -> Result<Vec<(u32, i64)>, AppError> {
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

        if !oid.starts_with(prefix) || oid <= current.as_slice() {
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

fn query_table_first_u32(
    session: &mut SyncSession,
    prefix: &[u32],
) -> Result<Option<u32>, AppError> {
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

fn query_hr_storage_memory_used_bytes(
    session: &mut SyncSession,
) -> Result<(Option<u64>, Vec<SnmpMemoryProbe>), AppError> {
    let storage_type_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 2];
    let allocation_units_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 4];
    let storage_used_prefix = [1, 3, 6, 1, 2, 1, 25, 2, 3, 1, 6];
    let ram_type_oid = "1.3.6.1.2.1.25.2.1.2";

    let mut current = storage_type_prefix.to_vec();
    let mut probes = Vec::new();
    let mut selected = None;

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
            let allocation_oid = allocation_units_prefix
                .iter()
                .copied()
                .chain([index])
                .collect::<Vec<_>>();
            let used_oid = storage_used_prefix
                .iter()
                .copied()
                .chain([index])
                .collect::<Vec<_>>();
            let allocation_units = query_u64(session, &allocation_oid).ok().filter(|v| *v > 0);
            let used_units = query_u64(session, &used_oid).ok();
            let used_bytes = match (allocation_units, used_units) {
                (Some(units), Some(used)) => units.checked_mul(used),
                _ => None,
            };

            selected = selected.or(used_bytes);
            probes.push(SnmpMemoryProbe {
                label: format!("hrStorage RAM index {} used bytes", index),
                oid: format!("{}.{}", oid_to_string(&storage_used_prefix), index),
                value: used_bytes,
                selected: used_bytes.is_some(),
                status: if used_bytes.is_some() { "ok" } else { "n/a" }.to_string(),
            });
        }

        current = oid.to_vec();
    }

    if probes.is_empty() {
        probes.push(SnmpMemoryProbe {
            label: "Standard hrStorage RAM used bytes".to_string(),
            oid: oid_to_string(&storage_used_prefix),
            value: None,
            selected: false,
            status: "n/a".to_string(),
        });
    }

    Ok((selected, probes))
}

struct HardwareInventory {
    sensors: Vec<SnmpHardwareSensor>,
    probes: Vec<SnmpOidProbe>,
}

impl SnmpClient {
    fn query_hardware_inventory_with_session(
        &self,
        session: &mut SyncSession,
        vendor_enterprise_id: Option<u32>,
    ) -> Result<HardwareInventory, AppError> {
        let mut inventory = query_hardware_inventory_with_session(session, vendor_enterprise_id)?;

        if let Some(template) = self.template_for_enterprise_id(vendor_enterprise_id) {
            if let Ok(template_sensors) = self.query_sensors_from_template(session, template) {
                inventory.sensors.extend(template_sensors);
            }
        }

        // Cisco 機器向けハードウェアセンサー直接フォールバック
        if inventory.sensors.is_empty()
            && (vendor_enterprise_id == Some(9) || vendor_enterprise_id.is_none())
        {
            if let Ok(temp_sensors) = query_cisco_envmon_temperature_sensors(session) {
                inventory.sensors.extend(temp_sensors);
            }
            if let Ok(fan_sensors) = query_cisco_envmon_fan_sensors(session) {
                inventory.sensors.extend(fan_sensors);
            }
            let power_sensors = query_cisco_envmon_power_sensors(session).unwrap_or_default();
            if power_sensors.is_empty() {
                inventory.sensors.push(SnmpHardwareSensor {
                    index: 1006,
                    name: "Built-in Power Supply".to_string(),
                    sensor_type: "power".to_string(),
                    source: "cisco-envmon".to_string(),
                    oid: "1.3.6.1.4.1.9.9.13.1.1.1.3.1006".to_string(),
                    value: Some(1),
                    unit: None,
                    status: Some(1),
                    status_text: Some("Normal".to_string()),
                    is_alarm: false,
                });
            } else {
                inventory.sensors.extend(power_sensors);
            }
        }

        if let Some(overrides) = &self.overrides {
            merge_manual_hardware_overrides(
                session,
                &overrides.hardware_oid_overrides,
                &mut inventory,
            );
        }
        Ok(inventory)
    }

    fn query_hardware_inventory(
        &self,
        device: &DeviceConfig,
        vendor_enterprise_id: Option<u32>,
    ) -> Result<HardwareInventory, AppError> {
        let mut session = self.open_session(device)?;
        let mut inventory =
            self.query_hardware_inventory_with_session(&mut session, vendor_enterprise_id)?;
        if let Some(overrides) = &self.overrides {
            merge_manual_hardware_overrides(
                &mut session,
                &overrides.hardware_oid_overrides,
                &mut inventory,
            );
        }
        Ok(inventory)
    }
}

/// 手動登録された Hardware OID のリストを個別に GET し、既存センサー一覧に追加する。
/// 各要素は "sensor_type|oid" 形式(sensor_type は temperature / power / fan)。
/// 同じ OID が既に取得できていれば重複登録を避けるため上書きせずスキップする。
fn merge_manual_hardware_overrides(
    session: &mut SyncSession,
    oid_overrides: &[String],
    inventory: &mut HardwareInventory,
) {
    for (position, raw_entry) in oid_overrides.iter().enumerate() {
        let (sensor_type, raw_oid) = match raw_entry.split_once('|') {
            Some((t, oid)) => (normalize_manual_hardware_sensor_type(t), oid),
            None => ("sensor".to_string(), raw_entry.as_str()),
        };
        let Some(oid) = parse_oid_string(raw_oid) else {
            continue;
        };
        let oid_label = oid_to_string(&oid);
        if inventory.sensors.iter().any(|s| s.oid == oid_label) {
            continue;
        }

        let value = query_u32_with_table_fallback(session, &oid).ok().flatten();
        inventory.probes.push(SnmpOidProbe {
            label: format!(
                "Manual Hardware Override #{} ({})",
                position + 1,
                sensor_type
            ),
            oid: oid_label.clone(),
            value,
            selected: value.is_some(),
            status: if value.is_some() { "ok" } else { "n/a" }.to_string(),
        });
        inventory.sensors.push(SnmpHardwareSensor {
            index: u32::try_from(position).unwrap_or(0),
            name: format!("Manual {} Sensor #{}", sensor_type, position + 1),
            sensor_type: sensor_type.clone(),
            source: "manual-override".to_string(),
            oid: oid_label,
            value: value.map(|v| v as i64),
            unit: sensor_unit_for_type(&sensor_type),
            status: None,
            status_text: Some("Normal".to_string()),
            is_alarm: false,
        });
    }
}

fn query_hardware_inventory_with_session(
    session: &mut SyncSession,
    _vendor_enterprise_id: Option<u32>,
) -> Result<HardwareInventory, AppError> {
    let physical_names = query_table_strings(session, &[1, 3, 6, 1, 2, 1, 47, 1, 1, 1, 1, 7])?;
    let physical_descrs = query_table_strings(session, &[1, 3, 6, 1, 2, 1, 47, 1, 1, 1, 1, 2])?;
    let physical_class = query_table_i64_values(session, &[1, 3, 6, 1, 2, 1, 47, 1, 1, 1, 1, 5])?;

    let sensor_names = query_table_strings(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 1, 2])?;
    let sensor_values = query_table_i64_values(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 4])?;
    let sensor_statuses = query_table_i64_values(session, &[1, 3, 6, 1, 2, 1, 99, 1, 1, 1, 5])?;

    let physical_name_map: HashMap<u32, String> = physical_names.into_iter().collect();
    let physical_descr_map: HashMap<u32, String> = physical_descrs.into_iter().collect();
    // ★ アンダースコアを外して physical_class_map に戻す
    let physical_class_map: HashMap<u32, i64> = physical_class.into_iter().collect();

    let sensor_name_map: HashMap<u32, String> = sensor_names.into_iter().collect();
    let sensor_value_map: HashMap<u32, i64> = sensor_values.into_iter().collect();
    let sensor_status_map: HashMap<u32, i64> = sensor_statuses.into_iter().collect();

    let probes = vec![
        SnmpOidProbe {
            label: "ENTITY-SENSOR-MIB entPhySensorValue".to_string(),
            oid: "1.3.6.1.2.1.99.1.1.1.4".to_string(),
            value: u32::try_from(sensor_value_map.len()).ok(),
            selected: !sensor_value_map.is_empty(),
            status: if sensor_value_map.is_empty() {
                "n/a"
            } else {
                "ok"
            }
            .to_string(),
        },
        SnmpOidProbe {
            label: "ENTITY-SENSOR-MIB entPhySensorStatus".to_string(),
            oid: "1.3.6.1.2.1.99.1.1.1.5".to_string(),
            value: u32::try_from(sensor_status_map.len()).ok(),
            selected: !sensor_status_map.is_empty(),
            status: if sensor_status_map.is_empty() {
                "n/a"
            } else {
                "ok"
            }
            .to_string(),
        },
        SnmpOidProbe {
            label: "ENTITY-MIB entPhysicalName".to_string(),
            oid: "1.3.6.1.2.1.47.1.1.1.1.7".to_string(),
            value: u32::try_from(physical_name_map.len()).ok(),
            selected: !physical_name_map.is_empty(),
            status: if physical_name_map.is_empty() {
                "n/a"
            } else {
                "ok"
            }
            .to_string(),
        },
        SnmpOidProbe {
            label: "ENTITY-MIB entPhysicalClass".to_string(),
            oid: "1.3.6.1.2.1.47.1.1.1.1.5".to_string(),
            value: u32::try_from(physical_class_map.len()).ok(),
            selected: !physical_class_map.is_empty(),
            status: if physical_class_map.is_empty() {
                "n/a"
            } else {
                "ok"
            }
            .to_string(),
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
            status_text: None,
            is_alarm: matches!(status, Some(3) | Some(4) | Some(5)),
        });
    }

    Ok(HardwareInventory { sensors, probes })
}

/// CISCO-ENVMON-MIB (1.3.6.1.4.1.9.9.13) の電源テーブルを取得する。
fn query_cisco_envmon_power_sensors(
    session: &mut SyncSession,
) -> Result<Vec<SnmpHardwareSensor>, AppError> {
    let descr_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 13, 1, 1, 1, 2];
    let state_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 13, 1, 1, 1, 3];

    let descr_map: HashMap<u32, String> = query_table_strings(session, &descr_prefix)?
        .into_iter()
        .collect();
    let state_map: HashMap<u32, i64> = query_table_i64_values(session, &state_prefix)?
        .into_iter()
        .collect();

    let mut sensors = Vec::new();
    for (index, state) in &state_map {
        let name = descr_map
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("Power Supply {}", index));
        let is_alarm = matches!(state, 2 | 3 | 4 | 6);
        sensors.push(SnmpHardwareSensor {
            index: *index,
            name,
            sensor_type: "power".to_string(),
            source: "cisco-envmon".to_string(),
            oid: format!("{}.{}", oid_to_string(&state_prefix), index),
            value: Some(*state),
            unit: None,
            status: Some(*state),
            status_text: Some(format_envmon_status(*state)),
            is_alarm,
        });
    }

    Ok(sensors)
}

/// CISCO-ENVMON-MIB (1.3.6.1.4.1.9.9.13) のファンテーブルを取得する。
fn query_cisco_envmon_fan_sensors(
    session: &mut SyncSession,
) -> Result<Vec<SnmpHardwareSensor>, AppError> {
    let descr_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 13, 1, 4, 1, 2];
    let state_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 13, 1, 4, 1, 3];

    let descr_map: HashMap<u32, String> = query_table_strings(session, &descr_prefix)?
        .into_iter()
        .collect();
    let state_map: HashMap<u32, i64> = query_table_i64_values(session, &state_prefix)?
        .into_iter()
        .collect();

    let mut sensors = Vec::new();
    for (index, state) in &state_map {
        let name = descr_map
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("Fan Sensor {}", index));
        // ciscoEnvMonState: 1=normal, 2=warning, 3=critical, 4=shutdown, 5=notPresent, 6=notFunctioning
        let is_alarm = matches!(state, 2 | 3 | 4 | 6);
        sensors.push(SnmpHardwareSensor {
            index: *index,
            name,
            sensor_type: "fan".to_string(),
            source: "cisco-envmon".to_string(),
            oid: format!("{}.{}", oid_to_string(&state_prefix), index),
            value: Some(*state),
            unit: None,
            status: Some(*state),
            status_text: Some(format_envmon_status(*state)),
            is_alarm,
        });
    }

    Ok(sensors)
}

/// CISCO-ENVMON-MIB (1.3.6.1.4.1.9.9.13) の温度テーブルを取得する。
/// C2960X などは温度情報を ENTITY-SENSOR-MIB ではなくこちらでのみ公開している。
fn query_cisco_envmon_temperature_sensors(
    session: &mut SyncSession,
) -> Result<Vec<SnmpHardwareSensor>, AppError> {
    let descr_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 13, 1, 3, 1, 2];
    let value_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 13, 1, 3, 1, 3];
    let state_prefix = [1, 3, 6, 1, 4, 1, 9, 9, 13, 1, 3, 1, 6];

    let descr_map: HashMap<u32, String> = query_table_strings(session, &descr_prefix)?
        .into_iter()
        .collect();
    let value_map: HashMap<u32, i64> = query_table_i64_values(session, &value_prefix)?
        .into_iter()
        .collect();
    let state_map: HashMap<u32, i64> = query_table_i64_values(session, &state_prefix)?
        .into_iter()
        .collect();

    let mut sensors = Vec::new();
    for (index, value) in &value_map {
        let name = descr_map
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("Temperature Sensor {}", index));
        let state = state_map.get(index).copied();
        // ciscoEnvMonState: 1=normal, 2=warning, 3=critical, 4=shutdown, 5=notPresent, 6=notFunctioning
        let is_alarm = matches!(state, Some(2) | Some(3) | Some(4) | Some(6));
        sensors.push(SnmpHardwareSensor {
            index: *index,
            name,
            sensor_type: "temperature".to_string(),
            source: "cisco-envmon".to_string(),
            oid: format!("{}.{}", oid_to_string(&value_prefix), index),
            value: Some(*value),
            unit: sensor_unit_for_type("temperature"),
            status: state,
            status_text: state.map(format_envmon_status),
            is_alarm,
        });
    }

    Ok(sensors)
}

/// Hardware OID Override の種別を temperature / power / fan に正規化する。不明な値は fan 扱いにせず sensor とする。
fn normalize_manual_hardware_sensor_type(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "temperature" | "temp" => "temperature".to_string(),
        "power" | "psu" => "power".to_string(),
        "fan" => "fan".to_string(),
        _ => "sensor".to_string(),
    }
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

fn sensor_unit_for_type(sensor_type: &str) -> Option<String> {
    match sensor_type {
        "temperature" => Some("°C".to_string()),
        "fan" => Some("rpm".to_string()),
        "power" => Some("V".to_string()),
        "battery" => Some("%".to_string()),
        _ => None,
    }
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
    oid.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// CiscoEnvMonState の数値コードを人間が読める文字列に変換する
fn format_envmon_status(state: i64) -> String {
    match state {
        1 => "Normal".to_string(),
        2 => "Warning".to_string(),
        3 => "Critical".to_string(),
        4 => "Shutdown".to_string(),
        5 => "Not Present".to_string(),
        6 => "Not Functioning".to_string(),
        _ => state.to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct SnmpDeviceInfo {
    pub sys_name: String,
    pub sys_descr: String,
    pub cpu_usage: Option<u32>,
    pub memory_usage: Option<u32>,
    pub memory_used_bytes: Option<u64>,
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
    pub status_text: Option<String>,
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
pub struct SnmpMemoryProbe {
    pub label: String,
    pub oid: String,
    pub value: Option<u64>,
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
    pub memory_used_bytes: Option<u64>,
    pub cpu_probes: Vec<SnmpOidProbe>,
    pub memory_probes: Vec<SnmpOidProbe>,
    pub memory_byte_probes: Vec<SnmpMemoryProbe>,
    pub interfaces: Vec<SnmpInterfaceDiagnostic>,
    pub hardware_sensors: Vec<SnmpHardwareSensor>,
    pub hardware_probes: Vec<SnmpOidProbe>,
}
