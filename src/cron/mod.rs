use crate::bus::EventBus;
use std::sync::Arc;
use std::time::Duration;

pub struct Cron;
impl Cron {
    pub fn new() -> Self { Self }
    pub fn start_ticks(&self, bus: Arc<EventBus>) {
        tokio::spawn(async move {
            let mut fast = tokio::time::interval(Duration::from_millis(250));
            let mut slow = tokio::time::interval(Duration::from_secs(300));
            loop {
                tokio::select! {
                    _ = fast.tick() => {},
                    _ = slow.tick() => { bus.publish(crate::bus::BrapiEvent::StateChanged("snapshot".into())); }
                }
            }
        });
    }
}
