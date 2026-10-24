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

pub fn rename_plan(request: &BulkRename, devices: &[Device]) -> Result<Vec<RenameItem>, AppError> {
    if request.device_ids.is_empty() || request.device_ids.len() > 1000 {
        return Err(invalid("select between 1 and 1000 devices"));
    }
    if request.padding > 10 {
        return Err(invalid("padding must be between 0 and 10"));
    }
    validate_text(&request.prefix, 512, "prefix")?;
    validate_text(request.suffix.as_deref().unwrap_or(""), 512, "suffix")?;
    request
        .start_number
        .checked_add(request.device_ids.len() as u32 - 1)
        .ok_or_else(|| invalid("sequence exceeds u32 maximum"))?;
    let mut ids = HashSet::new();
    let mut selected = Vec::new();
    for id in &request.device_ids {
        if *id <= 0 || !ids.insert(*id) {
            return Err(invalid("device_ids must be unique positive IDs"));
        }
        let device = devices
            .iter()
            .find(|device| device.id == Some(*id))
            .ok_or_else(|| invalid(format!("device {id} was not found")))?;
        let ip = device
            .ip
            .parse::<std::net::IpAddr>()
            .map_err(|_| invalid(format!("device {id} has an invalid IP address")))?;
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
