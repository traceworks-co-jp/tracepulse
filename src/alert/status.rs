#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Healthy,
    Warning,
    Critical,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Offline,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Offline => "offline",
        }
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl DeviceStatus {
    pub fn from_score(score: u32) -> Self {
        if score >= 80 {
            Self::Healthy
        } else if score >= 60 {
            Self::Warning
        } else {
            Self::Critical
        }
    }
}
