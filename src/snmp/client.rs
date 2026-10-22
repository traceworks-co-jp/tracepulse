use crate::config::SnmpConfig;
use crate::device::types::DeviceConfig;
use crate::error::AppError;
use crate::monitor::interface::InterfaceMonitor;
use crate::snmp::template::{
    InterfaceTableTemplate, PercentageAggregation, TemplateRegistry, VendorOidTemplate,
};
use crate::snmp::walk::{SnmpValue, SnmpVarBind};
use crate::snmp::walk_api::{WalkStep, walk_with};
use crate::snmp::{ReliableWalkOutcome, WalkOptions, WalkReason};
use snmp::{SyncSession, Value};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// `walk_oid` の既定取得上限。
pub const DEFAULT_WALK_MAX_VARBINDS: usize = 1024;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplateMetricReading {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub data_type: crate::snmp::template::NumericDataType,
    pub oid: String,
    pub value: Option<serde_json::Number>,
    pub value_text: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SnmpClient {
    community: String,
    overrides: Option<SnmpConfig>,
    templates: std::sync::Arc<TemplateRegistry>,
}

impl SnmpClient {
    pub fn query_template_metrics(
        &self,
        device: &DeviceConfig,
    ) -> Result<Vec<TemplateMetricReading>, AppError> {
        let mut session = self.open_session(device)?;
        self.query_template_metrics_with_session(&mut session, &device.ip)
    }

    fn query_template_metrics_with_session(
        &self,
        session: &mut SyncSession,
        device_label: &str,
    ) -> Result<Vec<TemplateMetricReading>, AppError> {
        let sys_object_id = query_string(session, &[1, 3, 6, 1, 2, 1, 1, 2, 0])?;
        let template = self.templates.select(Some(&sys_object_id));
        Ok(template.numeric_metrics.iter().map(|metric| {
            let result = parse_oid_string(&metric.oid)
                .ok_or_else(|| AppError::Validation(format!("invalid metric OID '{}'", metric.oid)))
                .and_then(|oid| query_numeric_value(session, &oid));
            let (value, error) = match result {
                Ok(value) => (Some(value), None),
                Err(error) => {
                    tracing::warn!(device = %device_label, metric = %metric.key, %error, "Template metric unavailable");
                    (None, Some(error.to_string()))
                }
            };
            TemplateMetricReading {
                key: metric.key.clone(),
                label: metric.label.clone(),
                unit: metric.unit.clone(),
                data_type: metric.data_type,
                oid: metric.oid.clone(),
                value_text: value.as_ref().map(ToString::to_string),
                value,
                error,
            }
        }).collect())
    }

    fn query_cpu_from_template(
        &self,
        session: &mut SyncSession,
        sys_object_id: Option<&str>,
    ) -> Option<u32> {
        let cpu = &self.templates.select(sys_object_id).cpu;
        cpu.candidates.iter().find_map(|candidate| {
            match query_template_percentage(session, candidate, cpu.aggregation) {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(oid = %candidate, %error, "CPU template candidate unavailable");
                    None
                }
            }
        })
    }

    fn query_sensors_from_template(
        &self,
        session: &mut SyncSession,
        template: &VendorOidTemplate,
    ) -> HardwareInventory {
        let mut inventory = HardwareInventory {
            sensors: Vec::new(),
            probes: Vec::new(),
        };

        for (sensor_type, group_list) in &template.sensors {
            for group in group_list {
                let result = (|| -> Result<Vec<SnmpHardwareSensor>, AppError> {
                    let mut sensors = Vec::new();
                    let value_map: std::collections::BTreeMap<Vec<u32>, i64> =
                        if let Some(value_prefix) = parse_oid_string(&group.value_prefix) {
                            template_i64_column(session, &value_prefix)?
                        } else {
                            std::collections::BTreeMap::new()
                        };

                    let state_map: std::collections::BTreeMap<Vec<u32>, i64> =
                        if let Some(state_prefix) = parse_oid_string(&group.state_prefix) {
                            template_i64_column(session, &state_prefix)?
                        } else {
                            std::collections::BTreeMap::new()
                        };

                    let mut all_indices: Vec<Vec<u32>> =
                        value_map.keys().chain(state_map.keys()).cloned().collect();
                    all_indices.sort();
                    all_indices.dedup();

                    for (position, index) in all_indices.into_iter().enumerate() {
                        let suffix = oid_to_string(&index);
                        let name = if let Some(mut oid) = parse_oid_string(&group.descr_prefix) {
                            oid.extend(&index);
                            match query_string(session, &oid) {
                                Ok(name) => Some(name),
                                Err(error) => {
                                    tracing::warn!(oid = %oid_to_string(&oid), %error, "Template sensor name unavailable");
                                    None
                                }
                            }
                        } else {
                            None
                        };
                        if !group.match_names.is_empty()
                            && name.as_ref().is_none_or(|name| {
                                !group.match_names.iter().any(|pattern| {
                                    name.to_lowercase().contains(&pattern.to_lowercase())
                                })
                            })
                        {
                            continue;
                        }
                        let name = name.unwrap_or_else(|| {
                            format!("{} Sensor {}", sensor_type.to_uppercase(), suffix)
                        });
                        let value = value_map.get(&index).copied();
                        let state = state_map.get(&index).copied();
                        let status_val = state.or(value);
                        let is_alarm = state.is_some_and(|state| {
                            group
                                .alarm_states
                                .as_deref()
                                .unwrap_or(&[2, 3, 4, 6])
                                .contains(&state)
                        });

                        let oid_str = if !group.state_prefix.is_empty() {
                            format!("{}.{}", group.state_prefix.trim_start_matches('.'), suffix)
                        } else if !group.value_prefix.is_empty() {
                            format!("{}.{}", group.value_prefix.trim_start_matches('.'), suffix)
                        } else {
                            format!("{}.{}", group.descr_prefix.trim_start_matches('.'), suffix)
                        };

                        let status_text = if sensor_type == "temperature" {
                            None
                        } else {
                            status_val.map(|value| {
                                group
                                    .state_labels
                                    .get(&value.to_string())
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        if group.state_labels.is_empty() {
                                            format_envmon_status(value)
                                        } else {
                                            format!("Unknown ({value})")
                                        }
                                    })
                            })
                        };

                        sensors.push(SnmpHardwareSensor {
                            index: if index.len() == 1 {
                                index[0]
                            } else {
                                position as u32
                            },
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
                            status: if sensor_type == "temperature" {
                                state
                            } else {
                                status_val
                            },
                            status_text,
                            is_alarm,
                            status_kind: state
                                .map(|state| {
                                    if is_alarm {
                                        "critical"
                                    } else if group.normal_states.contains(&state) {
                                        "normal"
                                    } else {
                                        "warning"
                                    }
                                    .to_string()
                                })
                                .filter(|_| !group.normal_states.is_empty()),
                        });
                    }
                    Ok(sensors)
                })();
                let (sensors, status) = match result {
                    Ok(sensors) => (sensors, "ok".to_string()),
                    Err(error) => {
                        tracing::warn!(template = %template.name, %sensor_type, %error, "Template sensor group unavailable");
                        (Vec::new(), error.to_string())
                    }
                };
                inventory.probes.push(SnmpOidProbe {
                    label: format!("{} {}", template.name, sensor_type),
                    oid: if group.state_prefix.is_empty() {
                        group.value_prefix.clone()
                    } else {
                        group.state_prefix.clone()
                    },
                    value: (status == "ok").then_some(sensors.len() as u32),
                    selected: !sensors.is_empty(),
                    status: if status == "ok" && sensors.is_empty() {
                        "n/a".into()
                    } else {
                        status
                    },
                });
                inventory.sensors.extend(sensors);
            }
        }

        inventory
    }
}

