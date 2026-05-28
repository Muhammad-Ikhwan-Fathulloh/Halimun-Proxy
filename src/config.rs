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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            admin_port: 8081,
            admin_api_key: "admin-secret-key".to_string(),
            telemetry_port: 9090,
            redis_url: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EncryptionConfig {
    pub aes_key: String,
    pub hmac_key: String,
    pub xor_key: u8,
    pub base32_alphabet: String,
    pub ttl_seconds: i64,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            aes_key: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            hmac_key: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            xor_key: 42,
            base32_alphabet: "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".to_string(),
            ttl_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub rate_limit_per_minute: u32,
    pub nonce_ttl_seconds: u64,
    pub bypass_whitelist: Vec<String>,
    pub strict_domain: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit_per_minute: 60,
            nonce_ttl_seconds: 60,
            bypass_whitelist: vec![],
            strict_domain: "localhost".to_string(),
        }
    }
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

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            encryption: EncryptionConfig::default(),
            security: SecurityConfig::default(),
            services: vec![],
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let parsed: AppConfig = serde_yaml::from_str(&contents)?;
    Ok(parsed)
}
