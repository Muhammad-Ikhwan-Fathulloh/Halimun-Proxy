use clap::Parser;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use std::sync::Arc;

use halimun_proxy::proxy::handler::{router as proxy_router, ProxyState};
use halimun_proxy::security::rate_limiter::RateLimiter;
use halimun_proxy::services::admin::router as admin_router;
use halimun_proxy::services::health::start_health_checker;
use halimun_proxy::services::logs::Logger;
use halimun_proxy::services::registry::ServiceRegistry;
use halimun_proxy::token::replay_guard::ReplayGuard;

#[derive(Parser, Debug)]
#[command(name = "halimun-proxy")]
#[command(about = "High-performance encrypted Rust proxy", long_about = None)]
struct Cli {
    /// Generate cross-language sync keys instead of starting the server
    #[arg(long)]
    keygen: bool,

    /// Format for keygen (env, json, yaml)
    #[arg(short, long, default_value = "env")]
    format: String,

    /// Path to config file
    #[arg(short, long, default_value = "config.yaml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    if cli.keygen {
        halimun_proxy::keygen::generate_keys(&cli.format);
        return Ok(());
    }

    // 1. Load Configuration
    let app_config = halimun_proxy::config::load_config(&cli.config).unwrap_or_else(|e| {
        eprintln!("Failed to load config ({}): {}", cli.config, e);
        std::process::exit(1);
    });

    // 2. Initialize Shared State Components
    let replay_guard = ReplayGuard::new();
    let rate_limiter = Arc::new(RateLimiter::new(app_config.security.rate_limit_per_minute));
    let registry = Arc::new(ServiceRegistry::new(app_config.services.clone()));
    let logger = Logger::new(100); // Store last 100 requests in memory
    let http_client = reqwest::Client::new();

    // 3. Setup Prometheus Telemetry
    let builder = PrometheusBuilder::new();
    builder
        .with_http_listener(SocketAddr::from((
            [0, 0, 0, 0],
            app_config.server.telemetry_port,
        )))
        .install()
        .expect("failed to install Prometheus recorder");

    // 4. Start Background Services
    start_health_checker(registry.clone()).await;

    // 5. Initialize States & Routers
    let proxy_state = ProxyState {
        config: app_config.clone(),
        replay_guard,
        rate_limiter,
        registry: registry.clone(),
        http_client,
        logger: logger.clone(),
    };

    let main_router = proxy_router(proxy_state);
    let admin_api_router = admin_router(
        registry,
        app_config.server.admin_api_key.clone(),
        logger,
        cli.config.clone(),
    );

    // 6. Bind and Serve
    let proxy_addr = SocketAddr::from(([0, 0, 0, 0], app_config.server.port));
    let admin_addr = SocketAddr::from(([0, 0, 0, 0], app_config.server.admin_port));

    println!("🚀 Halimun Proxy started on {}", proxy_addr);
    println!("🛠️ Admin API started on {}", admin_addr);
    println!(
        "📊 Telemetry (Prometheus) on port {}",
        app_config.server.telemetry_port
    );

    let proxy_handle = axum::serve(
        tokio::net::TcpListener::bind(proxy_addr).await?,
        main_router.into_make_service(),
    );

    let admin_handle = axum::serve(
        tokio::net::TcpListener::bind(admin_addr).await?,
        admin_api_router.into_make_service(),
    );

    // Wait for both servers
    let _ = tokio::try_join!(proxy_handle, admin_handle)?;

    Ok(())
}
