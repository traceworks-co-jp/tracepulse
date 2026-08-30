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
