use crate::cleanup::retention::RetentionCleaner;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct CleanupScheduler {
    cleaner: RetentionCleaner,
}

impl CleanupScheduler {
    pub fn new(history_days: u32) -> Self {
        Self {
            cleaner: RetentionCleaner::new(history_days),
        }
    }

    pub fn run(&self) -> Result<(), AppError> {
        self.cleaner.cleanup_old_history()?;
        println!("Cleanup scheduler completed");
        Ok(())
    }
}
