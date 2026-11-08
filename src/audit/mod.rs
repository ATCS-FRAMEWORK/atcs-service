use std::fs::{self, OpenOptions};
use std::io::Write;

pub struct Audit { log_path: String }

impl Audit {
    pub fn new() -> Self {
        let base = "/data/data/com.termux/files/home/NAYH_SYSTEM/brapi_state";
        let _ = fs::create_dir_all(base);
        Self { log_path: format!("{}/audit.log", base) }
    }
    pub fn record(&self, action: &str, details: &str) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&self.log_path) {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] {} | {}", ts, action, details);
        }
    }
}
