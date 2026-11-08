use std::net::TcpListener;
use std::io::{Read, Write};
use crate::bus::EventBus;
use crate::rules::Rules;
use crate::guard;
use std::sync::Arc;

pub struct Gate { bus: Arc<EventBus>, rules: Rules }
impl Gate {
    pub fn new(bus: Arc<EventBus>, rules: Rules) -> Self { Self { bus, rules } }
    pub fn start(&self) {
        let _bus = self.bus.clone();
        let rules = self.rules.clone();
        tokio::task::spawn_blocking(move || {
            if !rules.enable_security { return; }
            let bind_addr = format!("127.0.0.1:{}", rules.brapi_port);
            if let Ok(listener) = TcpListener::bind(&bind_addr) {
                println!("[BRAPI] Porta segura aberta em {}", bind_addr);
                for stream in listener.incoming() {
                    if let Ok(mut stream) = stream {
                        let mut buffer = [0; 1024];
                        if let Ok(bytes_read) = stream.read(&mut buffer) {
                            if bytes_read > 0 {
                                let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                                let valid = !rules.brapi_token.is_empty() && guard::validate_token(&request, &rules.brapi_token);
                                if valid { let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n{\"status\":\"ok\"}"); }
                                else { let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\n\r\n"); }
                            }
                        }
                    }
                }
            }
        });
    }
}
