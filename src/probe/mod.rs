use std::fs::read_to_string;
use std::sync::{Arc, Mutex};
use std::path::Path;
use std::ffi::CString;
use crate::bus::{EventBus, BrapiEvent};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct HardwareMetrics {
    pub ram_used_percent: f64, pub ram_total_mb: u64, pub ram_free_mb: u64,
    pub cpu_usage_percent: f32, pub cpu_temp_c: f32, pub processes_count: usize,
    pub disk_used_percent: f64, pub uptime_seconds: u64,
}

pub struct Probe {
    bus: Arc<EventBus>,
    last_cpu_total: Arc<Mutex<u64>>,
    last_cpu_idle: Arc<Mutex<u64>>,
    temp_path: Arc<Mutex<Option<String>>>,
    mission_active: bool,
}

impl Probe {
    pub fn new(bus: Arc<EventBus>, mission_active: bool) -> Self {
        Self {
            bus,
            last_cpu_total: Arc::new(Mutex::new(0)),
            last_cpu_idle: Arc::new(Mutex::new(0)),
            temp_path: Arc::new(Mutex::new(None)),
            mission_active,
        }
    }

    fn discover_temperature_path() -> Option<String> {
        if Path::new("/sys/class/thermal/thermal_zone0/temp").exists() { return Some("/sys/class/thermal/thermal_zone0/temp".to_string()); }
        if let Ok(entries) = std::fs::read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(type_content) = read_to_string(path.join("type")) {
                    let t = type_content.trim().to_lowercase();
                    if t.contains("cpu") || t.contains("soc") || t.contains("x86_pkg_temp") {
                        let temp_file = path.join("temp");
                        if temp_file.exists() { return Some(temp_file.to_string_lossy().to_string()); }
                    }
                }
            }
        }
        None
    }

    fn read_disk_usage() -> f64 {
        let path = CString::new("/data").unwrap_or_else(|_| CString::new("/").unwrap());
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(path.as_ptr(), &mut stat) == 0 {
                let total = stat.f_blocks as f64 * stat.f_frsize as f64;
                let free = stat.f_bavail as f64 * stat.f_frsize as f64;
                if total > 0.0 { return ((total - free) / total) * 100.0; }
            }
        }
        0.0
    }

    pub fn start_collection_loop(&self) {
        if !self.mission_active {
            println!("[PROBE] Missao desativada pelo Core.");
            return;
        }

        let bus_ref = self.bus.clone(); let last_total = self.last_cpu_total.clone();
        let last_idle = self.last_cpu_idle.clone(); let temp_path_ref = self.temp_path.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                if temp_path_ref.lock().unwrap().is_none() { *temp_path_ref.lock().unwrap() = Probe::discover_temperature_path(); }
                let mut total_mem = 0u64; let mut avail_mem = 0u64;
                if let Ok(mem) = read_to_string("/proc/meminfo") {
                    for line in mem.lines() {
                        if line.starts_with("MemTotal:") { total_mem = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0); }
                        if line.starts_with("MemAvailable:") { avail_mem = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0); }
                    }
                }
                let total_mem_mb = total_mem / 1024; let avail_mem_mb = avail_mem / 1024;
                let ram_pct = if total_mem_mb > 0 { ((total_mem_mb.saturating_sub(avail_mem_mb)) as f64 / total_mem_mb as f64) * 100.0 } else { 0.0 };
                let mut cpu_pct = 0.0;
                if let Ok(stat) = read_to_string("/proc/stat") {
                    if let Some(line) = stat.lines().find(|l| l.starts_with("cpu ")) {
                        let parts: Vec<u64> = line.split_whitespace().skip(1).filter_map(|v| v.parse().ok()).collect();
                        if parts.len() >= 4 {
                            let idle = parts[3]; let total: u64 = parts.iter().sum();
                            let mut lt = last_total.lock().unwrap(); let mut li = last_idle.lock().unwrap();
                            if *lt > 0 {
                                let diff_total = total - *lt; let diff_idle = idle - *li;
                                if diff_total > 0 { cpu_pct = ((diff_total - diff_idle) as f32 / diff_total as f32) * 100.0; }
                            }
                            *lt = total; *li = idle;
                        }
                    }
                }
                let mut temp = 0.0;
                if let Some(tp) = temp_path_ref.lock().unwrap().as_ref() {
                    if let Ok(t) = read_to_string(tp) { temp = t.trim().parse::<f32>().unwrap_or(0.0) / 1000.0; }
                }
                let mut proc_count = 0;
                if let Ok(dir) = std::fs::read_dir("/proc") {
                    proc_count = dir.filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit())).count();
                }
                let mut uptime = 0u64;
                if let Ok(up) = read_to_string("/proc/uptime") { uptime = up.split_whitespace().next().and_then(|v| v.split('.').next()).and_then(|v| v.parse().ok()).unwrap_or(0); }
                let disk_pct = Probe::read_disk_usage();
                let metrics = HardwareMetrics {
                    ram_used_percent: ram_pct, ram_total_mb: total_mem_mb, ram_free_mb: avail_mem_mb,
                    cpu_usage_percent: cpu_pct, cpu_temp_c: temp, processes_count: proc_count,
                    disk_used_percent: disk_pct, uptime_seconds: uptime,
                };
                let json_payload = serde_json::to_string(&metrics).unwrap_or_default();
                bus_ref.publish(BrapiEvent::MetricCollected(json_payload));
            }
        });
    }
}