pub fn initialize_templates() -> std::sync::Arc<TemplateRegistry> {
    static REGISTRY: std::sync::OnceLock<std::sync::Arc<TemplateRegistry>> =
        std::sync::OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            let dir = crate::data_dir().join("custom_templates");
            if let Err(error) = std::fs::create_dir_all(&dir) {
                tracing::error!("[Template] Cannot create {:?}: {}", dir, error);
            }
            std::sync::Arc::new(TemplateRegistry::load(&dir))
        })
        .clone()
}

impl SnmpClient {
    pub fn new(community: impl Into<String>) -> Self {
        Self {
            community: community.into(),
            overrides: None,
            templates: initialize_templates(),
        }
    }

    pub fn with_snmp_config(community: impl Into<String>, overrides: SnmpConfig) -> Self {
        Self {
            community: community.into(),
            overrides: Some(overrides),
            templates: initialize_templates(),
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

        let cpu_usage = self.query_cpu_usage(&mut session, sys_object_id.as_deref())?;

        let memory_usage = self
            .query_memory_usage(&mut session, sys_object_id.as_deref())
            .unwrap_or(None);
        let memory_used_bytes = self
            .query_memory_used_bytes(&mut session, sys_object_id.as_deref())
            .unwrap_or(None);
        let hardware =
            self.query_hardware_inventory_with_session(&mut session, sys_object_id.as_deref())?;

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
        Ok(self
            .query_hardware_inventory_with_session(&mut session, sys_object_id.as_deref())?
            .sensors)
    }

    pub fn query_sys_object_id(&self, device: &DeviceConfig) -> Result<String, AppError> {
        let mut session = self.open_session(device)?;
        query_string(&mut session, &[1, 3, 6, 1, 2, 1, 1, 2, 0])
    }

    pub fn query_numeric_oid(&self, device: &DeviceConfig, oid: &str) -> Result<i64, AppError> {
        let oid = parse_oid_string(oid)
            .ok_or_else(|| AppError::Validation(format!("invalid numeric OID '{oid}'")))?;
        let mut session = self.open_session(device)?;
        query_i64(&mut session, &oid)
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
            .filter(|v| *v <= 100);
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
            sys_object_id.as_deref(),
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
            status: if cpu_standard.filter(|v| *v <= 100).is_some() {
                "ok"
            } else {
                "n/a"
            }
            .to_string(),
        });

        let cpu_selected = cpu_override_value
            .or(vendor_cpu_value)
            .or(cpu_standard.filter(|v| *v <= 100));

