use clap::Parser;

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

    // Server bootstrap will be here
    let _app_config = halimun_proxy::config::load_config(&cli.config).unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}", e);
        std::process::exit(1);
    });

    println!("🚀 Halimun Proxy started!");

    Ok(())
}
