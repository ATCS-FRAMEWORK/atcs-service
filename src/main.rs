use std::fs::read_to_string;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct ATCSSystem {
    endpoint: String,
    headers: String,
    ciclo: u32,
}

impl ATCSSystem {
    fn new() -> Self {
        let endpoint = std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "https://otlp-gateway-prod-sa-east-1.grafana.net/otlp".to_string());
        let headers = std::env::var("OTLP_HEADERS").unwrap_or_default();
        Self { endpoint, headers, ciclo: 0 }
    }

    fn ts_ns(&self) -> u128 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    }

    fn enviar(&self, path: &str, payload: &str) {
        let url = format!("{}/{}", self.endpoint.trim_end_matches('/'), path);

        #[cfg(feature = "termux")]
        {
            let _ = std::process::Command::new("curl")
                .arg("-s").arg("-o").arg("/dev/null")
                .arg("-X").arg("POST").arg(&url)
                .arg("-H").arg("Content-Type: application/json")
                .arg("-H").arg(&self.headers)
                .arg("-d").arg(payload)
                .output();
        }

        #[cfg(feature = "server")]
        {
            let client = reqwest::blocking::Client::new();
            let _ = client.post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", self.headers.trim_start_matches("Authorization: "))
                .body(payload.to_string())
                .send();
        }
    }

    fn metricas(&self, ram: f64, disco: f64, proc: u32) {
        let ts = self.ts_ns();
        let payload = format!(
            r#"{{"resourceMetrics":[{{"resource":{{"attributes":[{{"key":"service.name","value":{{"stringValue":"atcs-enterprise"}}}}]}},"scopeMetrics":[{{"metrics":[{{"name":"atcs_ram_percent","gauge":{{"dataPoints":[{{"asDouble":{:.2},"timeUnixNano":"{}"}}]}}}},{{"name":"atcs_disco_percent","gauge":{{"dataPoints":[{{"asDouble":{:.2},"timeUnixNano":"{}"}}]}}}},{{"name":"atcs_processos","gauge":{{"dataPoints":[{{"asInt":"{}","timeUnixNano":"{}"}}]}}}}]}}]}}]}}"#,
            ram, ts, disco, ts, proc, ts
        );
        self.enviar("v1/metrics", &payload);
    }

    fn trace_hierarquico(&self, trace_id: &str, span_id: &str, parent_span_id: Option<&str>, nome: &str, modulo: &str, inicio: u128, fim: u128, erro: bool) {
        let parent = match parent_span_id {
            Some(p) => format!(r#""parentSpanId":"{}","#, p),
            None => "".to_string(),
        };
        let status_code = if erro { 2 } else { 1 };
        let payload = format!(
            r#"{{"resourceSpans":[{{"resource":{{"attributes":[{{"key":"service.name","value":{{"stringValue":"atcs-enterprise"}}}}]}},"scopeSpans":[{{"spans":[{{"traceId":"{}","spanId":"{}",{}"name":"{}","kind":1,"startTimeUnixNano":"{}","endTimeUnixNano":"{}","attributes":[{{"key":"modulo","value":{{"stringValue":"{}"}}}}],"status":{{"code":{}}}}}]}}]}}]}}"#,
            trace_id, span_id, parent, nome, inicio, fim, modulo, status_code
        );
        self.enviar("v1/traces", &payload);
    }

    fn coletar_linux(&self) -> (f64, f64, u32) {
        let mem = read_to_string("/proc/meminfo").unwrap_or_default();
        let mut total = 0u64;
        let mut livre = 0u64;
        for linha in mem.lines() {
            let p: Vec<&str> = linha.split_whitespace().collect();
            if p.len() >= 2 {
                let v: u64 = p[1].parse().unwrap_or(0);
                if p[0] == "MemTotal:" { total = v; }
                if p[0] == "MemFree:" { livre = v; }
            }
        }
        let ram = if total > 0 { ((total - livre) as f64 / total as f64) * 100.0 } else { 0.0 };

        let disco = std::process::Command::new("df").arg("/").output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
        let mut disco_pct = 0.0;
        for linha in disco.lines().skip(1) {
            let p: Vec<&str> = linha.split_whitespace().collect();
            if p.len() >= 5 { disco_pct = p[4].trim_end_matches('%').parse().unwrap_or(0.0); }
        }

        let proc = std::process::Command::new("ps").arg("-e").output().map(|o| String::from_utf8_lossy(&o.stdout).lines().count() as u32).unwrap_or(0);
        (ram, disco_pct, proc)
    }

    fn coletar(&self) -> (f64, f64, u32) {
        #[cfg(target_os = "linux")]
        { self.coletar_linux() }
        #[cfg(not(target_os = "linux"))]
        { (0.0, 0.0, 0) }
    }

    fn executar(&mut self) {
        println!("ATCS ENTERPRISE - Iniciando no sistema: {}", std::env::consts::OS);
        loop {
            self.ciclo += 1;
            let trace_id = format!("{:032x}", self.ts_ns());
            let span_principal = format!("{:016x}", self.ts_ns());
            let inicio_ciclo = self.ts_ns();

            let (ram, disco, proc) = self.coletar();
            let erro = ram > 95.0;

            let span_sentinel = format!("{:016x}", self.ts_ns());
            self.trace_hierarquico(&trace_id, &span_sentinel, Some(&span_principal), "sentinel_scan", "SENTINEL", inicio_ciclo, self.ts_ns(), erro);

            let span_phl = format!("{:016x}", self.ts_ns());
            self.trace_hierarquico(&trace_id, &span_phl, Some(&span_principal), "phl_coleta", "PHL", inicio_ciclo, self.ts_ns(), erro);

            self.metricas(ram, disco, proc);

            let span_monitor = format!("{:016x}", self.ts_ns());
            self.trace_hierarquico(&trace_id, &span_monitor, Some(&span_principal), "monitor_envio", "MONITOR", inicio_ciclo, self.ts_ns(), erro);

            self.trace_hierarquico(&trace_id, &span_principal, None, "ciclo_atcs", "CORE", inicio_ciclo, self.ts_ns(), erro);

            println!("[CICLO {}] RAM={:.1}% | Traces Hierárquicos Enviados (Erro: {})", self.ciclo, ram, erro);
            std::thread::sleep(Duration::from_secs(5));
        }
    }
}

fn main() {
    let mut sistema = ATCSSystem::new();
    sistema.executar();
}
