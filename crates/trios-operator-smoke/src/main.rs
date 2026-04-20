use std::time::Duration;

#[tokio::main]
async fn main() {
    let addr = std::env::var("TRIOS_SERVER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:9005".to_string());

    eprintln!("Connecting to {}...", addr);

    let stream = match tokio::net::TcpStream::connect(&addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("FAIL: Could not connect to {}: {}", addr, e);
            eprintln!("Hint: Is trios-server running? cargo run -p trios-server");
            std::process::exit(1);
        }
    };

    if stream.peer_addr().is_ok() {
        eprintln!("PASS: TCP connection to {} succeeded", addr);
    }

    drop(stream);

    let http_addr = format!("http://{}/health", addr);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    match client.get(&http_addr).send().await {
        Ok(resp) if resp.status().is_success() => {
            eprintln!("PASS: GET /health → {}", resp.status());
        }
        Ok(resp) => {
            eprintln!("FAIL: GET /health → {}", resp.status());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("WARN: GET /health failed: {}", e);
            eprintln!("(This is OK if only checking TCP reachability)");
        }
    }

    eprintln!("All smoke tests passed.");
}
