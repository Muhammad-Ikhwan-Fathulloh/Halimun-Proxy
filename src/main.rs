use clap::Parser;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use std::sync::Arc;

use halimun_proxy::config::{load_config, validate};
use halimun_proxy::proxy::handler::{router as proxy_router, ProxyState};
use halimun_proxy::services::admin::{router as admin_router, AdminOptions};
use halimun_proxy::services::health::start_health_checker;
use halimun_proxy::services::logs::Logger;
use halimun_proxy::services::registry::ServiceRegistry;

#[derive(Parser, Debug)]
#[command(name = "halimun-proxy")]
#[command(about = "Encrypted reverse proxy (AES-256-CBC + HMAC-SHA256)", long_about = None)]
struct Cli {
    /// Cetak kunci kriptografi baru lalu keluar
    #[arg(long)]
    keygen: bool,

    /// Format keygen: env, json, yaml
    #[arg(short, long, default_value = "env")]
    format: String,

    /// Path ke file konfigurasi
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    /// Validasi konfigurasi lalu keluar (tanpa menjalankan server)
    #[arg(long)]
    check_config: bool,
}

fn parse_addr(host: &str, port: u16) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    Ok(format!("{host}:{port}").parse()?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    if cli.keygen {
        halimun_proxy::keygen::generate_keys(&cli.format);
        return Ok(());
    }

    let app_config = load_config(&cli.config)
        .map_err(|e| format!("gagal membaca config {}: {}", cli.config, e))?;

    if let Err(e) = validate(&app_config) {
        eprintln!("Konfigurasi tidak valid: {e}");
        std::process::exit(1);
    }

    if cli.check_config {
        println!("Konfigurasi {} valid.", cli.config);
        return Ok(());
    }

    let proxy_addr = parse_addr(&app_config.server.host, app_config.server.port)?;
    let admin_addr = parse_addr(&app_config.server.admin_host, app_config.server.admin_port)?;
    let telemetry_addr = parse_addr(
        &app_config.server.telemetry_host,
        app_config.server.telemetry_port,
    )?;

    // Telemetry Prometheus (endpoint /metrics terpisah)
    PrometheusBuilder::new()
        .with_http_listener(telemetry_addr)
        .install()
        .map_err(|e| format!("gagal memasang Prometheus: {e}"))?;

    let registry = Arc::new(ServiceRegistry::new(app_config.services.clone()));
    let logger = Logger::new(100);
    start_health_checker(registry.clone());

    let admin_opts = AdminOptions {
        allow_keygen: app_config.security.allow_key_generation,
        xor_key: app_config.encryption.xor_key,
        alphabet: app_config.encryption.base32_alphabet.clone(),
    };
    let admin_key = app_config.server.admin_api_key.clone();
    let admin_app = admin_router(
        registry.clone(),
        admin_key,
        logger.clone(),
        cli.config.clone(),
        admin_opts,
    );

    let proxy_state = ProxyState::new(app_config, registry, logger)?;
    let proxy_app = proxy_router(proxy_state).into_make_service_with_connect_info::<SocketAddr>();

    println!("Halimun Proxy listening on {proxy_addr}");
    println!("Admin API listening on {admin_addr}");
    println!("Telemetry (Prometheus) on {telemetry_addr}");

    let proxy_listener = tokio::net::TcpListener::bind(proxy_addr).await?;
    let admin_listener = tokio::net::TcpListener::bind(admin_addr).await?;

    let proxy_server =
        axum::serve(proxy_listener, proxy_app).with_graceful_shutdown(shutdown_signal());
    let admin_server = axum::serve(
        admin_listener,
        admin_app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal());

    tokio::try_join!(proxy_server, admin_server)?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    println!("Mematikan server...");
}
