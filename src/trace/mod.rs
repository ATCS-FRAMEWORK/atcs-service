use crate::bus::{EventBus, BrapiEvent};
use crate::rules::Rules;
use std::sync::Arc;
use serde_json::json;
use tokio::sync::Mutex;

pub struct Tracer {
    bus: Arc<EventBus>,
    rules: Rules,
    client: ureq::Agent,
    buffer: Arc<Mutex<Vec<String>>>,
}

impl Tracer {
    pub fn new(bus: Arc<EventBus>, rules: Rules) -> Self {
        let client = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(3))
            .build();
        Self { 
            bus, 
            rules, 
            client, 
            buffer: Arc::new(Mutex::new(Vec::new())) 
        }
    }

    pub fn start_listener(&self) {
        let mut rx = self.bus.subscribe();
        let endpoint = self.rules.otlp_endpoint.clone();
        let headers = self.rules.otlp_headers.clone();
        let enabled = self.rules.enable_grafana;
        let client = self.client.clone();
        let buffer = self.buffer.clone();

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if let BrapiEvent::MetricCollected(payload) = event {
                    if !enabled || endpoint.is_empty() { continue; }

                    {
                        let mut b = buffer.lock().await;
                        b.push(payload);
                        if b.len() > 100 { b.remove(0); }
                    }

                    let mut tentativas = 0;
                    while tentativas < 3 {
                        let metrics_to_send = {
                            let mut b = buffer.lock().await;
                            if b.is_empty() { None } else { Some(b.remove(0)) }
                        };

                        let payload_str = match metrics_to_send {
                            Some(p) => p,
                            None => break,
                        };

                        let metrics: serde_json::Value = serde_json::from_str(&payload_str).unwrap_or(json!({}));
                        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().to_string();
                        let mut metric_array = Vec::new();

                        let keys_double = [
                            ("brapi_ram_used_percent", "ram_used_percent"), 
                            ("brapi_cpu_usage_percent", "cpu_usage_percent"),
                            ("brapi_cpu_temp_celsius", "cpu_temp_c"), 
                            ("brapi_disk_used_percent", "disk_used_percent"),
                            ("brapi_sla_percent", "brapi_sla_percent"),
                        ];
                        for (metric_name, field) in keys_double {
                            if let Some(val) = metrics.get(field).and_then(|v| v.as_f64()) {
                                metric_array.push(json!({"name": metric_name, "gauge": {"dataPoints": [{"asDouble": val, "timeUnixNano": ts}]}}));
                            }
                        }

                        let keys_int = [
                            ("brapi_ram_total_mb", "ram_total_mb"), 
                            ("brapi_ram_free_mb", "ram_free_mb"),
                            ("brapi_processes_count", "processes_count"), 
                            ("brapi_uptime_seconds", "uptime_seconds"),
                            ("brapi_ticks_total", "brapi_ticks_total"), 
                            ("brapi_checkpoint_status", "brapi_checkpoint_status"),
                            ("brapi_last_crash", "brapi_last_crash"),
                            ("brapi_triagem_errors", "brapi_triagem_errors"),
                        ];
                        for (metric_name, field) in keys_int {
                            if let Some(val) = metrics.get(field).and_then(|v| v.as_u64()) {
                                metric_array.push(json!({"name": metric_name, "gauge": {"dataPoints": [{"asInt": val.to_string(), "timeUnixNano": ts}]}}));
                            }
                        }

                        let otlp_payload = json!({
                            "resourceMetrics": [{
                                "resource": {"attributes": [{"key": "service.name", "value": {"stringValue": "brapi"}}]},
                                "scopeMetrics": [{"metrics": metric_array}]
                            }]
                        }).to_string();

                        let url = format!("{}/v1/metrics", endpoint.trim_end_matches('/'));
                        
                        let client_clone = client.clone();
                        let headers_clone = headers.clone();
                        let url_clone = url.clone();
                        let payload_clone = otlp_payload.clone();

                        let success = tokio::task::spawn_blocking(move || {
                            let mut req = client_clone.post(&url_clone).set("Content-Type", "application/json");
                            if !headers_clone.is_empty() {
                                if let Some((k, v)) = headers_clone.split_once(':') { req = req.set(k.trim(), v.trim()); }
                            }
                            req.send_string(&payload_clone).is_ok()
                        }).await.unwrap_or(false);

                        if success {
                            break;
                        } else {
                            tentativas += 1;
                            let mut b = buffer.lock().await;
                            b.insert(0, payload_str);
                            drop(b);
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            }
        });
    }
}
