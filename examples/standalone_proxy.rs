//! Minimal standalone example demonstrating how to spin up a Halimun Proxy natively
//! without relying on the physical `config.yaml` file.
//!
//! Run this example with: `cargo run --example standalone_proxy`

use halimun_proxy::config::{AppConfig, EncryptionConfig};
use halimun_proxy::proxy::handler::{router, ProxyState};
use halimun_proxy::security::rate_limiter::RateLimiter;
use halimun_proxy::services::logs::Logger;
use halimun_proxy::services::registry::ServiceRegistry;
use halimun_proxy::token::replay_guard::ReplayGuard;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // 1. Manually configure the encryption secrets bypassing environment variables
    let config = AppConfig {
        encryption: EncryptionConfig {
            aes_key: "0000000000000000000000000000000000000000000000000000000000000000".to_string(), // 64 hex characters (32 bytes)
            hmac_key: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            ttl_seconds: 60,
        },
        security: Default::default(),
        server: Default::default(),
        backend: vec![],
        frontend: vec![],
        fullstack: vec![],
    };

    // 2. Setup internal Services State
    let registry = Arc::new(ServiceRegistry::new(Vec::new(), Vec::new(), Vec::new()));
    let replay_guard = ReplayGuard::new(config.encryption.ttl_seconds);
    let rate_limiter = Arc::new(RateLimiter::new());
    let logger = Logger::new(100);
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let state = ProxyState {
        config,
        replay_guard,
        rate_limiter,
        registry,
        http_client,
        logger,
    };

    // 3. Create the highly-optimized Axum framework router
    let app = router(state);

    // 4. Bind and Start!
    println!("Standalone Test Proxy running on http://127.0.0.1:8080");
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
