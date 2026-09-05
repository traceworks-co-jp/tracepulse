use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VendorOidTemplate {
    pub name: String,
    pub enterprise_id: u32,
    #[serde(default)]
    pub cpu: CpuTemplate,
    #[serde(default)]
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
    pub used_prefix: String,
    #[serde(default)]
    pub free_prefix: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SensorGroupTemplate {
    #[serde(default)]
    pub descr_prefix: String,
    #[serde(default)]
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
            println!("[Template] Directory NOT found: {:?}", dir);
            return map;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => match toml::from_str::<VendorOidTemplate>(&content) {
                            Ok(template) => {
                                println!(
                                    "[Template] Loaded OID template: {} (Enterprise ID: {})",
                                    template.name, template.enterprise_id
                                );
                                map.insert(template.enterprise_id, template);
                            }
                            Err(err) => {
                                eprintln!(
                                    "[Template ERROR] Failed to parse TOML {:?}: {}",
                                    path, err
                                );
                            }
                        },
                        Err(err) => {
                            eprintln!("[Template ERROR] Failed to read file {:?}: {}", path, err);
                        }
                    }
                }
            }
        }
        map
    }
}