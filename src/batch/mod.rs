use std::sync::{Arc, Mutex};
use crate::bus::{EventBus, BrapiEvent};

pub struct Batch { bus: Arc<EventBus>, buffer: Arc<Mutex<Vec<String>>>, max_size: usize }
impl Batch {
    pub fn new(bus: Arc<EventBus>, max_size: usize) -> Self {
        Self { bus, buffer: Arc::new(Mutex::new(Vec::new())), max_size }
    }
    pub fn push(&self, payload: String) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.push(payload);
            if buf.len() >= self.max_size {
                let combined = buf.join(","); buf.clear();
                self.bus.publish(BrapiEvent::MetricCollected(format!("[{}]", combined)));
            }
        }
    }
}
