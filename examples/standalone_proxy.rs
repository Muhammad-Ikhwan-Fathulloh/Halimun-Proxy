//! Contoh menjalankan proxy tanpa file config (untuk uji lokal).
//! Kunci diambil dari environment: HALIMUN_AES_KEY, HALIMUN_HMAC_KEY, HALIMUN_ADMIN_KEY.
//!
//! Jalankan: `cargo run --example standalone_proxy`

use std::net::SocketAddr;
use std::sync::Arc;

use halimun_proxy::config::{apply_env_overrides, validate, AppConfig};
use halimun_proxy::proxy::handler::{router, ProxyState};
use halimun_proxy::services::logs::Logger;
use halimun_proxy::services::registry::ServiceRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let mut config = AppConfig::default();
    config.server.port = 8080;
    config.server.admin_port = 8081;
    config.server.telemetry_port = 9090;
    apply_env_overrides(&mut config);
    validate(&config)?;

    let registry = Arc::new(ServiceRegistry::new(Vec::new()));
    let state = ProxyState::new(config, registry, Logger::new(100))?;

    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    println!("Standalone proxy on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
