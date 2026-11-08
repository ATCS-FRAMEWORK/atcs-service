use crate::bus::{EventBus, BrapiEvent};
use std::sync::Arc;
use std::time::Duration;

pub struct Link { bus: Arc<EventBus> }
impl Link {
    pub fn new(bus: Arc<EventBus>) -> Self { Self { bus } }
    pub fn start_network_monitor(&self) {
        let bus = self.bus.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            loop { interval.tick().await; bus.publish(BrapiEvent::StateChanged("link_ok".to_string())); }
        });
    }
}
