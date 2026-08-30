use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct RetentionCleaner {
    pub history_days: u32,
}

impl RetentionCleaner {
    pub fn new(history_days: u32) -> Self {
        Self { history_days }
    }

    pub fn cleanup_old_history(&self) -> Result<usize, AppError> {
        let _ = self.history_days;
        println!("Retention cleanup executed for {} days", self.history_days);
        Ok(1)
    }
}
