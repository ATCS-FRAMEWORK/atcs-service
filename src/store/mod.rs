use tokio_postgres::{NoTls, Client};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

pub struct Store {
    pub client: Arc<Mutex<Option<Client>>>,
}

impl Store {
    pub async fn new() -> Self {
        let store = Self { client: Arc::new(Mutex::new(None)) };
        std::process::Command::new("sv").arg("up").arg("postgres").spawn().ok();
        store.reconnect().await;
        store
    }

    async fn reconnect(&self) {
        let mut tentativas = 0;
        loop {
            match tokio_postgres::connect("host=localhost user=u0_a234 dbname=postgres", NoTls).await {
                Ok((client, connection)) => {
                    tokio::spawn(async move { let _ = connection.await; });
                    let mut c = self.client.lock().await;
                    *c = Some(client);
                    
                    let db_guard = c.as_ref().unwrap();
                    let _ = db_guard.execute("CREATE TABLE IF NOT EXISTS brapi_events (id SERIAL PRIMARY KEY, timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(), level VARCHAR(20) NOT NULL, message TEXT NOT NULL)", &[]).await;
                    let _ = db_guard.execute("CREATE TABLE IF NOT EXISTS brapi_state (key VARCHAR(50) PRIMARY KEY, value TEXT NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())", &[]).await;
                    let _ = db_guard.execute("CREATE TABLE IF NOT EXISTS brapi_history (id SERIAL PRIMARY KEY, timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(), ram_used_percent FLOAT NOT NULL, cpu_usage_percent FLOAT NOT NULL, disk_used_percent FLOAT NOT NULL, sla_percent FLOAT NOT NULL)", &[]).await;
                    let _ = db_guard.execute("CREATE TABLE IF NOT EXISTS brapi_lessons (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), context VARCHAR(50) NOT NULL, avg_ram FLOAT, avg_cpu FLOAT, avg_disk FLOAT, evento TEXT)", &[]).await;
                    break;
                },
                Err(_) => {
                    tentativas += 1;
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    if tentativas > 30 { std::process::exit(1); }
                }
            }
        }
    }

    pub async fn write_event(&self, level: &str, message: &str) {
        let c = self.client.lock().await;
        if let Some(db) = c.as_ref() {
            if db.execute("INSERT INTO brapi_events (level, message) VALUES ($1, $2)", &[&level, &message]).await.is_err() {
                drop(c);
                self.reconnect().await;
            }
        }
    }

    pub async fn write_state(&self, key: &str, value: &str) -> bool {
        let c = self.client.lock().await;
        if let Some(db) = c.as_ref() {
            let ok = db.execute("INSERT INTO brapi_state (key, value, updated_at) VALUES ($1, $2, NOW()) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()", &[&key, &value]).await.is_ok();
            if !ok { drop(c); self.reconnect().await; }
            return ok;
        }
        false
    }

    pub async fn read_state(&self, key: &str) -> Option<String> {
        let c = self.client.lock().await;
        if let Some(db) = c.as_ref() {
            if let Ok(rows) = db.query("SELECT value FROM brapi_state WHERE key = $1", &[&key]).await {
                return rows.first().map(|r| r.get(0));
            } else {
                drop(c);
                self.reconnect().await;
            }
        }
        None
    }

    pub async fn checkpoint(&self) -> bool {
        self.write_state("checkpoint", "alive").await
    }

    pub async fn salvar_historico(&self, ram: f64, cpu: f64, disco: f64, sla: f64) {
        let c = self.client.lock().await;
        if let Some(db) = c.as_ref() {
            if db.execute("INSERT INTO brapi_history (ram_used_percent, cpu_usage_percent, disk_used_percent, sla_percent) VALUES ($1, $2, $3, $4)", &[&ram, &cpu, &disco, &sla]).await.is_err() {
                drop(c);
                self.reconnect().await;
                return;
            }
            
            let count_row = db.query("SELECT COUNT(*) FROM brapi_history", &[]).await.unwrap();
            let count: i64 = count_row[0].get(0);

            if count > 500 {
                let _ = db.execute("INSERT INTO brapi_lessons (context, avg_ram, avg_cpu, avg_disk, evento) SELECT 'Rotina', AVG(ram_used_percent), AVG(cpu_usage_percent), AVG(disk_used_percent), 'Estado medio consolidado' FROM brapi_history", &[]).await;
                let _ = db.execute("DELETE FROM brapi_history", &[]).await;
            }
        }
    }

    pub async fn buscar_padrao_correlacao(&self) -> Option<String> {
        let c = self.client.lock().await;
        if let Some(db) = c.as_ref() {
            if let Ok(rows) = db.query("SELECT context, avg_ram, avg_cpu, evento FROM brapi_lessons ORDER BY created_at DESC LIMIT 5", &[]).await {
                if let Some(row) = rows.first() {
                    let ctx: String = row.get(0);
                    let ram: f64 = row.get(1);
                    let cpu: f64 = row.get(2);
                    let evt: String = row.get(3);
                    return Some(format!("Licao antiga ({}): {} - RAM media {:.1}%, CPU {:.1}%", ctx, evt, ram, cpu));
                }
            } else {
                drop(c);
                self.reconnect().await;
            }
        }
        None
    }
}
