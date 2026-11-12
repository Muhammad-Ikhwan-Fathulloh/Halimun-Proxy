use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

/// Batas keamanan default. Semua bisa diubah di config.yaml.
pub const MAX_BODY_BYTES_CAP: usize = 16 * 1024 * 1024;
pub const MAX_TOKEN_TTL_SECS: i64 = 3600;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Alamat bind untuk endpoint proxy (publik di belakang Nginx).
    #[serde(default = "default_host")]
    pub host: String,
    pub port: u16,
    /// Alamat bind Admin API. Default loopback agar tidak terbuka ke jaringan.
    #[serde(default = "default_admin_host")]
    pub admin_host: String,
    pub admin_port: u16,
    /// Minimal 16 karakter. Bisa juga diisi lewat env HALIMUN_ADMIN_KEY.
    #[serde(default)]
    pub admin_api_key: String,
    #[serde(default = "default_telemetry_host")]
    pub telemetry_host: String,
    pub telemetry_port: u16,
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_admin_host() -> String {
    "127.0.0.1".into()
}
fn default_telemetry_host() -> String {
    "127.0.0.1".into()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: 8080,
            admin_host: default_admin_host(),
            admin_port: 8081,
            admin_api_key: String::new(),
            telemetry_host: default_telemetry_host(),
            telemetry_port: 9090,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EncryptionConfig {
    /// 64 karakter hex (32 byte). Wajib, lihat `halimun-proxy --keygen`.
    #[serde(default)]
    pub aes_key: String,
    /// Minimal 64 karakter hex (32 byte).
    #[serde(default)]
    pub hmac_key: String,
    #[serde(default = "default_xor_key")]
    pub xor_key: u8,
    #[serde(default = "default_alphabet")]
    pub base32_alphabet: String,
    #[serde(default = "default_ttl")]
    pub ttl_seconds: i64,
}

fn default_xor_key() -> u8 {
    172
}
fn default_alphabet() -> String {
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".into()
}
fn default_ttl() -> i64 {
    60
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            aes_key: String::new(),
            hmac_key: String::new(),
            xor_key: default_xor_key(),
            base32_alphabet: default_alphabet(),
            ttl_seconds: default_ttl(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    /// Jumlah request per IP per menit.
    pub rate_limit_per_minute: u32,
    pub nonce_ttl_seconds: u64,
    /// Host yang boleh diakses jika token memakai `bypass_url: true`
    /// (contoh: layanan internal). Dicocokkan persis (case-insensitive).
    pub bypass_whitelist: Vec<String>,
    /// Batas ukuran body request masuk.
    #[serde(default = "default_max_body")]
    pub max_body_bytes: usize,
    /// Batas ukuran body respons dari backend.
    #[serde(default = "default_max_response")]
    pub max_response_bytes: usize,
    /// Timeout per panggilan ke backend.
    #[serde(default = "default_upstream_timeout")]
    pub upstream_timeout_ms: u64,
    /// Metode HTTP yang boleh diteruskan.
    #[serde(default = "default_methods")]
    pub allowed_methods: Vec<String>,
    /// Percayai header X-Real-IP (hanya aktifkan di belakang Nginx yang
    /// menimpa header itu). Jika false, IP diambil dari koneksi TCP.
    #[serde(default)]
    pub trust_proxy_headers: bool,
    /// Izinkan endpoint Admin API untuk membuat kunci acak baru.
    #[serde(default)]
    pub allow_key_generation: bool,
}

fn default_max_body() -> usize {
    64 * 1024
}
fn default_max_response() -> usize {
    10 * 1024 * 1024
}
fn default_upstream_timeout() -> u64 {
    15_000
}
fn default_methods() -> Vec<String> {
    ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit_per_minute: 120,
            nonce_ttl_seconds: 300,
            bypass_whitelist: vec![],
            max_body_bytes: default_max_body(),
            max_response_bytes: default_max_response(),
            upstream_timeout_ms: default_upstream_timeout(),
            allowed_methods: default_methods(),
            trust_proxy_headers: false,
            allow_key_generation: false,
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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub encryption: EncryptionConfig,
    pub security: SecurityConfig,
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
}

/// Memuat YAML lalu menerapkan override dari environment variable
/// (HALIMUN_AES_KEY, HALIMUN_HMAC_KEY, HALIMUN_XOR_KEY, HALIMUN_ADMIN_KEY).
/// Dengan begitu rahasia tidak perlu disimpan di config.yaml.
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let mut cfg: AppConfig = serde_yaml::from_str(&contents)?;
    apply_env_overrides(&mut cfg);
    Ok(cfg)
}

pub fn apply_env_overrides(cfg: &mut AppConfig) {
    if let Ok(v) = std::env::var("HALIMUN_AES_KEY") {
        cfg.encryption.aes_key = v;
    }
    if let Ok(v) = std::env::var("HALIMUN_HMAC_KEY") {
        cfg.encryption.hmac_key = v;
    }
    if let Ok(v) = std::env::var("HALIMUN_XOR_KEY") {
        if let Ok(x) = v.parse::<u8>() {
            cfg.encryption.xor_key = x;
        }
    }
    if let Ok(v) = std::env::var("HALIMUN_ADMIN_KEY") {
        cfg.server.admin_api_key = v;
    }
}

/// Memeriksa konfigurasi secara menyeluruh. Server menolak start jika gagal.
pub fn validate(cfg: &AppConfig) -> Result<(), String> {
    let aes = decode_key(&cfg.encryption.aes_key, 32, "aes_key")?;
    decode_key(&cfg.encryption.hmac_key, 32, "hmac_key")?;
    if cfg
        .encryption
        .aes_key
        .eq_ignore_ascii_case(&cfg.encryption.hmac_key)
    {
        return Err("aes_key dan hmac_key tidak boleh sama".into());
    }
    if aes.iter().all(|&b| b == 0) {
        return Err("aes_key masih nilai contoh (nol), buat kunci baru dengan --keygen".into());
    }
    crate::crypto::base32::Base32::new(&cfg.encryption.base32_alphabet)
        .map_err(|e| e.to_string())?;

    let key = &cfg.server.admin_api_key;
    let weak = [
        "admin-secret-key",
        "change-me-in-production",
        "admin",
        "password",
    ];
    if key.len() < 16 || key.starts_with("change-me") || weak.contains(&key.as_str()) {
        return Err(
            "server.admin_api_key minimal 16 karakter dan bukan nilai contoh (atau set HALIMUN_ADMIN_KEY)"
                .into(),
        );
    }

    for (name, host, port) in [
        ("server", &cfg.server.host, cfg.server.port),
        ("admin", &cfg.server.admin_host, cfg.server.admin_port),
        (
            "telemetry",
            &cfg.server.telemetry_host,
            cfg.server.telemetry_port,
        ),
    ] {
        format!("{host}:{port}")
            .parse::<SocketAddr>()
            .map_err(|_| format!("alamat {name} tidak valid: {host}:{port}"))?;
    }
    if cfg.server.port == cfg.server.admin_port || cfg.server.port == cfg.server.telemetry_port {
        return Err("port proxy, admin, dan telemetry harus berbeda".into());
    }

    let s = &cfg.security;
    if s.rate_limit_per_minute == 0 {
        return Err("security.rate_limit_per_minute harus > 0".into());
    }
    if s.max_body_bytes == 0 || s.max_body_bytes > MAX_BODY_BYTES_CAP {
        return Err("security.max_body_bytes di luar rentang (1..16MB)".into());
    }
    if s.max_response_bytes == 0 {
        return Err("security.max_response_bytes harus > 0".into());
    }
    if !(100..=120_000).contains(&s.upstream_timeout_ms) {
        return Err("security.upstream_timeout_ms harus 100..120000".into());
    }
    if cfg.encryption.ttl_seconds <= 0 || cfg.encryption.ttl_seconds > MAX_TOKEN_TTL_SECS {
        return Err("encryption.ttl_seconds harus 1..3600".into());
    }
    for h in &s.bypass_whitelist {
        if h.is_empty() || h.contains('/') || h.contains(':') || h.contains('@') {
            return Err(format!("bypass_whitelist berisi host tidak valid: {h}"));
        }
    }
    for svc in &cfg.services {
        validate_service(svc)?;
    }
    Ok(())
}

pub fn validate_service(svc: &ServiceConfig) -> Result<(), String> {
    let valid_name = !svc.name.is_empty()
        && svc.name.len() <= 64
        && svc
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid_name {
        return Err(format!("nama service tidak valid: {}", svc.name));
    }
    let url =
        url::Url::parse(&svc.url).map_err(|_| format!("url service {} tidak valid", svc.name))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(format!("url service {} harus http/https", svc.name));
    }
    if !svc.health.is_empty() && !svc.health.starts_with('/') {
        return Err(format!("health service {} harus diawali '/'", svc.name));
    }
    if !(100..=120_000).contains(&svc.timeout_ms) {
        return Err(format!("timeout_ms service {} di luar rentang", svc.name));
    }
    Ok(())
}

