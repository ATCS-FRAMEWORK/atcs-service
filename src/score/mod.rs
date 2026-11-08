use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct Score {
    pub total_ticks: Arc<AtomicU64>,
    pub failed_ticks: Arc<AtomicU64>,
}

impl Score {
    pub fn new() -> Self {
        Self { total_ticks: Arc::new(AtomicU64::new(0)), failed_ticks: Arc::new(AtomicU64::new(0)) }
    }
    pub fn register_tick(&self, success: bool) {
        self.total_ticks.fetch_add(1, Ordering::Relaxed);
        if !success { self.failed_ticks.fetch_add(1, Ordering::Relaxed); }
    }
    pub fn get_sla(&self) -> f64 {
        let total = self.total_ticks.load(Ordering::Relaxed);
        if total == 0 { return 100.0; }
        let failed = self.failed_ticks.load(Ordering::Relaxed);
        ((total - failed) as f64 / total as f64) * 100.0
    }
}
