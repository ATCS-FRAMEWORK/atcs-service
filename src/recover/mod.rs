use crate::bus::{EventBus, BrapiEvent};
use crate::store::Store;
use std::sync::Arc;

pub struct Recover { pub bus: Arc<EventBus>, pub store: Arc<Store> }

impl Recover {
    pub fn new(bus: Arc<EventBus>, store: Arc<Store>) -> Self { Self { bus, store } }
    pub async fn handle_critical_error(&self, error_msg: &str) {
        self.store.write_event("CRITICAL", error_msg).await;
        self.bus.publish(BrapiEvent::SystemRecovered(error_msg.to_string()));
    }
}