/// Mendekode kunci hex dan memastikan panjangnya tepat/minimal `min_len` byte.
pub fn decode_key(hex_str: &str, min_len: usize, name: &str) -> Result<Vec<u8>, String> {
    if hex_str.is_empty() {
        return Err(format!(
            "encryption.{name} kosong. Buat kunci dengan: halimun-proxy --keygen"
        ));
    }
    let bytes = hex::decode(hex_str).map_err(|_| format!("encryption.{name} bukan hex valid"))?;
    if bytes.len() < min_len {
        return Err(format!(
            "encryption.{name} harus minimal {min_len} byte ({} hex)",
            min_len * 2
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> AppConfig {
        let mut cfg = AppConfig::default();
        cfg.encryption.aes_key = "11".repeat(32);
        cfg.encryption.hmac_key = "22".repeat(32);
        cfg.server.admin_api_key = "a-very-long-admin-key".into();
        cfg
    }

    #[test]
    fn accepts_valid_config() {
        assert!(validate(&valid()).is_ok());
    }

    #[test]
    fn rejects_zero_and_default_keys() {
        let mut cfg = valid();
        cfg.encryption.aes_key = "00".repeat(32);
        assert!(validate(&cfg).is_err());
        let mut cfg = valid();
        cfg.encryption.aes_key = String::new();
        assert!(validate(&cfg).is_err());
    }

    #[test]
    fn rejects_weak_admin_key() {
        let mut cfg = valid();
        cfg.server.admin_api_key = "admin-secret-key".into();
        assert!(validate(&cfg).is_err());
    }

    #[test]
    fn rejects_bad_service_url() {
        let svc = ServiceConfig {
            name: "api".into(),
            r#type: ServiceType::Backend,
            url: "file:///etc/passwd".into(),
            health: "/health".into(),
            timeout_ms: 1000,
        };
        assert!(validate_service(&svc).is_err());
    }
}
