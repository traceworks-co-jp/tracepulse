#[derive(Debug, Clone)]
pub struct ApiSummary {
    pub healthy: usize,
    pub warning: usize,
    pub critical: usize,
    pub offline: usize,
}

impl ApiSummary {
    pub fn empty() -> Self {
        Self {
            healthy: 0,
            warning: 0,
            critical: 0,
            offline: 0,
        }
    }
}
