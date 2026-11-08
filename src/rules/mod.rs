use serde::Deserialize;
use std::fs::read_to_string;

#[derive(Deserialize, Clone, Debug)]
pub struct Rules {
    pub otlp_endpoint: String,
    pub otlp_headers: String,
    pub enable_grafana: bool,
    pub enable_security: bool,
    pub brapi_port: u16,
    pub brapi_token: String,
    pub log_level: String,
    pub mission_mode: String,
}

impl Rules {
    pub fn load() -> Self {
        let path = "/data/data/com.termux/files/home/NAYH_SYSTEM/config.env";
        let content = read_to_string(path).unwrap_or_default();
        let mut config = Rules {
            otlp_endpoint: String::new(), otlp_headers: String::new(),
            enable_grafana: false, enable_security: true,
            brapi_port: 49152, brapi_token: String::new(), log_level: "info".to_string(),
            mission_mode: "FULL".to_string(),
        };
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() { continue; }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_uppercase();
                let value = value.trim().trim_matches('"').to_string();
                match key.as_str() {
                    "OTLP_ENDPOINT" => config.otlp_endpoint = value,
                    "OTLP_HEADERS" => config.otlp_headers = value,
                    "ENABLE_GRAFANA" => config.enable_grafana = value.parse().unwrap_or(false),
                    "ENABLE_SECURITY" => config.enable_security = value.parse().unwrap_or(true),
                    "BRAPI_PORT" => config.brapi_port = value.parse().unwrap_or(49152),
                    "BRAPI_TOKEN" => config.brapi_token = value,
                    "LOG_LEVEL" => config.log_level = value,
                    "MISSION_MODE" => config.mission_mode = value,
                    _ => {}
                }
            }
        }
        config
    }
}
