use std::io::Write;
use std::net::TcpListener;

pub fn start_dashboard(port: u16) {
    tokio::task::spawn_blocking(move || {
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(listener) = TcpListener::bind(&addr) {
            println!("[BRAPI] Dashboard local em http://{}", addr);
            for mut stream in listener.incoming().flatten() {
                let html = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h1 style='color:green;background:black'>BRAPI OPS ONLINE</h1>";
                let _ = stream.write_all(html.as_bytes());
            }
        }
    });
}
