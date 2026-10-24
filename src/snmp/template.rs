use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct VendorOidTemplate {
    pub name: String,
    pub description: String,
    pub sys_object_id_prefix: Option<String>,
    pub enterprise_id: u32,
    pub cpu: CpuTemplate,
    pub memory: MemoryTemplate,
    pub sensors: HashMap<String, Vec<SensorGroupTemplate>>,
    pub metrics: Vec<PredictiveMetricTemplate>,
    pub numeric_metrics: Vec<NumericMetricTemplate>,
    pub tables: TemplateTables,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VendorOidTemplateInput {
    #[serde(alias = "vendor_name")]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default, rename = "sysObjectID_prefix")]
    sys_object_id_prefix: Option<String>,
    #[serde(default)]
    enterprise_id: u32,
    #[serde(default)]
    cpu: CpuTemplate,
    #[serde(default)]
    memory: Option<MemoryTemplate>,
    #[serde(default)]
    sensors: HashMap<String, Vec<SensorGroupTemplate>>,
    #[serde(default)]
    metrics: Option<toml::Value>,
    #[serde(default)]
    tables: TemplateTables,
}

impl<'de> Deserialize<'de> for VendorOidTemplate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let input = VendorOidTemplateInput::deserialize(deserializer)?;
        let memory_supplied = input.memory.is_some();
        let mut memory = input.memory.unwrap_or_default();
        let mut metrics = Vec::new();
        let mut numeric_metrics = Vec::new();
        if let Some(value) = input.metrics {
            match value {
                toml::Value::Array(values) => {
                    for value in values {
                        if value.get("key").is_some() {
                            numeric_metrics.push(value.try_into().map_err(|error| {
                                D::Error::custom(format!("invalid numeric metric: {error}"))
                            })?);
                        } else {
                            metrics.push(value.try_into().map_err(|error| {
                                D::Error::custom(format!("invalid predictive metric: {error}"))
                            })?);
                        }
                    }
                }
                toml::Value::Table(_) => {
                    if memory_supplied {
                        return Err(D::Error::custom(
                            "both [memory] and legacy [metrics] were supplied",
                        ));
                    }
                    memory = value.try_into().map_err(|error| {
                        D::Error::custom(format!("invalid legacy memory metrics: {error}"))
                    })?;
                }
                _ => {
                    return Err(D::Error::custom(
                        "metrics must be an array of predictive metric tables",
                    ));
                }
            }
        }
        let template = Self {
            name: input.name,
            description: input.description,
            sys_object_id_prefix: input.sys_object_id_prefix,
            enterprise_id: input.enterprise_id,
            cpu: input.cpu,
            memory,
            sensors: input.sensors,
            metrics,
            numeric_metrics,
            tables: input.tables,
        };
        template.validate().map_err(D::Error::custom)?;
        Ok(template)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NumericMetricTemplate {
    pub key: String,
    pub oid: String,
    pub unit: String,
    pub label: String,
    pub data_type: NumericDataType,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NumericDataType {
    Gauge,
    Counter,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateTables {
    pub interfaces: Option<InterfaceTableTemplate>,
    pub dom: Option<DomTableTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DomTableTemplate {
    pub base_oid: String,
    /// Omit when the row suffix itself is the ifIndex (e.g. Juniper).
    pub index_oid: Option<String>,
    pub entity: Option<DomEntityTemplate>,
    pub metrics: Vec<DomMetricTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DomEntityTemplate {
    pub name_oid: String,
    pub contained_in_oid: String,
    pub alias_oid: String,
    pub type_oid: String,
    pub scale_oid: String,
    pub precision_oid: String,
    pub status_oid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DomMetricTemplate {
    pub key: String,
    pub oid: String,
    pub unit: String,
    pub label: String,
    #[serde(default = "default_metric_scale")]
    pub scale: f64,
    #[serde(default)]
    pub invalid_values: Vec<f64>,
    #[serde(default)]
    pub match_names: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceTableTemplate {
    pub base_oid: String,
    pub index: String,
    pub name: String,
    pub in_octets: String,
    pub out_octets: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PredictiveMetricTemplate {
    pub name: String,
    pub category: String,
    pub oid: String,
    pub unit: String,
    pub rule: String,
    #[serde(default)]
    pub instance: MetricInstance,
    #[serde(default)]
    pub name_oid: Option<String>,
    #[serde(default)]
    pub match_name: Option<String>,
    #[serde(default)]
    pub unit_oid: Option<String>,
    #[serde(default)]
    pub match_unit: Option<String>,
    #[serde(default)]
    pub scale_oid: Option<String>,
    #[serde(default)]
    pub precision_oid: Option<String>,
    #[serde(default = "default_metric_scale")]
    pub scale: f64,
    #[serde(default)]
    pub offset: f64,
    #[serde(default)]
    pub warning_value: Option<f64>,
    #[serde(default)]
    pub min_slope_per_day: f64,
    #[serde(default = "default_metric_forecast_days")]
    pub forecast_days: f64,
    #[serde(default = "default_metric_min_samples")]
    pub min_samples: usize,
    #[serde(default = "default_metric_min_period_days")]
    pub min_period_days: f64,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricInstance {
    #[default]
    Scalar,
    Table,
}

impl PredictiveMetricTemplate {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("metric name must not be empty".into());
        }
        if self.unit.trim().is_empty() {
            return Err(format!("metric '{}' must define a unit", self.name));
        }
        if !matches!(
            self.category.as_str(),
            "optical" | "thermal" | "fan" | "power" | "interface"
        ) {
            return Err(format!("unsupported metric category '{}'", self.category));
        }
        if !matches!(
            self.rule.as_str(),
            "trend_decrease" | "trend_increase" | "approach_lower" | "approach_upper"
        ) {
            return Err(format!("unsupported metric rule '{}'", self.rule));
        }
        if !is_valid_oid(&self.oid) {
            return Err(format!("invalid OID '{}'", self.oid));
        }
        if !self.scale.is_finite()
            || self.scale == 0.0
            || !self.offset.is_finite()
            || !self.min_slope_per_day.is_finite()
            || self.min_slope_per_day < 0.0
            || !self.forecast_days.is_finite()
            || self.forecast_days <= 0.0
            || !self.min_period_days.is_finite()
            || self.min_period_days <= 0.0
            || self.min_samples < 3
            || self.warning_value.is_some_and(|value| !value.is_finite())
        {
            return Err(format!(
                "invalid thresholds or scale for metric '{}'",
                self.name
            ));
        }
        if matches!(self.rule.as_str(), "approach_lower" | "approach_upper")
            && self.warning_value.is_none()
        {
            return Err(format!(
                "metric '{}' requires warning_value for rule '{}'",
                self.name, self.rule
            ));
        }
        if self.instance == MetricInstance::Table
            && self
                .name_oid
                .as_deref()
                .is_some_and(|oid| !is_valid_oid(oid))
        {
            return Err(format!("invalid name_oid for metric '{}'", self.name));
        }
        if self.instance == MetricInstance::Table
            && self
                .unit_oid
                .as_deref()
                .is_some_and(|oid| !is_valid_oid(oid))
        {
            return Err(format!("invalid unit_oid for metric '{}'", self.name));
        }
        if self.match_name.is_some() && self.name_oid.is_none() {
            return Err(format!(
                "metric '{}' requires name_oid when match_name is configured",
                self.name
            ));
        }
        if self.match_unit.is_some() && self.unit_oid.is_none() {
            return Err(format!(
                "metric '{}' requires unit_oid when match_unit is configured",
                self.name
            ));
        }
        for oid in [
            self.scale_oid.as_deref(),
            self.precision_oid.as_deref(),
            self.unit_oid.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if !is_valid_oid(oid) {
                return Err(format!("invalid scale/precision OID for '{}'", self.name));
            }
        }
        Ok(())
    }
}

pub(crate) fn is_valid_oid(value: &str) -> bool {
    let oid = value.strip_prefix('.').unwrap_or(value);
    if !oid
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return false;
    }
    let parts = oid
        .split('.')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>();
    parts.is_ok_and(|parts| {
        (2..=128).contains(&parts.len()) && parts[0] <= 2 && (parts[0] == 2 || parts[1] <= 39)
    })
}

fn default_metric_scale() -> f64 {
    1.0
}

fn default_metric_forecast_days() -> f64 {
    30.0
}

fn default_metric_min_samples() -> usize {
    5
}

fn default_metric_min_period_days() -> f64 {
    1.0
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CpuTemplate {
    #[serde(default)]
    pub candidates: Vec<String>,
    #[serde(default)]
    pub aggregation: PercentageAggregation,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PercentageAggregation {
    #[default]
    First,
    Max,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryTemplate {
    #[serde(flatten)]
    pub primary: MemorySource,
    #[serde(default)]
    pub fallbacks: Vec<MemorySource>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySource {
    #[serde(default)]
    pub mode: String,
    #[serde(default, alias = "util_prefix")]
    pub oid: String,
    #[serde(default, alias = "used_oid")]
    pub used_prefix: String,
    #[serde(default, alias = "free_oid")]
    pub free_prefix: String,
    #[serde(default, alias = "total_oid")]
    pub total_prefix: String,
    #[serde(default)]
    pub aggregation: PercentageAggregation,
    #[serde(default)]
    pub used_is_bytes: bool,
}

impl MemoryTemplate {
    pub fn sources(&self) -> impl Iterator<Item = &MemorySource> {
        std::iter::once(&self.primary).chain(&self.fallbacks)
    }
}

impl MemoryTemplate {
    pub fn calculate_utilization(
        mode: &str,
        direct_val: Option<f64>,
        used_val: Option<f64>,
        free_val: Option<f64>,
        total_val: Option<f64>,
    ) -> Option<f64> {
        let mode_clean = mode.trim().to_lowercase();
        if mode_clean == "direct" {
            direct_val.map(|v| v.clamp(0.0, 100.0))
        } else {
            // "calculated" または デフォルト (Used + Free / Used + Total)
            if let (Some(used), Some(free)) = (used_val, free_val) {
                let sum = used + free;
                if sum == 0.0 {
                    Some(0.0)
                } else {
                    Some(((used / sum) * 100.0).clamp(0.0, 100.0))
                }
            } else if let (Some(used), Some(total)) = (used_val, total_val) {
                if total == 0.0 {
                    Some(0.0)
                } else {
                    Some(((used / total) * 100.0).clamp(0.0, 100.0))
                }
            } else {
                direct_val.map(|v| v.clamp(0.0, 100.0))
            }
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SensorGroupTemplate {
    #[serde(default)]
    pub descr_prefix: String,
    #[serde(default, alias = "speed_prefix")]
    pub value_prefix: String,
    #[serde(default)]
    pub state_prefix: String,
    #[serde(default)]
    pub fallback_builtin_power: bool,
    #[serde(default)]
    pub alarm_states: Option<Vec<i64>>,
    #[serde(default)]
    pub normal_states: Vec<i64>,
    #[serde(default)]
    pub state_labels: HashMap<String, String>,
    #[serde(default)]
    pub match_names: Vec<String>,
}

impl VendorOidTemplate {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("template name must not be empty".into());
        }
        let mut oids: Vec<&str> = self.cpu.candidates.iter().map(String::as_str).collect();
        if let Some(prefix) = &self.sys_object_id_prefix {
            oids.push(prefix.strip_suffix('.').unwrap_or(prefix));
        }
        for memory in self.memory.sources() {
            for oid in [
                &memory.oid,
                &memory.used_prefix,
                &memory.free_prefix,
                &memory.total_prefix,
            ] {
                if !oid.is_empty() {
                    oids.push(oid);
                }
            }
            if !matches!(memory.mode.as_str(), "" | "direct" | "calculated") {
                return Err("memory mode must be direct or calculated".into());
            }
            if memory.mode == "direct" && memory.oid.is_empty() {
                return Err("direct memory mode requires oid".into());
            }
            if memory.mode == "calculated"
                && (memory.used_prefix.is_empty()
                    || (memory.free_prefix.is_empty() && memory.total_prefix.is_empty()))
            {
                return Err(
                    "calculated memory mode requires used_prefix and free_prefix or total_prefix"
                        .into(),
                );
            }
            if memory.used_is_bytes && memory.mode != "calculated" {
                return Err("used_is_bytes requires calculated memory mode".into());
            }
            if memory.mode != "direct" && memory.aggregation != PercentageAggregation::First {
                return Err("memory aggregation is only supported in direct mode".into());
            }
        }
        for group in self.sensors.values().flatten() {
            for oid in [
                &group.descr_prefix,
                &group.value_prefix,
                &group.state_prefix,
            ] {
                if !oid.is_empty() {
                    oids.push(oid);
                }
            }
            if group.fallback_builtin_power {
                return Err("synthetic built-in power readings are not supported".into());
            }
            if group.state_labels.iter().any(|(key, label)| {
                key.parse::<i64>()
                    .map_or(true, |value| value.to_string() != *key)
                    || label.trim().is_empty()
            }) || group.normal_states.iter().any(|value| {
                group
                    .alarm_states
                    .as_deref()
                    .unwrap_or(&[2, 3, 4, 6])
                    .contains(value)
            }) || group.match_names.iter().any(|name| name.trim().is_empty())
                || (group.state_prefix.is_empty()
                    && (!group.state_labels.is_empty()
                        || group.alarm_states.is_some()
                        || !group.normal_states.is_empty()))
            {
                return Err("invalid sensor state mapping or name filter".into());
            }
        }
        let mut keys = std::collections::HashSet::new();
        for metric in &self.numeric_metrics {
            if metric.key.is_empty()
                || !metric
                    .key
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_')
                || !keys.insert(&metric.key)
                || metric.label.trim().is_empty()
            {
                return Err(format!("invalid or duplicate metric key '{}'", metric.key));
            }
            oids.push(&metric.oid);
        }
        if let Some(table) = &self.tables.interfaces {
            oids.extend([
                table.base_oid.as_str(),
                table.index.as_str(),
                table.name.as_str(),
                table.in_octets.as_str(),
                table.out_octets.as_str(),
            ]);
            for column in [
                &table.index,
                &table.name,
                &table.in_octets,
                &table.out_octets,
            ] {
                if !oid_has_prefix(column, &table.base_oid)
                    || column.trim_start_matches('.') == table.base_oid.trim_start_matches('.')
                {
                    return Err(format!("interface column '{column}' is outside base_oid"));
                }
            }
        }
        for oid in oids {
            if !is_valid_oid(oid) {
                return Err(format!("invalid numeric OID '{oid}'"));
            }
        }
        if let Some(dom) = &self.tables.dom {
            let mut dom_oids = vec![dom.base_oid.as_str()];
            if dom.index_oid.is_some() && dom.entity.is_some() {
                return Err("DOM index_oid and entity mapping are mutually exclusive".into());
            }
            if let Some(oid) = &dom.index_oid {
                dom_oids.push(oid);
            }
            if let Some(entity) = &dom.entity {
                dom_oids.extend([
                    entity.name_oid.as_str(),
                    &entity.contained_in_oid,
                    &entity.alias_oid,
                    &entity.type_oid,
                    &entity.scale_oid,
                    &entity.precision_oid,
                    &entity.status_oid,
                ]);
            }
            let mut keys = std::collections::HashSet::new();
            if dom.metrics.is_empty() {
                return Err("DOM requires metrics".into());
            }
            for metric in &dom.metrics {
                if !matches!(metric.key.as_str(), "rx_power" | "tx_power")
                    || !keys.insert(&metric.key)
                    || metric.unit != "dBm"
                    || metric.label.trim().is_empty()
                    || !metric.scale.is_finite()
                    || metric.scale <= 0.0
                    || metric.invalid_values.iter().any(|value| !value.is_finite())
                    || metric.match_names.iter().any(|name| name.trim().is_empty())
                    || (dom.entity.is_some() && metric.match_names.is_empty())
                    || (dom.entity.is_none() && !metric.match_names.is_empty())
                {
                    return Err(format!("invalid DOM metric '{}'", metric.key));
                }
                if !oid_has_prefix(&metric.oid, &dom.base_oid)
                    || metric.oid.trim_start_matches('.') == dom.base_oid.trim_start_matches('.')
                {
                    return Err("DOM metric OID must be a column within base_oid".into());
                }
                dom_oids.push(&metric.oid);
            }
            if dom_oids.iter().any(|oid| !is_valid_oid(oid)) {
                return Err("invalid numeric DOM OID".into());
            }
        }
        for metric in &self.metrics {
            metric.validate()?;
        }
        Ok(())
    }
}

fn oid_has_prefix(oid: &str, prefix: &str) -> bool {
    let oid = oid.trim_start_matches('.');
    let prefix = prefix.trim_matches('.');
    oid == prefix
        || oid
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

#[derive(Debug, Clone)]
pub struct TemplateRegistry {
    templates: Vec<VendorOidTemplate>,
}

impl TemplateRegistry {
    pub fn load(dir: &Path) -> Self {
        let templates = [
            include_str!("../../templates/generic.toml"),
            include_str!("../../templates/cisco.toml"),
            include_str!("../../templates/yamaha.toml"),
            include_str!("../../templates/juniper.toml"),
        ]
        .into_iter()
        .map(|content| toml::from_str(content).expect("bundled template must be valid"))
        .collect();
        let mut registry = Self { templates };
        let mut paths = match std::fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(|entry| match entry {
                    Ok(entry) => Some(entry.path()),
                    Err(error) => {
                        tracing::error!("[Template] Cannot read entry in {:?}: {}", dir, error);
                        None
                    }
                })
                .collect::<Vec<_>>(),
            Err(error) => {
                tracing::error!(
                    "[Template] Cannot read custom directory {:?}: {}",
                    dir,
                    error
                );
                return registry;
            }
        };
        paths.sort();
        let mut custom_selectors = std::collections::HashSet::new();
        for path in paths {
            if path.extension().and_then(|value| value.to_str()) != Some("toml") {
                continue;
            }
            let result = std::fs::read_to_string(&path)
                .map_err(|error| error.to_string())
                .and_then(|content| {
                    toml::from_str::<VendorOidTemplate>(&content).map_err(|error| error.to_string())
                });
            match result {
                Ok(template) => {
                    let selector = template.selector();
                    if selector.is_empty() {
                        tracing::error!(
                            "[Template] Custom template {:?} requires sysObjectID_prefix or enterprise_id",
                            path
                        );
                        continue;
                    }
                    if !custom_selectors.insert(selector.clone()) {
                        tracing::error!(
                            "[Template] Duplicate custom selector '{}' in {:?}; ignored",
                            selector,
                            path
                        );
                        continue;
                    }
                    tracing::info!("[Template] Loaded {:?}: {}", path, template.name);
                    if let Some(existing) = registry
                        .templates
                        .iter_mut()
                        .find(|existing| existing.selector() == selector)
                    {
                        tracing::warn!(
                            "[Template] Replacing selector '{}' from {:?}",
                            selector,
                            path
                        );
                        *existing = template;
                    } else {
                        registry.templates.push(template);
                    }
                }
                Err(error) => tracing::error!("[Template] Invalid {:?}: {}", path, error),
            }
        }
        registry
    }

    pub fn select(&self, sys_object_id: Option<&str>) -> &VendorOidTemplate {
        sys_object_id
            .filter(|oid| is_valid_oid(oid))
            .and_then(|oid| {
                self.templates
                    .iter()
                    .filter(|template| {
                        let prefix = template.selector();
                        !prefix.is_empty() && oid_has_prefix(oid, &prefix)
                    })
                    .max_by_key(|template| template.selector().split('.').count())
            })
            .unwrap_or(&self.templates[0])
    }
}

impl VendorOidTemplate {
    fn selector(&self) -> String {
        self.sys_object_id_prefix
            .as_deref()
            .map(|prefix| prefix.trim_matches('.').to_string())
            .unwrap_or_else(|| {
                if self.enterprise_id == 0 {
                    String::new()
                } else {
                    format!("1.3.6.1.4.1.{}", self.enterprise_id)
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct_memory_metric() {
        let toml_str = r#"
            name = "Fortinet"
            enterprise_id = 12356
            [memory]
            mode = "direct"
            oid = "1.3.6.1.4.1.12356.101.4.1.4"
        "#;
        let template: VendorOidTemplate = toml::from_str(toml_str).unwrap();
        assert_eq!(template.memory.primary.mode, "direct");
        assert_eq!(template.memory.primary.oid, "1.3.6.1.4.1.12356.101.4.1.4");

        let util = MemoryTemplate::calculate_utilization("direct", Some(45.0), None, None, None);
        assert_eq!(util, Some(45.0));
    }

    #[test]
    fn test_parse_calculated_memory_metric() {
        let toml_str = r#"
            name = "Cisco Systems"
            enterprise_id = 9
            [memory]
            mode = "calculated"
            used_oid = "1.3.6.1.4.1.9.9.48.1.1.1.5"
            free_oid = "1.3.6.1.4.1.9.9.48.1.1.1.6"
        "#;
        let template: VendorOidTemplate = toml::from_str(toml_str).unwrap();
        assert_eq!(template.memory.primary.mode, "calculated");
        assert_eq!(
            template.memory.primary.used_prefix,
            "1.3.6.1.4.1.9.9.48.1.1.1.5"
        );
        assert_eq!(
            template.memory.primary.free_prefix,
            "1.3.6.1.4.1.9.9.48.1.1.1.6"
        );

        // Used=42MB, Free=58MB -> 42.0%
        let util =
            MemoryTemplate::calculate_utilization("calculated", None, Some(42.0), Some(58.0), None);
        assert_eq!(util, Some(42.0));
    }

    #[test]
    fn test_calculated_memory_zero_division() {
        // used + free == 0 -> Some(0.0)
        let util =
            MemoryTemplate::calculate_utilization("calculated", None, Some(0.0), Some(0.0), None);
        assert_eq!(util, Some(0.0));

        // total == 0 -> Some(0.0)
        let util_total =
            MemoryTemplate::calculate_utilization("calculated", None, Some(0.0), None, Some(0.0));
        assert_eq!(util_total, Some(0.0));
    }

    #[test]
    fn parses_and_validates_predictive_metric_profiles() {
        let profile: VendorOidTemplate = toml::from_str(
            r#"
                vendor_name = "Example Vendor"
                enterprise_id = 65000

                [[metrics]]
                name = "chassis_temperature"
                category = "thermal"
                oid = "1.3.6.1.2.1.99.1.1.1.4"
                unit = "celsius"
                rule = "trend_increase"
                instance = "table"
                name_oid = "1.3.6.1.2.1.47.1.1.1.1.7"
                match_name = "temp"
            "#,
        )
        .unwrap();
        assert_eq!(profile.name, "Example Vendor");
        assert_eq!(profile.metrics.len(), 1);
        assert!(profile.metrics[0].validate().is_ok());
    }

    #[test]
    fn retains_legacy_metrics_table_as_memory_template() {
        let profile: VendorOidTemplate = toml::from_str(
            r#"
                name = "Legacy Vendor"
                enterprise_id = 65001
                [metrics]
                mode = "direct"
                oid = "1.3.6.1.4.1.65001.1.0"
            "#,
        )
        .unwrap();
        assert_eq!(profile.memory.primary.mode, "direct");
        assert_eq!(profile.memory.primary.oid, "1.3.6.1.4.1.65001.1.0");
        assert!(profile.metrics.is_empty());
    }

    #[test]
    fn rejects_predictive_metrics_with_invalid_oids_or_rules() {
        let mut profile: VendorOidTemplate = toml::from_str(
            r#"
                name = "Example Vendor"
                enterprise_id = 65000
                [[metrics]]
                name = "bad_metric"
                category = "thermal"
                oid = "1.3.6.1.2.1.1.1"
                unit = "celsius"
                rule = "trend_increase"
            "#,
        )
        .unwrap();
        assert!(profile.metrics[0].validate().is_ok());
        profile.metrics[0].oid = "1..3".into();
        assert!(profile.metrics[0].validate().is_err());
        profile.metrics[0].oid = "1.3.6.1.2.1.1.1".into();
        profile.metrics[0].rule = "linear_ai".into();
        assert!(profile.metrics[0].validate().is_err());
    }

    #[test]
    fn bundles_standard_and_approved_vendor_definitions() {
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        let files: Vec<_> = std::fs::read_dir(template_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
            .collect();
        assert_eq!(files.len(), 4);
        for file in &files {
            toml::from_str::<VendorOidTemplate>(&std::fs::read_to_string(file).unwrap()).unwrap();
        }
        let content = std::fs::read_to_string(
            files
                .iter()
                .find(|file| file.file_name().unwrap() == "generic.toml")
                .unwrap(),
        )
        .unwrap();
        let generic: VendorOidTemplate = toml::from_str(&content).unwrap();
        assert_eq!(generic.enterprise_id, 0);
        assert!(!content.contains("1.3.6.1.4.1."));
        assert!(generic.tables.interfaces.is_some());
        let registry = TemplateRegistry::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("templates")
                .join("absent"),
        );
        for (id, name) in [
            (9, "Cisco Network Device"),
            (1182, "Yamaha RT Series"),
            (2636, "Juniper Network Device"),
        ] {
            assert_eq!(
                registry.select(Some(&format!("1.3.6.1.4.1.{id}.1"))).name,
                name
            );
        }
        assert_eq!(
            registry.select(Some("1.3.6.1.4.1.999999.1")).enterprise_id,
            0
        );
    }

    #[test]
    fn custom_vendor_override_preserves_rfc_fallback_and_first_custom_wins() {
        let dir = TestDirectory::new();
        dir.write(
            "a.toml",
            "name = 'My Cisco'\nenterprise_id = 9\n[cpu]\ncandidates = ['1.3.6.1.4.1.9.1.0']",
        );
        dir.write(
            "b.toml",
            "name = 'Duplicate'\nsysObjectID_prefix = '.1.3.6.1.4.1.9.'",
        );
        dir.write(
            "model.toml",
            "name = 'My Cisco model'\nsysObjectID_prefix = '1.3.6.1.4.1.9.1.2'",
        );
        let registry = TemplateRegistry::load(&dir.0);
        assert_eq!(registry.select(Some("1.3.6.1.4.1.9.1")).name, "My Cisco");
        assert_eq!(
            registry.select(Some("1.3.6.1.4.1.9.1.2.3")).name,
            "My Cisco model"
        );
        assert_eq!(registry.select(None).enterprise_id, 0);
        assert_eq!(
            registry.select(Some("1.3.6.1.4.1.2636.1")).enterprise_id,
            2636
        );
    }

    #[test]
    fn validates_memory_fallbacks_and_sensor_state_configuration() {
        for suffix in [
            "[cpu]\naggregation = 'average'",
            "[memory]\nunknown = true",
            "[memory]\nmode = 'direct'\noid = '1.3.6.1.2.1.1.3.0'\nused_is_bytes = true",
            "[[memory.fallbacks]]\nmode = 'direct'",
            "[[memory.fallbacks]]\nmode = 'calculated'\nused_prefix = 'symbolic'\nfree_prefix = '1.3.6.1.2.1.1.3.0'",
            "[[sensors.power]]\nstate_labels = { bad = 'Bad' }",
            "[[sensors.power]]\nalarm_states = [2]\nnormal_states = [2]",
            "[[sensors.power]]\nmatch_names = ['']",
        ] {
            assert!(
                toml::from_str::<VendorOidTemplate>(&format!("name = 'Bad'\n{suffix}")).is_err(),
                "{suffix}"
            );
        }
        let vendor: VendorOidTemplate =
            toml::from_str(include_str!("../../templates/cisco.toml")).unwrap();
        assert_eq!(vendor.memory.sources().count(), 2);
        assert!(vendor.memory.primary.used_is_bytes);
        assert_eq!(vendor.sensors["power"][0].normal_states, vec![2]);
    }

    struct TestDirectory(std::path::PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "tracepulse-templates-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn write(&self, name: &str, content: &str) {
            std::fs::write(self.0.join(name), content).unwrap();
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn loads_custom_metrics_and_selects_longest_oid_arc_prefix() {
        let dir = TestDirectory::new();
        dir.write(
            "vendor.toml",
            r#"
name = "Example"
description = "User definition"
sysObjectID_prefix = "1.3.6.1.4.1.65000."
[[metrics]]
key = "cpu_usage"
oid = "1.3.6.1.4.1.65000.1.0"
unit = "percent"
label = "CPU Utilization"
data_type = "gauge"
"#,
        );
        dir.write(
            "model.toml",
            "name = 'Model'\nsysObjectID_prefix = '.1.3.6.1.4.1.65000.2'",
        );
        let registry = TemplateRegistry::load(&dir.0);
        let vendor = registry.select(Some("1.3.6.1.4.1.65000.1"));
        assert_eq!(vendor.name, "Example");
        assert_eq!(vendor.numeric_metrics[0].key, "cpu_usage");
        assert_eq!(vendor.numeric_metrics[0].data_type, NumericDataType::Gauge);
        assert_eq!(registry.select(Some("1.3.6.1.4.1.65000.2.3")).name, "Model");
        assert_eq!(
            registry.select(Some("1.3.6.1.4.1.650000.2")).enterprise_id,
            0
        );
        assert_eq!(registry.select(None).enterprise_id, 0);
        assert_eq!(registry.select(Some("1..3")).enterprise_id, 0);
    }

    #[test]
    fn ignores_invalid_files_and_resolves_duplicates_deterministically() {
        let dir = TestDirectory::new();
        dir.write("a.toml", "name = 'First'\nenterprise_id = 65000");
        dir.write(
            "b.toml",
            "name = 'Second'\nsysObjectID_prefix = '1.3.6.1.4.1.65000.'",
        );
        dir.write("bad.toml", "name = ");
        dir.write("raw.mib", "not TOML");
        dir.write("raw.txt", "not TOML");
        dir.write("fallback.toml", "name = 'Cannot replace RFC fallback'");
        dir.write(
            "symbolic.toml",
            "name = 'Bad'\nenterprise_id = 65001\n[cpu]\ncandidates = ['IF-MIB::ifIndex']",
        );
        let registry = TemplateRegistry::load(&dir.0);
        assert_eq!(registry.templates.len(), 5);
        assert_eq!(registry.select(Some("1.3.6.1.4.1.65000.1")).name, "First");
        assert_eq!(
            registry.select(Some("1.3.6.1.4.1.65001.1")).enterprise_id,
            0
        );
    }

    #[test]
    fn validates_all_oid_fields_and_metric_shape() {
        for invalid in [
            "IF-MIB::ifIndex",
            "1..3",
            "1.3.x.0",
            "3.1.0",
            "1.40.0",
            "1.3.4294967296",
            "1.3.",
        ] {
            let content = format!(
                "name = 'Bad'\n[[metrics]]\nkey = 'test'\nlabel = 'Test'\nunit = 'count'\ndata_type = 'gauge'\noid = '{invalid}'"
            );
            assert!(
                toml::from_str::<VendorOidTemplate>(&content).is_err(),
                "{invalid}"
            );
        }
        let metric = "[[metrics]]\nkey = 'test'\nlabel = 'Test'\nunit = 'count'\ndata_type = 'gauge'\noid = '1.3.6.1.2.1.1.3.0'";
        assert!(toml::from_str::<VendorOidTemplate>(&format!("name = 'Valid'\n{metric}")).is_ok());
        assert!(
            toml::from_str::<VendorOidTemplate>(&format!("name = 'Bad'\n{metric}\n{metric}"))
                .is_err()
        );
        assert!(
            toml::from_str::<VendorOidTemplate>(&format!(
                "name = 'Bad'\n{}",
                metric.replace("'gauge'", "'unknown'")
            ))
            .is_err()
        );
        for suffix in [
            "sysObjectID_prefix = '1.3.x'",
            "[cpu]\ncandidates = ['1..3']",
            "[memory]\noid = 'symbolic'",
            "[[sensors.fan]]\nvalue_prefix = 'symbolic'",
            "[tables.interfaces]\nbase_oid = '1.3.6.1.2.1.2.2.1'\nindex = '1.3.6.1.4.1.1'\nname = '1.3.6.1.2.1.2.2.1.2'\nin_octets = '1.3.6.1.2.1.2.2.1.10'\nout_octets = '1.3.6.1.2.1.2.2.1.16'",
        ] {
            assert!(
                toml::from_str::<VendorOidTemplate>(&format!("name = 'Bad'\n{suffix}")).is_err()
            );
        }
    }
}