        let (memory_usage, memory_probes) =
            self.diagnose_memory_candidates(&mut session, sys_object_id.as_deref())?;
        let (memory_used_bytes, memory_byte_probes) = self.diagnose_memory_bytes_candidates(
            &mut session,
            sys_object_id.as_deref(),
            vendor_name.as_deref(),
        )?;

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
            .query_hardware_inventory(device, sys_object_id.as_deref())
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
                    if let Some(&local_if) = suffix.get(1)
                        && let Some(val) = vb.value.as_string()
                        && !val.trim().is_empty()
                    {
                        lldp_names.insert(local_if as i32, val);
                    }
                } else if let Some(suffix) = vb.index_suffix(&LLDP_REM_PORT_ID_OID)
                    && let Some(&local_if) = suffix.get(1)
                    && let Some(val) = vb.value.as_string()
                    && !val.trim().is_empty()
                {
                    lldp_ports.insert(local_if as i32, val);
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
                    if let Some(&local_if) = suffix.first()
                        && let Some(val) = vb.value.as_string()
                        && !val.trim().is_empty()
                    {
                        cdp_names.insert(local_if as i32, val);
                    }
                } else if let Some(suffix) = vb.index_suffix(&CDP_CACHE_DEVICE_PORT_OID)
                    && let Some(&local_if) = suffix.first()
                    && let Some(val) = vb.value.as_string()
                    && !val.trim().is_empty()
                {
                    cdp_ports.insert(local_if as i32, val);
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
        let table = self.interface_table(&mut session)?;
        // ifIndex テーブル (1.3.6.1.2.1.2.2.1.1) を GETNEXT でウォーク
        let prefix = parse_oid_string(&table.index)
            .ok_or_else(|| AppError::Validation("invalid interface index OID".into()))?;
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
                let mut oid = prefix.clone();
                oid.push(i);
                if let Ok(mut r) = session.get(&oid)
                    && r.varbinds.next().is_some()
                {
                    indexes.push(i);
                }
            }
        }

        Ok(indexes)
    }

    fn interface_table(
        &self,
        session: &mut SyncSession,
    ) -> Result<InterfaceTableTemplate, AppError> {
        let sys_object_id = match query_string(session, &[1, 3, 6, 1, 2, 1, 1, 2, 0]) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::warn!(%error, "Cannot identify interface template; using RFC defaults");
                None
            }
        };
        self.templates
            .select(sys_object_id.as_deref())
            .tables
            .interfaces
            .as_ref()
            .or_else(|| self.templates.select(None).tables.interfaces.as_ref())
            .cloned()
            .ok_or_else(|| AppError::Validation("RFC interface template is missing".into()))
    }

    pub fn query_interface(
        &self,
        device: &DeviceConfig,
        if_index: u32,
        if_name: Option<&str>,
    ) -> Result<InterfaceMonitor, AppError> {
        self.query_interface_with_session(self.open_session(device)?, if_index, if_name)
    }

    fn query_interface_with_session(
        &self,
        mut session: SyncSession,
        if_index: u32,
        if_name: Option<&str>,
    ) -> Result<InterfaceMonitor, AppError> {
        let table = self.interface_table(&mut session)?;
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
        let in_octets = query_u64(&mut session, &indexed_oid(&table.in_octets, if_index)?)?;
        let out_octets = query_u64(&mut session, &indexed_oid(&table.out_octets, if_index)?)?;

        let resolved_name = match if_name {
            Some(name) => name.to_string(),
            None => {
                let name_oid = indexed_oid(&table.name, if_index)?;
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
        self.open_session_with_timeout(device, Duration::from_secs(2))
    }

    fn open_session_with_timeout(
        &self,
        device: &DeviceConfig,
        timeout: Duration,
    ) -> Result<SyncSession, AppError> {
        let community = if device.community.trim().is_empty() {
            self.community.clone()
        } else {
            device.community.clone()
        };

        let session_addr = format!("{}:161", device.ip);
        SyncSession::new(
            session_addr.as_str(),
            community.as_bytes(),
            Some(timeout),
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

    /// A bounded GETNEXT walk that reports completion quality instead of silently
    /// treating transport and agent errors as the end of a table.
    pub fn walk_oid_reliable(
        &self,
        device: &DeviceConfig,
        prefix: &[u32],
        options: &WalkOptions,
        cancellation: &AtomicBool,
    ) -> ReliableWalkOutcome {
        if device.ip.trim().is_empty() {
            return failed_walk(WalkReason::InvalidRequest(
                "SNMP device IP is empty".to_string(),
            ));
        }

        walk_with(prefix, options, cancellation, |current, remaining| {
            let request_timeout = remaining.min(Duration::from_secs(2));
            let mut session = match self.open_session_with_timeout(device, request_timeout) {
                Ok(session) => session,
                Err(error) => return WalkStep::Error(WalkReason::Transport(error.to_string())),
            };
            let mut response = match session.getnext(current) {
                Ok(response) => response,
                Err(error) => {
                    let error_text = format!("{error:?}");
                    let reason = if error_text.contains("ReceiveError") {
                        WalkReason::RequestTimeout
                    } else {
                        WalkReason::Transport(error_text)
                    };
                    return WalkStep::Error(reason);
                }
            };

            if response.error_status == 2 {
                return WalkStep::Unsupported(format!(
                    "SNMP noSuchName at varbind {}",
                    response.error_index
                ));
            }
            if response.error_status != 0 {
                return WalkStep::Error(WalkReason::AgentError {
                    status: response.error_status,
                    index: response.error_index,
                });
            }

            let Some((name, value)) = response.varbinds.next() else {
                return WalkStep::Error(WalkReason::EmptyResponse);
            };
            let mut oid_buf = [0u32; 128];
            let oid = match name.read_name(&mut oid_buf) {
                Ok(oid) => oid.to_vec(),
                Err(_) => return WalkStep::Error(WalkReason::InvalidOid),
            };

            match value {
                Value::Constructed(0x82, _) => WalkStep::EndOfMib,
                Value::Constructed(0x80 | 0x81, _) => {
                    WalkStep::Unsupported(format!("SNMP exception value for OID {oid:?}"))
                }
                value => WalkStep::VarBind(SnmpVarBind {
                    oid,
                    value: SnmpValue::from_raw(&value),
                }),
            }
        })
    }

    /// Async wrapper for callers that need to keep blocking SNMP socket I/O off
    /// the Tokio executor. Cancellation remains observable inside the worker.
    pub async fn walk_oid_reliable_async(
        &self,
        device: &DeviceConfig,
        prefix: &[u32],
        options: WalkOptions,
        cancellation: std::sync::Arc<AtomicBool>,
    ) -> ReliableWalkOutcome {
        let client = self.clone();
        let device = device.clone();
        let prefix = prefix.to_vec();
        tokio::task::spawn_blocking(move || {
            client.walk_oid_reliable(&device, &prefix, &options, &cancellation)
        })
        .await
        .unwrap_or_else(|error| {
            failed_walk(WalkReason::Transport(format!(
                "SNMP walk task failed: {error}"
            )))
        })
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

fn failed_walk(reason: WalkReason) -> ReliableWalkOutcome {
    ReliableWalkOutcome {
        status: crate::snmp::ReliableWalkStatus::Failed,
        varbinds: Vec::new(),
        reason: Some(reason),
        count: 0,
        cursor: None,
    }
}

fn indexed_oid(column: &str, index: u32) -> Result<Vec<u32>, AppError> {
    let mut oid = parse_oid_string(column)
        .ok_or_else(|| AppError::Validation(format!("invalid table column OID '{column}'")))?;
    oid.push(index);
    Ok(oid)
}

fn query_numeric_value(
    session: &mut SyncSession,
    oid: &[u32],
) -> Result<serde_json::Number, AppError> {
    let mut response = session.get(oid).map_err(|error| {
        AppError::Validation(format!("SNMP GET failed for OID {:?}: {:?}", oid, error))
    })?;
    if response.error_status != 0 {
        return Err(AppError::Validation(format!(
            "SNMP agent error {} at index {} for OID {:?}",
            response.error_status, response.error_index, oid
        )));
    }
    response
        .varbinds
        .find_map(|(name, value)| {
            if name != oid {
                return None;
            }
            match value {
                Value::Integer(value) => Some(value.into()),
                Value::Unsigned32(value) | Value::Counter32(value) | Value::Timeticks(value) => {
                    Some(value.into())
                }
                Value::Counter64(value) => Some(value.into()),
                _ => None,
            }
        })
        .ok_or_else(|| {
            AppError::Validation(format!("OID {:?} did not return a numeric value", oid))
        })
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
    let parts = parse_oid_string(sys_object_id)?;

    if parts.len() >= 7 && parts[..6] == [1, 3, 6, 1, 4, 1] {
        parts.get(6).copied()
    } else {
        None
    }
}

impl SnmpClient {
    fn query_cpu_usage(
        &self,
        session: &mut SyncSession,
        sys_object_id: Option<&str>,
    ) -> Result<Option<u32>, AppError> {
        if let Some(overrides) = &self.overrides
            && let Some(cpu_oid) = parse_oid_string(&overrides.cpu_oid_override)
            && let Ok(value) = query_u32(session, &cpu_oid)
            && value <= 100
        {
            return Ok(Some(value));
        }
        if let Some(value) = self.query_cpu_from_template(session, sys_object_id) {
            return Ok(Some(value));
        }
        query_table_average_u32(session, &[1, 3, 6, 1, 2, 1, 25, 3, 3, 1, 2])
    }

    fn query_memory_usage(
        &self,
        session: &mut SyncSession,
        sys_object_id: Option<&str>,
    ) -> Result<Option<u32>, AppError> {
        Ok(self.diagnose_memory_candidates(session, sys_object_id)?.0)
    }

    fn diagnose_memory_candidates(
        &self,
        session: &mut SyncSession,
        sys_object_id: Option<&str>,
    ) -> Result<(Option<u32>, Vec<SnmpOidProbe>), AppError> {
        let mut probes = Vec::new();
        if let Some(overrides) = &self.overrides
            && let Some(memory_oid) = parse_oid_string(&overrides.memory_oid_override)
            && let Ok(value) = query_u32_with_table_fallback(session, &memory_oid)
            && let Some(v) = value
            && v <= 100
        {
            probes.push(SnmpOidProbe {
                label: "Manual Memory Override".into(),
                oid: oid_to_string(&memory_oid),
                value: Some(v),
                selected: true,
                status: "ok".into(),
            });
            return Ok((Some(v), probes));
        }

        let template = self.templates.select(sys_object_id);
        for source in template
            .memory
            .sources()
            .filter(|source| !source.mode.is_empty() || !source.used_prefix.is_empty())
        {
            let (value, status) = match query_template_memory_source(session, source) {
                Ok(value) => (
                    value.map(|(percent, _)| percent),
                    if value.is_some() {
                        "ok".into()
                    } else {
                        "n/a".into()
                    },
                ),
                Err(error) => {
                    tracing::warn!(%error, template = %template.name, "Memory template source unavailable");
                    (None, error.to_string())
                }
            };
            probes.push(SnmpOidProbe {
                label: template.name.clone(),
                oid: if source.mode == "direct" {
                    source.oid.clone()
                } else {
                    source.used_prefix.clone()
                },
                value,
                selected: value.is_some(),
                status,
            });
            if value.is_some() {
                return Ok((value, probes));
            }
        }

        if let Ok((Some(used), Some(total))) = query_hr_storage_memory_used_and_total(session)
            && total > 0
        {
            let util = ((used as f64 / total as f64) * 100.0).round() as u32;
            probes.push(SnmpOidProbe {
                label: "Standard hrStorage".into(),
                oid: "1.3.6.1.2.1.25.2.3.1.6".into(),
                value: Some(util.min(100)),
                selected: true,
                status: "ok".into(),
            });
            return Ok((Some(util.min(100)), probes));
        }

        Ok((None, probes))
    }

    fn query_memory_used_bytes(
        &self,
        session: &mut SyncSession,
        sys_object_id: Option<&str>,
    ) -> Result<Option<u64>, AppError> {
        Ok(self
            .diagnose_memory_bytes_candidates(session, sys_object_id, None)?
            .0)
    }

    fn diagnose_cpu_vendor_candidates(
        &self,
        session: &mut SyncSession,
        sys_object_id: Option<&str>,
        _vendor_name: Option<&str>,
    ) -> Result<(Option<u32>, Vec<SnmpOidProbe>), AppError> {
        let mut probes = Vec::new();
        let template = self.templates.select(sys_object_id);
        let mut selected = None;
        for candidate in &template.cpu.candidates {
            let vendor_oid = parse_oid_string(candidate)
                .ok_or_else(|| AppError::Validation(format!("invalid CPU OID '{candidate}'")))?;
            let (vendor_value, status) = match query_template_percentage(
                session,
                candidate,
                template.cpu.aggregation,
            ) {
                Ok(value) => (
                    value,
                    if value.is_some() {
                        "ok".into()
                    } else {
                        "n/a".into()
                    },
                ),
                Err(error) => {
                    tracing::warn!(oid = %candidate, %error, "CPU template candidate unavailable");
                    (None, error.to_string())
                }
            };
            probes.push(SnmpOidProbe {
                label: template.name.clone(),
                oid: oid_to_string(&vendor_oid),
                value: vendor_value,
                selected: selected.is_none() && vendor_value.is_some(),
                status,
            });
            selected = selected.or(vendor_value);
        }

        Ok((selected, probes))
    }

    fn diagnose_memory_bytes_candidates(
        &self,
        session: &mut SyncSession,
        sys_object_id: Option<&str>,
        _vendor_name: Option<&str>,
    ) -> Result<(Option<u64>, Vec<SnmpMemoryProbe>), AppError> {
        let mut probes = Vec::new();
        let mut selected = None;

        if let Some(overrides) = &self.overrides
            && let Some(memory_oid) = parse_oid_string(&overrides.memory_oid_override)
        {
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

        if selected.is_none() {
            let template = self.templates.select(sys_object_id);
            for source in template
                .memory
                .sources()
                .filter(|source| source.used_is_bytes)
            {
                let (value, status) = match query_template_memory_source(session, source) {
                    Ok(value) => {
                        let bytes = value.and_then(|(_, bytes)| bytes);
                        (
                            bytes,
                            if bytes.is_some() {
                                "ok".into()
                            } else {
                                "n/a".into()
                            },
                        )
                    }
                    Err(error) => {
                        tracing::warn!(%error, template = %template.name, "Memory byte source unavailable");
                        (None, error.to_string())
                    }
                };
                probes.push(SnmpMemoryProbe {
                    label: template.name.clone(),
                    oid: source.used_prefix.clone(),
                    value,
                    selected: value.is_some(),
                    status,
                });
                if value.is_some() {
                    selected = value;
                    break;
                }
            }
        }
        if selected.is_none() {
            let (hr_used, hr_probes) = query_hr_storage_memory_used_bytes(session)?;
            selected = hr_used;
            probes.extend(hr_probes);
        }

        Ok((selected, probes))
    }
}

fn parse_oid_string(input: &str) -> Option<Vec<u32>> {
    if !crate::snmp::template::is_valid_oid(input) {
        return None;
    }
    input
        .trim_start_matches('.')
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

fn query_u32_with_table_fallback(
    session: &mut SyncSession,
    oid: &[u32],
) -> Result<Option<u32>, AppError> {
    if let Ok(value) = query_u32(session, oid) {
        return Ok(Some(value));
    }
    query_table_first_u32(session, oid)
}

fn query_template_column(
    session: &mut SyncSession,
    prefix: &[u32],
) -> Result<std::collections::BTreeMap<Vec<u32>, SnmpValue>, AppError> {
    let mut rows = std::collections::BTreeMap::new();
    let mut current = prefix.to_vec();
    for _ in 0..DEFAULT_WALK_MAX_VARBINDS {
        let response = session.getnext(&current).map_err(|error| {
            AppError::Io(format!(
                "template walk {}: {error:?}",
                oid_to_string(prefix)
            ))
        })?;
        if response.error_status != 0 {
            return Err(AppError::Io(format!(
                "template walk {}: agent error {}",
                oid_to_string(prefix),
                response.error_status
            )));
        }
        let mut advanced = false;
        for (oid, value) in response.varbinds {
            let mut buffer = [0u32; 128];
            let oid = oid
                .read_name(&mut buffer)
                .map_err(|error| AppError::Io(format!("invalid template row OID: {error:?}")))?;
            if matches!(value, Value::Null | Value::Constructed(0x82, _))
                || !oid.starts_with(prefix)
            {
                return Ok(rows);
            }
            if oid <= current.as_slice() {
                return Err(AppError::Io(format!(
                    "non-increasing template row OID: {}",
                    oid_to_string(oid)
                )));
            }
            rows.insert(oid[prefix.len()..].to_vec(), SnmpValue::from_raw(&value));
            current = oid.to_vec();
            advanced = true;
        }
        if !advanced {
            // snmp 0.2.2 exhausts the iterator for primitive endOfMibView values.
            tracing::debug!(oid = %oid_to_string(prefix), rows = rows.len(), "Template walk has no further decodable varbinds");
            return Ok(rows);
        }
    }
    Err(AppError::Io(format!(
        "template column {} exceeds {} rows",
        oid_to_string(prefix),
        DEFAULT_WALK_MAX_VARBINDS
    )))
}

fn template_i64_column(
    session: &mut SyncSession,
    prefix: &[u32],
) -> Result<std::collections::BTreeMap<Vec<u32>, i64>, AppError> {
    Ok(query_template_column(session, prefix)?
        .into_iter()
        .filter_map(|(index, value)| {
            let number = match value {
                SnmpValue::Integer(value) => Some(value),
                other => other.as_u64().and_then(|value| i64::try_from(value).ok()),
            };
            number.map(|value| (index, value))
        })
        .collect())
}

fn template_percentage(value: f64) -> Option<u32> {
    (value.is_finite() && (0.0..=100.0).contains(&value)).then(|| value.round() as u32)
}

fn query_template_percentage(
    session: &mut SyncSession,
    oid: &str,
    aggregation: PercentageAggregation,
) -> Result<Option<u32>, AppError> {
    let oid = parse_oid_string(oid)
        .ok_or_else(|| AppError::Validation("invalid template percentage OID".into()))?;
    if let Ok(value) = query_numeric_value(session, &oid) {
        return Ok(value.as_f64().and_then(template_percentage));
    }
    let rows = query_template_column(session, &oid)?;
    let mut values = rows
        .values()
        .filter_map(|value| value.as_u64())
        .filter_map(|value| template_percentage(value as f64));
    Ok(match aggregation {
        PercentageAggregation::First => values.next(),
        PercentageAggregation::Max => values.max(),
    })
}

fn query_template_memory_source(
    session: &mut SyncSession,
    source: &crate::snmp::template::MemorySource,
) -> Result<Option<(u32, Option<u64>)>, AppError> {
    if source.mode == "direct" {
        return Ok(
            query_template_percentage(session, &source.oid, source.aggregation)?
                .map(|value| (value, None)),
        );
    }
    let column = |session: &mut SyncSession,
                  raw: &str|
     -> Result<std::collections::BTreeMap<Vec<u32>, u64>, AppError> {
        if raw.is_empty() {
            return Ok(std::collections::BTreeMap::new());
        }
        let oid = parse_oid_string(raw)
            .ok_or_else(|| AppError::Validation("invalid memory column OID".into()))?;
        if let Ok(value) = query_numeric_value(session, &oid) {
            return Ok(value
                .as_u64()
                .map(|value| [(Vec::new(), value)].into_iter().collect())
                .unwrap_or_default());
        }
        Ok(query_template_column(session, &oid)?
            .into_iter()
            .filter_map(|(index, value)| value.as_u64().map(|value| (index, value)))
            .collect())
    };
    let used = column(session, &source.used_prefix)?;
    let free = column(session, &source.free_prefix)?;
    let total = column(session, &source.total_prefix)?;
    Ok(used
        .into_iter()
        .filter_map(|(index, used)| {
            let capacity = free
                .get(&index)
                .and_then(|free| used.checked_add(*free))
                .or_else(|| total.get(&index).copied())?;
            if capacity == 0 || used > capacity {
                return None;
            }
            Some((used, capacity))
        })
        .max_by_key(|(_, capacity)| *capacity)
        .map(|(used, capacity)| {
            (
                ((used as f64 / capacity as f64) * 100.0).round() as u32,
                source.used_is_bytes.then_some(used),
            )
        }))
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
            Value::Integer(value) => Some(value),
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
        sys_object_id: Option<&str>,
    ) -> Result<HardwareInventory, AppError> {
        let mut inventory = query_hardware_inventory_with_session(session, None)?;

        let template = self.templates.select(sys_object_id);
        let template_inventory = self.query_sensors_from_template(session, template);
        inventory.sensors.extend(template_inventory.sensors);
        inventory.probes.extend(template_inventory.probes);

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
        sys_object_id: Option<&str>,
    ) -> Result<HardwareInventory, AppError> {
        let mut session = self.open_session(device)?;
        let mut inventory =
            self.query_hardware_inventory_with_session(&mut session, sys_object_id)?;
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
            status_kind: None,
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
            status_kind: None,
        });
    }

    Ok(HardwareInventory { sensors, probes })
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
        2636 => Some("Juniper"),
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
    pub status_kind: Option<String>,
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

#[cfg(test)]
mod template_collection_tests {
    use super::*;
    use std::net::UdpSocket;
    use std::sync::atomic::Ordering;

    struct TemplateAgent {
        session: SyncSession,
        requests: std::sync::Arc<std::sync::Mutex<Vec<Vec<u32>>>>,
        stop: std::sync::Arc<AtomicBool>,
        server: Option<std::thread::JoinHandle<()>>,
    }

    impl TemplateAgent {
        fn new(rows: Vec<(&str, Vec<u8>)>) -> Self {
            let rows: std::collections::BTreeMap<_, _> = rows
                .into_iter()
                .map(|(oid, value)| (parse_oid_string(oid).unwrap(), value))
                .collect();
            let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
            socket
                .set_read_timeout(Some(Duration::from_millis(25)))
                .unwrap();
            let address = socket.local_addr().unwrap();
            let stop = std::sync::Arc::new(AtomicBool::new(false));
            let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let server_stop = stop.clone();
            let server_requests = requests.clone();
            let server = std::thread::spawn(move || {
                while !server_stop.load(Ordering::Relaxed) {
                    let mut buffer = [0u8; 2048];
                    let (size, peer) = match socket.recv_from(&mut buffer) {
                        Ok(value) => value,
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                            ) =>
                        {
                            continue;
                        }
                        Err(error) => panic!("mock receive: {error}"),
                    };
                    let (message, _) = take_tlv(&buffer[..size]);
                    let (version, rest) = take_tlv(&message[2..]);
                    let (community, rest) = take_tlv(rest);
                    let (request, _) = take_tlv(rest);
                    let (request_id, rest) = take_tlv(&request[2..]);
                    let (_, rest) = take_tlv(rest);
                    let (_, rest) = take_tlv(rest);
                    let (bindings, _) = take_tlv(rest);
                    let (binding, _) = take_tlv(&bindings[2..]);
                    let (oid, _) = take_tlv(&binding[2..]);
                    assert_eq!(oid[2], 43);
                    let mut requested = vec![1, 3];
                    let mut arc = 0u32;
                    for byte in &oid[3..] {
                        arc = (arc << 7) | u32::from(byte & 0x7f);
                        if byte & 0x80 == 0 {
                            requested.push(arc);
                            arc = 0;
                        }
                    }
                    server_requests.lock().unwrap().push(requested.clone());
                    let (returned, value) = if request[0] == 0xa0 {
                        (
                            requested.clone(),
                            rows.get(&requested)
                                .cloned()
                                .unwrap_or_else(|| tlv(0x80, &[])),
                        )
                    } else {
                        assert_eq!(request[0], 0xa1);
                        rows.range((
                            std::ops::Bound::Excluded(requested.clone()),
                            std::ops::Bound::Unbounded,
                        ))
                        .next()
                        .map(|(oid, value)| (oid.clone(), value.clone()))
                        .unwrap_or_else(|| (requested.clone(), tlv(0x82, &[])))
                    };
                    let returned = encoded_oid(&oid_to_string(&returned));
                    let bindings = tlv(0x30, &tlv(0x30, &[returned, value].concat()));
                    let pdu = tlv(0xa2, &[request_id, &[2, 1, 0, 2, 1, 0], &bindings].concat());
                    socket
                        .send_to(&tlv(0x30, &[version, community, &pdu].concat()), peer)
                        .unwrap();
                }
            });
            Self {
                session: SyncSession::new(
                    &address.to_string(),
                    b"public",
                    Some(Duration::from_secs(1)),
                    1,
                )
                .unwrap(),
                requests,
                stop,
                server: Some(server),
            }
        }
    }

    impl Drop for TemplateAgent {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            self.server.take().unwrap().join().unwrap();
        }
    }

    fn bundled_client() -> SnmpClient {
        SnmpClient {
            community: "public".into(),
            overrides: None,
            templates: std::sync::Arc::new(TemplateRegistry::load(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("templates")
                    .join("absent"),
            )),
        }
    }

    fn unsigned(tag: u8, value: u64) -> Vec<u8> {
        let bytes = value.to_be_bytes();
        let start = bytes.iter().position(|byte| *byte != 0).unwrap_or(7);
        let mut body = bytes[start..].to_vec();
        if body[0] & 0x80 != 0 {
            body.insert(0, 0);
        }
        tlv(tag, &body)
    }

    fn tlv(tag: u8, body: &[u8]) -> Vec<u8> {
        assert!(body.len() < 128);
        [vec![tag, body.len() as u8], body.to_vec()].concat()
    }

    fn take_tlv(bytes: &[u8]) -> (&[u8], &[u8]) {
        assert!(bytes.len() >= 2 && bytes[1] < 128);
        let end = 2 + usize::from(bytes[1]);
        (&bytes[..end], &bytes[end..])
    }

    fn encoded_oid(oid: &str) -> Vec<u8> {
        let parts = parse_oid_string(oid).unwrap();
        let mut result = vec![(parts[0] * 40 + parts[1]) as u8];
        for part in &parts[2..] {
            let mut value = *part;
            let mut bytes = vec![(value & 0x7f) as u8];
            value >>= 7;
            while value > 0 {
                bytes.push((value as u8 & 0x7f) | 0x80);
                value >>= 7;
            }
            result.extend(bytes.into_iter().rev());
        }
        tlv(0x06, &result)
    }

    fn mock_session(
        replies: Vec<(&'static str, Vec<u8>)>,
    ) -> (SyncSession, std::thread::JoinHandle<()>) {
        mock_session_with_status(replies, 0)
    }

    fn mock_session_with_status(
        replies: Vec<(&'static str, Vec<u8>)>,
        error_status: u8,
    ) -> (SyncSession, std::thread::JoinHandle<()>) {
        mock_session_with_returned_oid(replies, error_status, None)
    }

    fn mock_session_with_returned_oid(
        replies: Vec<(&'static str, Vec<u8>)>,
        error_status: u8,
        returned_oid: Option<&'static str>,
    ) -> (SyncSession, std::thread::JoinHandle<()>) {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let address = socket.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for (expected_oid, value) in replies {
                let mut buffer = [0u8; 1024];
                let (size, peer) = socket.recv_from(&mut buffer).unwrap();
                let (message, _) = take_tlv(&buffer[..size]);
                let (version, rest) = take_tlv(&message[2..]);
                let (community, rest) = take_tlv(rest);
                let (request, _) = take_tlv(rest);
                assert!(matches!(request[0], 0xa0 | 0xa1));
                let (request_id, rest) = take_tlv(&request[2..]);
                let (_, rest) = take_tlv(rest);
                let (_, rest) = take_tlv(rest);
                let (bindings, _) = take_tlv(rest);
                let (binding, _) = take_tlv(&bindings[2..]);
                let (oid, _) = take_tlv(&binding[2..]);
                assert_eq!(oid, encoded_oid(expected_oid));
                let returned_oid = if let Some(returned_oid) = returned_oid {
                    encoded_oid(returned_oid)
                } else if request[0] == 0xa1 {
                    let mut next = parse_oid_string(expected_oid).unwrap();
                    *next.last_mut().unwrap() += 1;
                    encoded_oid(&oid_to_string(&next))
                } else {
                    oid.to_vec()
                };
                let bindings = tlv(
                    0x30,
                    &tlv(0x30, &[returned_oid.as_slice(), &value].concat()),
                );
                let pdu = tlv(
                    0xa2,
                    &[request_id, &[2, 1, error_status, 2, 1, 0], &bindings].concat(),
                );
                let response = tlv(0x30, &[version, community, &pdu].concat());
                socket.send_to(&response, peer).unwrap();
            }
        });
        let session = SyncSession::new(
            address.to_string().as_str(),
            b"public",
            Some(Duration::from_secs(2)),
            1,
        )
        .unwrap();
        (session, server)
    }

    #[test]
    fn collects_custom_toml_metrics_via_numeric_snmp_get() {
        let registry = TemplateRegistry::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("examples")
                .join("custom_templates"),
        );
        let client = SnmpClient {
            community: "public".into(),
            overrides: None,
            templates: std::sync::Arc::new(registry),
        };
        let (mut session, server) = mock_session(vec![
            ("1.3.6.1.2.1.1.2.0", encoded_oid("1.3.6.1.4.1.65000.1")),
            ("1.3.6.1.2.1.1.3.0", tlv(0x43, &[0])),
            ("1.3.6.1.2.1.25.3.3.1.2.1", tlv(0x02, &[25])),
        ]);
        let readings = client
            .query_template_metrics_with_session(&mut session, "mock")
            .unwrap();
        server.join().unwrap();
        assert_eq!(readings.len(), 2);
        assert_eq!(readings[0].key, "uptime");
        assert_eq!(readings[0].value.as_ref().unwrap().as_u64(), Some(0));
        assert_eq!(readings[1].value.as_ref().unwrap().as_u64(), Some(25));
        assert_eq!(readings[1].unit, "percent");
        assert!(readings.iter().all(|reading| reading.error.is_none()));
    }

    #[test]
    fn preserves_signed_gauges_large_counter64_and_missing_values() {
        let oid = "1.3.6.1.2.1.1.3.0";
        let (mut session, server) = mock_session(vec![
            (oid, tlv(0x02, &[0xfb])),
            (oid, tlv(0x46, &[0x20, 0, 0, 0, 0, 0, 1])),
            (oid, tlv(0x80, &[])),
        ]);
        let parsed = parse_oid_string(oid).unwrap();
        assert_eq!(
            query_numeric_value(&mut session, &parsed).unwrap().as_i64(),
            Some(-5)
        );
        assert_eq!(
            query_numeric_value(&mut session, &parsed).unwrap().as_u64(),
            Some(9_007_199_254_740_993)
        );
        assert!(query_numeric_value(&mut session, &parsed).is_err());
        server.join().unwrap();
    }

    #[test]
    fn reports_agent_errors_instead_of_accepting_returned_numbers() {
        let oid = "1.3.6.1.2.1.1.3.0";
        let (mut session, server) = mock_session_with_status(vec![(oid, tlv(0x02, &[42]))], 16);
        let error = query_numeric_value(&mut session, &parse_oid_string(oid).unwrap()).unwrap_err();
        assert!(error.to_string().contains("SNMP agent error 16"));
        server.join().unwrap();
    }

    #[test]
    fn rfc_and_custom_interface_templates_preserve_traffic_and_link_status() {
        for (sys_object_id, name_oid, status, expected_status) in [
            ("1.3.6.1.4.1.65001.1", "1.3.6.1.2.1.2.2.1.2.7", 1, "up"),
            ("1.3.6.1.4.1.65000.1", "1.3.6.1.2.1.31.1.1.1.1.7", 2, "down"),
        ] {
            let registry = TemplateRegistry::load(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("examples")
                    .join("custom_templates"),
            );
            let client = SnmpClient {
                community: "public".into(),
                overrides: None,
                templates: std::sync::Arc::new(registry),
            };
            let (session, server) = mock_session(vec![
                ("1.3.6.1.2.1.1.2.0", encoded_oid(sys_object_id)),
                ("1.3.6.1.2.1.2.2.1.8.7", tlv(0x02, &[status])),
                ("1.3.6.1.2.1.2.2.1.14.7", tlv(0x41, &[0])),
                ("1.3.6.1.2.1.2.2.1.20.7", tlv(0x41, &[0])),
                ("1.3.6.1.2.1.2.2.1.11.7", tlv(0x41, &[10])),
                ("1.3.6.1.2.1.2.2.1.17.7", tlv(0x41, &[20])),
                ("1.3.6.1.2.1.2.2.1.13.7", tlv(0x41, &[0])),
                ("1.3.6.1.2.1.2.2.1.19.7", tlv(0x41, &[0])),
                ("1.3.6.1.2.1.10.7.2.1.8.7", tlv(0x41, &[0])),
                ("1.3.6.1.2.1.10.7.2.1.3.7", tlv(0x41, &[0])),
                ("1.3.6.1.2.1.10.7.2.1.2.7", tlv(0x41, &[0])),
                ("1.3.6.1.2.1.10.7.2.1.13.7", tlv(0x41, &[0])),
                ("1.3.6.1.2.1.10.7.2.1.16.7", tlv(0x41, &[0])),
                ("1.3.6.1.2.1.2.2.1.10.7", tlv(0x41, &[100])),
                ("1.3.6.1.2.1.2.2.1.16.7", tlv(0x41, &[0, 200])),
                (name_oid, tlv(0x04, b"Ethernet7")),
                ("1.3.6.1.2.1.99.1.1.1.2", tlv(0x80, &[])),
            ]);
            let interface = client
                .query_interface_with_session(session, 7, None)
                .unwrap();
            server.join().unwrap();
            assert_eq!(interface.if_name, "Ethernet7");
            assert_eq!(interface.link_status, expected_status);
            assert_eq!(interface.in_octets, 100);
            assert_eq!(interface.out_octets, 200);
            assert_eq!(interface.in_errors, 0);
            let sample = interface.into_sample(1);
            assert_eq!(sample.if_index, 7);
            assert_eq!(sample.in_octets, 100);
        }
    }

    #[test]
    fn keeps_unavailable_custom_metric_as_null_with_error() {
        let registry = TemplateRegistry::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("examples")
                .join("custom_templates"),
        );
        let client = SnmpClient {
            community: "public".into(),
            overrides: None,
            templates: std::sync::Arc::new(registry),
        };
        let (mut session, server) = mock_session(vec![
            ("1.3.6.1.2.1.1.2.0", encoded_oid("1.3.6.1.4.1.65000.1")),
            ("1.3.6.1.2.1.1.3.0", tlv(0x43, &[0])),
            ("1.3.6.1.2.1.25.3.3.1.2.1", tlv(0x80, &[])),
        ]);
        let readings = client
            .query_template_metrics_with_session(&mut session, "mock")
            .unwrap();
        server.join().unwrap();
        assert_eq!(readings[0].value_text.as_deref(), Some("0"));
        assert!(readings[0].error.is_none());
        assert!(readings[1].value.is_none());
        assert!(readings[1].value_text.is_none());
        assert!(readings[1].error.is_some());
    }

    #[test]
    fn rejects_malformed_oids_instead_of_dropping_invalid_arcs() {
        for oid in ["1.3.x.6", "1..3", "1.3.", "1.+3", "9.1.2"] {
            assert!(parse_oid_string(oid).is_none(), "{oid}");
        }
        assert_eq!(
            indexed_oid(".1.3.6.1.2.1.2.2.1.10", 7).unwrap(),
            vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 10, 7]
        );
    }

    #[test]
    fn yamaha_cpu_candidates_and_memory_keep_native_percentage_units() {
        let client = bundled_client();
        let sys_oid = Some("1.3.6.1.4.1.1182.1.53");
        for candidate in [
            "1.3.6.1.4.1.1182.2.1.5.0",
            "1.3.6.1.4.1.1182.2.1.6.0",
            "1.3.6.1.4.1.1182.2.1.7.0",
        ] {
            let mut agent = TemplateAgent::new(vec![
                (candidate, unsigned(0x42, 37)),
                ("1.3.6.1.4.1.1182.2.1.4.0", unsigned(0x42, 74)),
            ]);
            assert_eq!(
                client.query_cpu_from_template(&mut agent.session, sys_oid),
                Some(37)
            );
            assert_eq!(
                client
                    .query_memory_usage(&mut agent.session, sys_oid)
                    .unwrap(),
                Some(74)
            );
        }
        let mut agent = TemplateAgent::new(vec![
            ("1.3.6.1.4.1.1182.2.1.5.0", unsigned(0x42, 0)),
            ("1.3.6.1.4.1.1182.2.1.6.0", unsigned(0x42, 99)),
        ]);
        assert_eq!(
            client.query_cpu_from_template(&mut agent.session, sys_oid),
            Some(0)
        );
        assert_eq!(agent.requests.lock().unwrap().len(), 1);
        let (value, probes) = client
            .diagnose_cpu_vendor_candidates(&mut agent.session, sys_oid, None)
            .unwrap();
        assert_eq!(value, Some(0));
        assert_eq!(probes.iter().filter(|probe| probe.selected).count(), 1);
        assert!(probes[0].selected);
    }

    #[test]
    fn rejects_invalid_percentages_instead_of_clamping_or_wrapping() {
        let client = bundled_client();
        let mut agent = TemplateAgent::new(vec![
            ("1.3.6.1.4.1.1182.2.1.5.0", unsigned(0x42, 200)),
            ("1.3.6.1.4.1.1182.2.1.6.0", tlv(0x02, &[0xff])),
            ("1.3.6.1.4.1.1182.2.1.7.0", unsigned(0x42, 55)),
        ]);
        assert_eq!(
            client.query_cpu_from_template(&mut agent.session, Some("1.3.6.1.4.1.1182.1")),
            Some(55)
        );
        for value in [-1.0, 100.1, f64::INFINITY, f64::NAN] {
            assert_eq!(template_percentage(value), None);
        }
        assert_eq!(template_percentage(0.0), Some(0));
        assert_eq!(template_percentage(100.0), Some(100));
    }

    #[test]
    fn cisco_cpu_prefers_one_minute_total_then_core_and_uses_max_valid_row() {
        let client = bundled_client();
        let sys_oid = Some("1.3.6.1.4.1.9.1.1");
        for total_supported in [true, false] {
            let mut rows = vec![
                ("1.3.6.1.4.1.9.9.109.1.1.2.1.4.1.1", unsigned(0x42, 0)),
                ("1.3.6.1.4.1.9.9.109.1.1.2.1.4.2.1", unsigned(0x42, 92)),
            ];
            if total_supported {
                rows.extend([
                    ("1.3.6.1.4.1.9.9.109.1.1.1.1.7.1", unsigned(0x42, 10)),
                    ("1.3.6.1.4.1.9.9.109.1.1.1.1.7.2", unsigned(0x42, 55)),
                    ("1.3.6.1.4.1.9.9.109.1.1.1.1.7.3", unsigned(0x42, 101)),
                ]);
            }
            let mut agent = TemplateAgent::new(rows);
            assert_eq!(
                client.query_cpu_from_template(&mut agent.session, sys_oid),
                Some(if total_supported { 55 } else { 92 })
            );
        }
    }

    #[test]
    fn cisco_memory_joins_complete_indices_and_reports_same_pool_used_bytes() {
        let client = bundled_client();
        let sys_oid = Some("1.3.6.1.4.1.9.1");
        let mut agent = TemplateAgent::new(vec![
            ("1.3.6.1.4.1.9.9.221.1.1.1.1.7.1.1", unsigned(0x42, 100)),
            ("1.3.6.1.4.1.9.9.221.1.1.1.1.8.1.1", unsigned(0x42, 300)),
            ("1.3.6.1.4.1.9.9.221.1.1.1.1.7.2.1", unsigned(0x42, 900)),
            ("1.3.6.1.4.1.9.9.221.1.1.1.1.8.2.1", unsigned(0x42, 100)),
            ("1.3.6.1.4.1.9.9.221.1.1.1.1.7.3.1", unsigned(0x42, 9999)),
        ]);
        assert_eq!(
            client
                .query_memory_usage(&mut agent.session, sys_oid)
                .unwrap(),
            Some(90)
        );
        assert_eq!(
            client
                .query_memory_used_bytes(&mut agent.session, sys_oid)
                .unwrap(),
            Some(900)
        );
        let (value, probes) = client
            .diagnose_memory_candidates(&mut agent.session, sys_oid)
            .unwrap();
        assert_eq!(value, Some(90));
        assert!(probes[0].selected);
        assert!(probes[0].oid.contains(".221."));
    }

    #[test]
    fn cisco_memory_falls_back_to_legacy_and_preserves_zero_and_counter64() {
        let client = bundled_client();
        let sys_oid = Some("1.3.6.1.4.1.9.1");
        for used in [0, 30, 5_000_000_000] {
            let free = if used == 5_000_000_000 {
                used
            } else {
                100 - used
            };
            let mut agent = TemplateAgent::new(vec![
                ("1.3.6.1.4.1.9.9.221.1.1.1.1.7.1.1", unsigned(0x42, 0)),
                ("1.3.6.1.4.1.9.9.221.1.1.1.1.8.1.1", unsigned(0x42, 0)),
                ("1.3.6.1.4.1.9.9.48.1.1.1.5.1", unsigned(0x46, used)),
                ("1.3.6.1.4.1.9.9.48.1.1.1.6.1", unsigned(0x46, free)),
            ]);
            assert_eq!(
                client
                    .query_memory_usage(&mut agent.session, sys_oid)
                    .unwrap(),
                Some(if used > 100 { 50 } else { used as u32 })
            );
            assert_eq!(
                client
                    .query_memory_used_bytes(&mut agent.session, sys_oid)
                    .unwrap(),
                Some(used)
            );
        }
    }

    #[test]
    fn juniper_uses_max_percentage_and_keeps_four_component_sensor_rows() {
        let client = bundled_client();
        let sys_oid = Some("1.3.6.1.4.1.2636.1.1");
        let mut agent = TemplateAgent::new(vec![
            ("1.3.6.1.4.1.2636.3.1.13.1.8.1.1.0.0", unsigned(0x42, 0)),
            ("1.3.6.1.4.1.2636.3.1.13.1.8.2.1.0.0", unsigned(0x42, 80)),
            ("1.3.6.1.4.1.2636.3.1.13.1.11.1.1.0.0", unsigned(0x42, 0)),
            ("1.3.6.1.4.1.2636.3.1.13.1.11.2.1.0.0", unsigned(0x42, 65)),
            ("1.3.6.1.4.1.2636.3.1.13.1.12.2.1.0.0", unsigned(0x42, 99)),
            ("1.3.6.1.4.1.2636.3.1.13.1.7.1.1.0.0", unsigned(0x42, 31)),
            ("1.3.6.1.4.1.2636.3.1.13.1.7.2.1.0.0", unsigned(0x42, 42)),
            ("1.3.6.1.4.1.2636.3.1.13.1.5.1.1.0.0", tlv(0x04, b"RE 0")),
            ("1.3.6.1.4.1.2636.3.1.13.1.5.2.1.0.0", tlv(0x04, b"RE 1")),
            (
                "1.3.6.1.4.1.2636.3.1.15.1.5.1.1.0.0",
                tlv(0x04, b"Power Supply 0"),
            ),
            (
                "1.3.6.1.4.1.2636.3.1.15.1.5.2.1.0.0",
                tlv(0x04, b"Fan Tray 0"),
            ),
            (
                "1.3.6.1.4.1.2636.3.1.15.1.5.3.1.0.0",
                tlv(0x04, b"Routing Engine"),
            ),
            ("1.3.6.1.4.1.2636.3.1.15.1.8.1.1.0.0", unsigned(0x02, 6)),
            ("1.3.6.1.4.1.2636.3.1.15.1.8.2.1.0.0", unsigned(0x02, 8)),
            ("1.3.6.1.4.1.2636.3.1.15.1.8.3.1.0.0", unsigned(0x02, 6)),
        ]);
        assert_eq!(
            client.query_cpu_from_template(&mut agent.session, sys_oid),
            Some(80)
        );
        assert_eq!(
            client
                .query_memory_usage(&mut agent.session, sys_oid)
                .unwrap(),
            Some(65)
        );
        let template = client.templates.select(sys_oid);
        assert!(
            template
                .memory
                .sources()
                .all(|source| !source.used_is_bytes)
        );
        let inventory = client.query_sensors_from_template(&mut agent.session, template);
        assert_eq!(inventory.sensors.len(), 4);
        let temperatures: Vec<_> = inventory
            .sensors
            .iter()
            .filter(|sensor| sensor.sensor_type == "temperature")
            .collect();
        assert_eq!(temperatures.len(), 2);
        assert_eq!(temperatures[0].name, "RE 0");
        assert_eq!(temperatures[1].name, "RE 1");
        assert_eq!(temperatures[0].value, Some(31));
        assert_eq!(temperatures[1].value, Some(42));
        assert!(temperatures[0].oid.ends_with(".1.1.0.0"));
        assert!(temperatures[1].oid.ends_with(".2.1.0.0"));
        assert!(temperatures.iter().all(|sensor| sensor.status.is_none()));
        let power = inventory
            .sensors
            .iter()
            .find(|sensor| sensor.sensor_type == "power")
            .unwrap();
        assert!(!power.is_alarm);
        assert_eq!(power.status_kind.as_deref(), Some("normal"));
        assert_eq!(power.unit, None);
        let fan = inventory
            .sensors
            .iter()
            .find(|sensor| sensor.sensor_type == "fan")
            .unwrap();
        assert!(fan.is_alarm);
        assert_eq!(fan.status_text.as_deref(), Some("Offline"));
    }

    #[test]
    fn cisco_fru_power_on_is_normal_and_failure_is_alarm_without_phantom_entities() {
        let client = bundled_client();
        let template = client.templates.select(Some("1.3.6.1.4.1.9.1"));
        let mut agent = TemplateAgent::new(vec![
            ("1.3.6.1.2.1.47.1.1.1.1.7.1", tlv(0x04, b"Chassis")),
            ("1.3.6.1.2.1.47.1.1.1.1.7.10", tlv(0x04, b"PSU 1")),
            ("1.3.6.1.2.1.47.1.1.1.1.7.11", tlv(0x04, b"PSU 2")),
            ("1.3.6.1.2.1.47.1.1.1.1.7.12", tlv(0x04, b"PSU 3")),
            ("1.3.6.1.2.1.47.1.1.1.1.7.13", tlv(0x04, b"PSU 4")),
            ("1.3.6.1.4.1.9.9.117.1.1.2.1.2.10", unsigned(0x02, 2)),
            ("1.3.6.1.4.1.9.9.117.1.1.2.1.2.11", unsigned(0x02, 8)),
            ("1.3.6.1.4.1.9.9.117.1.1.2.1.2.12", unsigned(0x02, 3)),
            ("1.3.6.1.4.1.9.9.117.1.1.2.1.2.13", unsigned(0x02, 99)),
        ]);
        let inventory = client.query_sensors_from_template(&mut agent.session, template);
        assert_eq!(inventory.sensors.len(), 4);
        assert_eq!(inventory.sensors[0].name, "PSU 1");
        assert_eq!(inventory.sensors[0].status_text.as_deref(), Some("On"));
        assert_eq!(inventory.sensors[0].status_kind.as_deref(), Some("normal"));
        assert!(!inventory.sensors[0].is_alarm);
        assert!(inventory.sensors[1].is_alarm);
        assert_eq!(inventory.sensors[1].status_text.as_deref(), Some("Failed"));
        assert!(!inventory.sensors[2].is_alarm);
        assert_eq!(inventory.sensors[2].status_kind.as_deref(), Some("warning"));
        assert_eq!(
            inventory.sensors[3].status_text.as_deref(),
            Some("Unknown (99)")
        );
        assert!(
            inventory
                .sensors
                .iter()
                .all(|sensor| sensor.unit.is_none() && sensor.value.is_none())
        );
    }

    #[test]
    fn template_walk_rejects_non_increasing_rows_and_reports_sensor_agent_errors() {
        let oid = "1.3.6.1.2.1.1.3";
        let (mut session, server) =
            mock_session_with_returned_oid(vec![(oid, unsigned(0x42, 42))], 0, Some(oid));
        assert!(
            query_template_column(&mut session, &parse_oid_string(oid).unwrap())
                .unwrap_err()
                .to_string()
                .contains("non-increasing")
        );
        server.join().unwrap();
        let client = bundled_client();
        let template = client.templates.select(Some("1.3.6.1.4.1.9.1"));
        let (mut session, server) = mock_session_with_status(
            vec![("1.3.6.1.4.1.9.9.117.1.1.2.1.2", unsigned(0x02, 2))],
            16,
        );
        let inventory = client.query_sensors_from_template(&mut session, template);
        server.join().unwrap();
        assert!(inventory.sensors.is_empty());
        assert_eq!(inventory.probes.len(), 1);
        assert_eq!(inventory.probes[0].value, None);
        assert!(!inventory.probes[0].selected);
        assert!(inventory.probes[0].status.contains("agent error 16"));
    }

    #[test]
    fn legacy_scalar_memory_and_envmon_sensor_configuration_remain_compatible() {
        let source: crate::snmp::template::MemoryTemplate = toml::from_str(
            "mode = 'calculated'\nused_oid = '1.3.6.1.2.1.1.3.0'\nfree_oid = '1.3.6.1.2.1.1.4.0'",
        )
        .unwrap();
        let mut agent = TemplateAgent::new(vec![
            ("1.3.6.1.2.1.1.3.0", unsigned(0x42, 0)),
            ("1.3.6.1.2.1.1.4.0", unsigned(0x42, 100)),
        ]);
        assert_eq!(
            query_template_memory_source(&mut agent.session, &source.primary).unwrap(),
            Some((0, None))
        );
        let template: VendorOidTemplate = toml::from_str(
            "name = 'Legacy'\n[[sensors.power]]\nstate_prefix = '1.3.6.1.4.1.65000.1'",
        )
        .unwrap();
        let mut agent = TemplateAgent::new(vec![
            ("1.3.6.1.4.1.65000.1.1", unsigned(0x02, 1)),
            ("1.3.6.1.4.1.65000.1.2", unsigned(0x02, 2)),
        ]);
        let inventory = bundled_client().query_sensors_from_template(&mut agent.session, &template);
        assert_eq!(inventory.sensors.len(), 2);
        assert_eq!(inventory.sensors[0].status_text.as_deref(), Some("Normal"));
        assert!(!inventory.sensors[0].is_alarm);
        assert!(inventory.sensors[1].is_alarm);
        assert!(
            inventory
                .sensors
                .iter()
                .all(|sensor| sensor.status_kind.is_none())
        );
    }
}
