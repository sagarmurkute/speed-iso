use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlashingPhase {
    Writing,
    Verifying,
    Complete,
}

impl std::fmt::Display for FlashingPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlashingPhase::Writing => write!(f, "Writing Image"),
            FlashingPhase::Verifying => write!(f, "Verifying Integrity"),
            FlashingPhase::Complete => write!(f, "Complete"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteProgress {
    pub bytes_written: u64,
    pub total_bytes: u64,
    pub speed_mb_per_sec: f64,
    pub eta_seconds: u64,
    pub phase: FlashingPhase,
}

pub type ProgressCallback = Arc<dyn Fn(&WriteProgress) + Send + Sync>;
use std::sync::Arc;
