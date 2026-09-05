use crate::alert::status::DeviceStatus;

#[derive(Debug, Clone)]
pub struct HealthScorer {
    pub warning_threshold: u32,
    pub critical_threshold: u32,
}

impl HealthScorer {
    pub fn new(warning_threshold: u32, critical_threshold: u32) -> Self {
        Self {
            warning_threshold,
            critical_threshold,
        }
    }

    pub fn score(&self, error_rate: f64, bandwidth: f64, cpu_usage: u32, memory_usage: u32) -> u32 {
        let mut score = 100u32;

        score = score.saturating_sub((error_rate * 1.5) as u32);
        score = score.saturating_sub((bandwidth * 0.3) as u32);
        score = score.saturating_sub(cpu_usage / 3);
        score = score.saturating_sub(memory_usage / 3);

        if score > 100 {
            return 100;
        }

        score
    }

    pub fn status(&self, score: u32) -> DeviceStatus {
        DeviceStatus::from_score(score)
    }
}
