use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VendorOidTemplate {
    pub name: String,
    pub enterprise_id: u32,
    #[serde(default)]
    pub cpu: CpuTemplate,
    #[serde(default, alias = "metrics")]
    pub memory: MemoryTemplate,
    #[serde(default)]
    pub sensors: HashMap<String, Vec<SensorGroupTemplate>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CpuTemplate {
    #[serde(default)]
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MemoryTemplate {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub oid: String,
    #[serde(default, alias = "used_oid")]
    pub used_prefix: String,
    #[serde(default, alias = "free_oid")]
    pub free_prefix: String,
    #[serde(default, alias = "total_oid")]
    pub total_prefix: String,
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
pub struct SensorGroupTemplate {
    #[serde(default)]
    pub descr_prefix: String,
    #[serde(default, alias = "speed_prefix")]
    pub value_prefix: String,
    #[serde(default)]
    pub state_prefix: String,
    #[serde(default)]
    pub fallback_builtin_power: bool,
}

impl VendorOidTemplate {
    /// templates フォルダ配下の全 .toml ファイルを読み込み Enterprise ID ごとのマップを返す
    pub fn load_all_from_dir(dir: &Path) -> HashMap<u32, VendorOidTemplate> {
        let mut map = HashMap::new();
        if !dir.exists() || !dir.is_dir() {
            tracing::warn!("[Template] Directory NOT found: {:?}", dir);
            return map;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => match toml::from_str::<VendorOidTemplate>(&content) {
                            Ok(template) => {
                                tracing::info!(
                                    "[Template] Loaded OID template: {} (Enterprise ID: {})",
                                    template.name,
                                    template.enterprise_id
                                );
                                map.insert(template.enterprise_id, template);
                            }
                            Err(err) => {
                                tracing::error!(
                                    "[Template ERROR] Failed to parse TOML {:?}: {}",
                                    path,
                                    err
                                );
                            }
                        },
                        Err(err) => {
                            tracing::error!(
                                "[Template ERROR] Failed to read file {:?}: {}",
                                path,
                                err
                            );
                        }
                    }
                }
            }
        }
        map
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
        assert_eq!(template.memory.mode, "direct");
        assert_eq!(template.memory.oid, "1.3.6.1.4.1.12356.101.4.1.4");

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
        assert_eq!(template.memory.mode, "calculated");
        assert_eq!(template.memory.used_prefix, "1.3.6.1.4.1.9.9.48.1.1.1.5");
        assert_eq!(template.memory.free_prefix, "1.3.6.1.4.1.9.9.48.1.1.1.6");

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
    fn parses_all_vendor_templates() {
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        let templates = VendorOidTemplate::load_all_from_dir(&template_dir);

        assert!(
            templates.contains_key(&0),
            "generic template should be loaded"
        );
        assert!(
            templates.contains_key(&9),
            "Cisco template should be loaded"
        );
        assert!(templates.len() > 2, "vendor templates should be loaded");
    }
}
