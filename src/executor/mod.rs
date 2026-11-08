use crate::store::Store;
use crate::control;
use std::sync::Arc;
use std::path::Path;

pub struct Executor {
    store: Arc<Store>,
}

impl Executor {
    pub fn new(store: Arc<Store>) -> Self {
        Self { store }
    }

    fn is_dangerous(cmd: &str) -> bool {
        let dangerous = ["rm -rf /", "mkfs", "dd if=/dev/zero", "chmod 777 /", "> /dev/sda"];
        dangerous.iter().any(|d| cmd.contains(d))
    }

    fn detect_environment(&self) -> String {
        if Path::new("/system/build.prop").exists() {
            "Android/Termux".to_string()
        } else if Path::new("/.dockerenv").exists() {
            "Docker".to_string()
        } else if Path::new("/etc/os-release").exists() {
            "Linux".to_string()
        } else {
            "Unknown".to_string()
        }
    }

    pub async fn process_commands(&self) {
        let mut pending = Vec::new();
        let db = self.store.client.lock().await;
        if let Some(client) = db.as_ref() {
            if let Ok(rows) = client.query("SELECT id, command FROM brapi_commands WHERE status = 'PENDING'", &[]).await {
                for row in rows {
                    pending.push((row.get::<_, i32>(0), row.get::<_, String>(1)));
                }
            }
        }
        drop(db);

        for (id, cmd) in pending {
            if Self::is_dangerous(&cmd) {
                let _ = self.store.write_event("SECURITY", &format!("Interceptado comando destrutivo: {}", cmd)).await;
                let db = self.store.client.lock().await;
                if let Some(client) = db.as_ref() {
                    let _ = client.execute("UPDATE brapi_commands SET status = 'BLOCKED' WHERE id = $1", &[&id]).await;
                }
                continue;
            }

            let env = self.detect_environment();
            let _ = self.store.write_event("CONTEXT", &format!("Ambiente detectado: {}. Executando: {}", env, cmd)).await;

            let output = control::execute(&cmd);
            let _ = self.store.write_event("EXECUTOR", &format!("Executado: {} -> {}", cmd, output.trim())).await;

            let db = self.store.client.lock().await;
            if let Some(client) = db.as_ref() {
                let _ = client.execute("UPDATE brapi_commands SET status = 'DONE', executed_at = NOW() WHERE id = $1", &[&id]).await;
            }
        }
    }
}
