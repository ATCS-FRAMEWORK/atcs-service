use tokio::sync::broadcast;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub enum BrapiEvent {
    MetricCollected(String),
    ErrorOccurred(String),
    SecurityAlert(String),
    StateChanged(String),
    SystemRecovered(String),
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<BrapiEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self { tx }
    }
    pub fn publish(&self, event: BrapiEvent) { let _ = self.tx.send(event); }
    pub fn subscribe(&self) -> broadcast::Receiver<BrapiEvent> { self.tx.subscribe() }
}
