use crate::store::Store;
use crate::recover::Recover;
use std::sync::Arc;

pub struct Restore { store: Arc<Store>, recover: Arc<Recover> }

impl Restore {
    pub fn new(store: Arc<Store>, recover: Arc<Recover>) -> Self { Self { store, recover } }
    pub async fn check_and_restore(&self) {
        if let Some(state) = self.store.read_state("last_crash").await {
            if state == "true" {
                self.store.write_event("RESTORE", "Crash anterior detectado.").await;
                self.recover.handle_critical_error("restore_from_crash").await;
                self.store.write_state("last_crash", "false").await;
            }
        } else { self.store.write_state("last_crash", "true").await; }
    }
    pub async fn mark_clean_shutdown(&self) { self.store.write_state("last_crash", "false").await; }
}
