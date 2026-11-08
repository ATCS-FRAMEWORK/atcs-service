use crate::store::Store;
use crate::control;
use std::sync::Arc;
use std::time::Duration;

pub struct Zelador {
    store: Arc<Store>,
}

impl Zelador {
    pub fn new(store: Arc<Store>) -> Self {
        Self { store }
    }

    pub async fn start(&self) {
        let store = self.store.clone();
        tokio::spawn(async move {
            let mut tick = 0;
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                tick += 1;

                let db = store.client.lock().await;
                let mut ram_high = false;
                let mut temp_high = false;

                if let Some(client) = db.as_ref() {
                    if let Ok(rows) = client.query("SELECT message FROM brapi_events WHERE level = 'CONTEXT' ORDER BY timestamp DESC LIMIT 1", &[]).await {
                        if let Some(row) = rows.first() {
                            let msg: String = row.get(0);
                            if msg.contains("RAM alta") { ram_high = true; }
                            if msg.contains("Temp alta") { temp_high = true; }
                        }
                    }
                }
                drop(db);

                if ram_high {
                    let _ = store.write_event("ZELADOR", "Intervencao: RAM alta detectada. Limpando cache.").await;
                    let _ = control::execute("sync; echo 3 > /proc/sys/vm/drop_caches 2>/dev/null");
                }

                if temp_high {
                    let _ = store.write_event("ZELADOR", "Intervencao: Temperatura alta. Reduzindo carga.").await;
                    let _ = control::execute("killall -STOP top 2>/dev/null");
                }

                if tick % 1800 == 0 {
                    let db = store.client.lock().await;
                    if let Some(client) = db.as_ref() {
                        let _ = client.execute("DELETE FROM brapi_events WHERE timestamp < NOW() - INTERVAL '7 days'", &[]).await;
                        let _ = client.execute("VACUUM ANALYZE brapi_events;", &[]).await;
                    }
                    drop(db);
                    let _ = store.write_event("ZELADOR", "Limpeza de manutencao concluida.").await;
                }
            }
        });
    }
}
