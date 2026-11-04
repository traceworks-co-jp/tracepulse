use crate::{config::SnmpConfig, error::AppError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SnmpCredentials {
    pub version: Option<String>,
    pub community: Option<String>,
    pub v3_username: Option<String>,
    pub v3_auth_protocol: Option<String>,
    pub v3_auth_password: Option<String>,
    pub v3_priv_protocol: Option<String>,
    pub v3_priv_password: Option<String>,
}

impl std::fmt::Debug for SnmpCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SnmpCredentials([REDACTED])")
    }
}

impl SnmpCredentials {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        for value in [
            &mut self.version,
            &mut self.community,
            &mut self.v3_username,
            &mut self.v3_auth_protocol,
            &mut self.v3_auth_password,
            &mut self.v3_priv_protocol,
            &mut self.v3_priv_password,
        ] {
            if value.as_ref().is_some_and(|value| value.trim().is_empty()) {
                *value = None;
            }
            if value
                .as_ref()
                .is_some_and(|value| value == crate::crypto::MASK || value.len() > 1024)
            {
                return Err(AppError::Validation(
                    "invalid SNMP credential (contents redacted)".into(),
                ));
            }
        }
        for (value, allowed) in [
            (self.version.as_deref(), &["2c", "3"][..]),
            (
                self.v3_auth_protocol.as_deref(),
                &["none", "sha1", "sha256", "sha512"][..],
            ),
            (self.v3_priv_protocol.as_deref(), &["none", "aes128"][..]),
        ] {
            if value.is_some_and(|value| !allowed.contains(&value)) {
                return Err(AppError::Validation(
                    "unsupported SNMP security protocol".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn resolve(&self, global: &SnmpConfig, legacy_community: &str) -> Self {
        let defaults = &global.credentials;
        let choose = |individual: &Option<String>, default: &Option<String>| {
            individual
                .clone()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| default.clone().filter(|value| !value.trim().is_empty()))
        };
        Self {
            version: choose(&self.version, &defaults.version).or(Some("2c".into())),
            community: self
                .community
                .clone()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| (!legacy_community.trim().is_empty()).then(|| legacy_community.into()))
                .or_else(|| Some(global.default_community.clone())),
            v3_username: choose(&self.v3_username, &defaults.v3_username),
            v3_auth_protocol: choose(&self.v3_auth_protocol, &defaults.v3_auth_protocol)
                .or(Some("sha256".into())),
            v3_auth_password: choose(&self.v3_auth_password, &defaults.v3_auth_password),
            v3_priv_protocol: choose(&self.v3_priv_protocol, &defaults.v3_priv_protocol)
                .or(Some("aes128".into())),
            v3_priv_password: choose(&self.v3_priv_password, &defaults.v3_priv_password),
        }
    }

    pub fn validate_resolved(&self) -> Result<(), AppError> {
        if self.version.as_deref() != Some("3") {
            return Ok(());
        }
        let auth = self.v3_auth_protocol.as_deref().unwrap_or("sha256");
        let privacy = self.v3_priv_protocol.as_deref().unwrap_or("aes128");
        if self
            .v3_username
            .as_ref()
            .is_none_or(|value| value.is_empty() || value.len() > 32)
            || (auth != "none"
                && self
                    .v3_auth_password
                    .as_ref()
                    .is_none_or(|value| value.len() < 8))
            || (privacy != "none"
                && (auth == "none"
                    || self
                        .v3_priv_password
                        .as_ref()
                        .is_none_or(|value| value.len() < 8)))
        {
            return Err(AppError::Validation(
                "SNMPv3 requires a username and valid security-level passwords (minimum 8 bytes)"
                    .into(),
            ));
        }
        Ok(())
    }
}
