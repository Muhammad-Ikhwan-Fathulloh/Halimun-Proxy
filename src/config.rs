use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub admin_port: u16,
    pub admin_api_key: String,
    pub telemetry_port: u16,
    pub redis_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EncryptionConfig {
    pub aes_key: String,
    pub hmac_key: String,
    pub xor_key: u8,
    pub base32_alphabet: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub rate_limit_per_minute: u32,
    pub nonce_ttl_seconds: u64,
    pub bypass_whitelist: Vec<String>,
    pub strict_domain: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Backend,
    Frontend,
    Fullstack,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub name: String,
    pub r#type: ServiceType,
    pub url: String,
    pub health: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub encryption: EncryptionConfig,
    pub security: SecurityConfig,
    pub services: Vec<ServiceConfig>,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let parsed: AppConfig = serde_yaml::from_str(&contents)?;
    Ok(parsed)
}
