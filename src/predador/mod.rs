use crate::store::Store;
use crate::control;
use std::sync::Arc;
use std::time::Duration;
use std::path::Path;

pub struct Predador {
    store: Arc<Store>,
}

impl Predador {
    pub fn new(store: Arc<Store>) -> Self {
        Self { store }
    }

    fn is_android(&self) -> bool {
        Path::new("/system/build.prop").exists()
    }

    fn is_idle(&self) -> bool {
        if !self.is_android() { return false; }
        if let Ok(status) = std::fs::read_to_string("/sys/class/power_supply/battery/status") {
            if status.trim() == "Charging" { return true; }
        }
        if let Ok(brightness) = std::fs::read_to_string("/sys/class/leds/lcd-backlight/brightness") {
            if brightness.trim() == "0" { return true; }
        }
        false
    }

    pub async fn start(&self) {
        let store = self.store.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                
                let is_android = Path::new("/system/build.prop").exists();
                let is_idle = {
                    if let Ok(status) = std::fs::read_to_string("/sys/class/power_supply/battery/status") {
                        status.trim() == "Charging"
                    } else { false }
                };

                if is_android && is_idle {
                    let _ = store.write_event("PREDADOR", "Ambiente ocioso. Iniciando varredura noturna.").await;
                    
                    if let Ok(output) = std::process::Command::new("sh").arg("-c").arg("ps -eo pid,comm,pcpu --sort=-pcpu | head -n 5").output() {
                        let text = String::from_utf8_lossy(&output.stdout).to_string();
                        for line in text.lines().skip(1) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 3 {
                                let pid = parts[0];
                                let comm = parts[1];
                                let cpu: f32 = parts[2].parse().unwrap_or(0.0);
                                if cpu > 20.0 && !comm.contains("brapi") && !comm.contains("postgres") {
                                    let _ = store.write_event("PREDADOR", &format!("Ameaca detectada: {} (PID {}). CPU: {:.1}%. Abatendo.", comm, pid, cpu)).await;
                                    let _ = control::execute(&format!("kill -9 {}", pid));
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
