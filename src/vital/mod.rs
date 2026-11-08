use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub struct Vital { alive: Arc<AtomicBool> }

impl Vital {
    pub fn new() -> Self { Self { alive: Arc::new(AtomicBool::new(true)) } }
    pub fn start_watchdog(&self) {
        let alive = self.alive.clone();
        tokio::spawn(async move {
            let mut last_tick = std::time::Instant::now();
            loop {
                sleep(Duration::from_secs(1)).await;
                if last_tick.elapsed() > Duration::from_secs(10) {
                    if !alive.load(Ordering::Relaxed) { std::process::exit(1); }
                    last_tick = std::time::Instant::now();
                }
            }
        });
    }
    pub fn heartbeat(&self) { self.alive.store(true, Ordering::Relaxed); }
}
