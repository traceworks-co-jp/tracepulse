use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, RwLock};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::template::{TemplateRegistry, VendorOidTemplate, is_valid_oid};
use crate::config::SnmpConfig;
use crate::error::AppError;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonOids {
    #[serde(default)]
    pub cpu_oid: String,
    #[serde(default)]
    pub memory_oid: String,
    #[serde(default)]
    pub hardware_oids: Vec<String>,
}

impl CommonOids {
    fn validate(&self) -> Result<(), AppError> {
        for oid in [&self.cpu_oid, &self.memory_oid] {
            if !oid.is_empty() && !is_valid_oid(oid) {
                return Err(AppError::Validation(format!("invalid numeric OID: {oid}")));
            }
        }
        if self.hardware_oids.len() > 128 {
            return Err(AppError::Validation(
                "at most 128 common hardware OIDs".into(),
            ));
        }
        for entry in &self.hardware_oids {
            let (kind, oid) = entry.split_once('|').unwrap_or(("sensor", entry));
            if !matches!(kind, "temperature" | "power" | "fan" | "sensor") || !is_valid_oid(oid) {
                return Err(AppError::Validation(format!(
                    "invalid common hardware OID: {entry}"
                )));
            }
        }
        Ok(())
    }

    pub fn apply(&self, config: &mut SnmpConfig) {
        config.cpu_oid_override.clone_from(&self.cpu_oid);
        config.memory_oid_override.clone_from(&self.memory_oid);
        config
            .hardware_oid_overrides
            .clone_from(&self.hardware_oids);
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definitions {
    #[serde(default)]
    pub common: CommonOids,
    /// A profile is an explicit user-owned fork; absence restores its file-backed definition.
    #[serde(default)]
    pub profiles: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveDefinitions {
    pub generation: i64,
    pub definitions: Definitions,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewDefinitions {
    pub definitions: Definitions,
    pub sys_object_id: String,
}

#[derive(Debug, Clone)]
pub struct TemplateState {
    baseline: Arc<TemplateRegistry>,
    pub registry: Arc<TemplateRegistry>,
    pub generation: i64,
    pub definitions: Definitions,
    managed: bool,
}

pub fn document(template: &VendorOidTemplate) -> Result<Value, AppError> {
    let mut value = serde_json::to_value(template).map_err(validation)?;
    let map = value
        .as_object_mut()
        .ok_or_else(|| validation("invalid template document"))?;
    if let Some(prefix) = map.remove("sys_object_id_prefix") {
        map.insert("sysObjectID_prefix".into(), prefix);
    }
    let extra = map
        .remove("numeric_metrics")
        .unwrap_or(Value::Array(Vec::new()));
    if let (Some(Value::Array(metrics)), Value::Array(extra)) = (map.get_mut("metrics"), extra) {
        metrics.extend(extra);
    }
    remove_nulls(&mut value);
    Ok(value)
}

fn remove_nulls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|_, value| !value.is_null());
            for value in map.values_mut() {
                remove_nulls(value);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(remove_nulls),
        _ => {}
    }
}

fn validation(error: impl std::fmt::Display) -> AppError {
    AppError::Validation(error.to_string())
}

pub fn parse_document(value: &Value) -> Result<VendorOidTemplate, AppError> {
    serde_json::from_value(value.clone()).map_err(validation)
}

pub fn export_document(value: &Value) -> Result<String, AppError> {
    let template = parse_document(value)?;
    let canonical = document(&template)?;
    toml::to_string_pretty(&canonical).map_err(validation)
}

impl TemplateState {
    pub fn preview(&self, request: PreviewDefinitions) -> Result<Value, AppError> {
        if !is_valid_oid(&request.sys_object_id) {
            return Err(validation("invalid numeric sysObjectID"));
        }
        let candidate = self.build(request.definitions, self.generation)?;
        let template = candidate.registry.select(Some(&request.sys_object_id));
        Ok(serde_json::json!({
            "id": profile_id(template),
            "document": document(template)?,
            "common": candidate.definitions.common,
            "generation": self.generation
        }))
    }

    pub fn new(baseline: TemplateRegistry) -> Self {
        let baseline = Arc::new(baseline);
        Self {
            registry: baseline.clone(),
            baseline,
            generation: 0,
            definitions: Definitions::default(),
            managed: false,
        }
    }

    fn build(&self, definitions: Definitions, generation: i64) -> Result<Self, AppError> {
        definitions.common.validate()?;
        if definitions.profiles.len() > 256 {
            return Err(validation("at most 256 user profiles"));
        }
        let mut templates = self.baseline.templates().to_vec();
        for (id, value) in &definitions.profiles {
            let template = parse_document(value)?;
            if id != &profile_id(&template) {
                return Err(validation("profile ID must match its selector"));
            }
            if let Some(existing) = templates.iter_mut().find(|item| profile_id(item) == *id) {
                *existing = template;
            } else {
                templates.push(template);
            }
        }
        let registry = Arc::new(TemplateRegistry::from_templates(templates).map_err(validation)?);
        Ok(Self {
            baseline: self.baseline.clone(),
            registry,
            generation,
            definitions,
            managed: true,
        })
    }

    pub fn view(&self) -> Result<Value, AppError> {
        let profiles = self.registry.templates().iter().map(|template| {
            let id = profile_id(template);
            let source = if self.definitions.profiles.contains_key(&id) {
                "database"
            } else {
                self.baseline.source(&template.selector())
            };
            Ok(serde_json::json!({
                "id": id, "source": source, "document": document(template)?,
                "resettable": self.definitions.profiles.contains_key(&profile_id(template)),
                "base": self.baseline.templates().iter().find(|item| profile_id(item) == profile_id(template)).map(document).transpose()?
            }))
        }).collect::<Result<Vec<_>, AppError>>()?;
        Ok(serde_json::json!({
            "generation": self.generation,
            "applied_generation": self.generation,
            "definitions": self.definitions,
            "profiles": profiles
        }))
    }

    pub fn load(&self, connection: &Connection, legacy: &SnmpConfig) -> Result<Self, AppError> {
        let stored = connection
            .query_row(
                "SELECT generation, document FROM snmp_definitions WHERE id = 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((generation, json)) = stored {
            return self.build(serde_json::from_str(&json).map_err(validation)?, generation);
        }
        let definitions = Definitions {
            common: CommonOids {
                cpu_oid: legacy.cpu_oid_override.trim().into(),
                memory_oid: legacy.memory_oid_override.trim().into(),
                hardware_oids: legacy.hardware_oid_overrides.clone(),
            },
            ..Definitions::default()
        };
        let state = self.build(definitions, 0)?;
        connection.execute(
            "INSERT INTO snmp_definitions(id, generation, document) VALUES (1, 0, ?1)",
            [serde_json::to_string(&state.definitions).map_err(validation)?],
        )?;
        Ok(state)
    }

    pub fn save(
        &self,
        connection: &Connection,
        request: SaveDefinitions,
    ) -> Result<Self, AppError> {
        if request.generation != self.generation {
            return Err(validation("definitions changed; reload before saving"));
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| validation("generation overflow"))?;
        let next = self.build(request.definitions, generation)?;
        let json = serde_json::to_string(&next.definitions).map_err(validation)?;
        let transaction = connection.unchecked_transaction()?;
        let affected = transaction.execute(
            "UPDATE snmp_definitions SET generation = ?1, document = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = 1 AND generation = ?3",
            params![generation, json, self.generation],
        )?;
        if affected != 1 {
            return Err(validation(
                "definitions changed in storage; restart or reload before saving",
            ));
        }
        transaction.commit()?;
        Ok(next)
    }
}

pub fn profile_id(template: &VendorOidTemplate) -> String {
    let selector = template.selector();
    if selector.is_empty() {
        "generic".into()
    } else {
        selector
    }
}

pub fn runtime() -> &'static RwLock<TemplateState> {
    static STATE: OnceLock<RwLock<TemplateState>> = OnceLock::new();
    STATE.get_or_init(|| {
        let dir = crate::data_dir().join("custom_templates");
        if let Err(error) = std::fs::create_dir_all(&dir) {
            tracing::error!(%error, "Cannot create custom template directory");
        }
        RwLock::new(TemplateState::new(TemplateRegistry::load(&dir)))
    })
}

pub fn initialize(connection: &Connection, legacy: &SnmpConfig) -> Result<(), AppError> {
    let mut state = runtime()
        .write()
        .map_err(|_| validation("template state lock poisoned"))?;
    *state = state.load(connection, legacy)?;
    Ok(())
}

pub fn client_snapshot(
    mut config: Option<SnmpConfig>,
) -> (Arc<TemplateRegistry>, Option<SnmpConfig>) {
    let state = runtime().read().expect("template state lock poisoned");
    if state.managed {
        let config = config.get_or_insert_with(SnmpConfig::default);
        state.definitions.common.apply(config);
    }
    (state.registry.clone(), config)
}
