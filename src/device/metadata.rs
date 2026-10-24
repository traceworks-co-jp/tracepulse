use crate::{db::models::Device, error::AppError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceMetadata {
    pub display_name: Option<String>,
    pub tags: Vec<String>,
}

impl DeviceMetadata {
    pub fn normalize(self) -> Result<Self, AppError> {
        let display_name = self
            .display_name
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty());
        if let Some(name) = &display_name {
            validate_text(name, 512, "display_name")?;
        }
        if self.tags.len() > 64 {
            return Err(invalid("at most 64 tags are allowed"));
        }
        let mut tags = Vec::new();
        for tag in self.tags {
            let tag = tag.trim().to_string();
            validate_text(&tag, 64, "tag")?;
            if tag.is_empty() {
                return Err(invalid("tags must not be empty"));
            }
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        Ok(Self { display_name, tags })
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenameSort {
    IpAsc,
    SelectionOrder,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulkRename {
    pub device_ids: Vec<i64>,
    pub prefix: String,
    #[serde(default = "default_start")]
    pub start_number: u32,
    pub padding: usize,
    pub suffix: Option<String>,
    pub sort_by: RenameSort,
}

fn default_start() -> u32 {
    1
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RenameItem {
    pub id: i64,
    pub ip: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum BulkAction {
    AddTags {
        device_ids: Vec<i64>,
        tags: Vec<String>,
    },
    Unregister {
        device_ids: Vec<i64>,
    },
}

pub fn selected_devices<'a>(
    ids: &[i64],
    devices: &'a [Device],
) -> Result<Vec<&'a Device>, AppError> {
    if ids.is_empty() || ids.len() > 1000 {
        return Err(invalid("select between 1 and 1000 devices"));
    }
    let mut seen = HashSet::new();
    ids.iter()
        .map(|id| {
            if *id <= 0 || !seen.insert(*id) {
                return Err(invalid("device_ids must be unique positive IDs"));
            }
            devices
                .iter()
                .find(|device| device.id == Some(*id))
                .ok_or_else(|| invalid(format!("device {id} was not found")))
        })
        .collect()
}

pub fn rename_plan(request: &BulkRename, devices: &[Device]) -> Result<Vec<RenameItem>, AppError> {
    let devices = selected_devices(&request.device_ids, devices)?;
    if request.padding > 10 {
        return Err(invalid("padding must be between 0 and 10"));
    }
    validate_text(&request.prefix, 512, "prefix")?;
    validate_text(request.suffix.as_deref().unwrap_or(""), 512, "suffix")?;
    request
        .start_number
        .checked_add(request.device_ids.len() as u32 - 1)
        .ok_or_else(|| invalid("sequence exceeds u32 maximum"))?;
    let mut selected = Vec::new();
    for device in devices {
        let ip = device
            .ip
            .parse::<std::net::IpAddr>()
            .map_err(|_| invalid(format!("device {:?} has an invalid IP address", device.id)))?;
        selected.push((device, ip));
    }
    if matches!(request.sort_by, RenameSort::IpAsc) {
        selected.sort_by_key(|(device, ip)| (*ip, device.id));
    }
    selected
        .into_iter()
        .enumerate()
        .map(|(offset, (device, _))| {
            let name = format!(
                "{}{:0width$}{}",
                request.prefix,
                request.start_number + offset as u32,
                request.suffix.as_deref().unwrap_or(""),
                width = request.padding
            );
            validate_text(&name, 512, "generated display_name")?;
            if name.trim().is_empty() {
                return Err(invalid("generated name must not be empty"));
            }
            Ok(RenameItem {
                id: device.id.expect("selected persisted device"),
                ip: device.ip.clone(),
                display_name: name,
            })
        })
        .collect()
}

fn validate_text(value: &str, max: usize, field: &str) -> Result<(), AppError> {
    if value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(invalid(format!(
            "{field} must contain at most {max} characters and no control characters"
        )));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}
