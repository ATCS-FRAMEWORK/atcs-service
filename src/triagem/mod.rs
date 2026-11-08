use crate::bus::{EventBus, BrapiEvent};
use crate::store::Store;
use std::sync::Arc;
use std::collections::HashMap;

pub struct Triagem {
    pub bus: Arc<EventBus>,
    pub store: Arc<Store>,
    pub error_counts: HashMap<String, u32>,
}

impl Triagem {
    pub fn new(bus: Arc<EventBus>, store: Arc<Store>) -> Self {
        Self { bus, store, error_counts: HashMap::new() }
    }
    pub async fn process_error(&mut self, error_type: &str, details: &str) {
        let count = self.error_counts.entry(error_type.to_string()).or_insert(0);
        *count += 1;
        self.store.write_event("WARN", &format!("Falha {}: {}", error_type, details)).await;
        if *count > 5 {
            self.bus.publish(BrapiEvent::ErrorOccurred(format!("Limite excedido: {}", error_type)));
        }
    }
    pub fn reset_counters(&mut self) { self.error_counts.clear(); }
}
