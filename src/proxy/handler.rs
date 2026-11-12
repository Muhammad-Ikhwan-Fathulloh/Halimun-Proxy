use axum::{
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::{header::CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::post,
    Router,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::{decode_key, validate, AppConfig};
use crate::crypto::codec::Codec;
use crate::security::rate_limiter::RateLimiter;
use crate::security::ssrf::{self, SafeResolver};
use crate::services::logs::{sanitize_url, Logger, RequestLog};
use crate::services::registry::ServiceRegistry;
use crate::token::payload::HalimunToken;
use crate::token::replay_guard::ReplayGuard;
use crate::token::validator::validate_payload;

/// Maksimal jumlah header dari token yang diteruskan ke backend.
const MAX_FORWARDED_HEADERS: usize = 32;

/// Header yang tidak boleh diatur oleh token (hop-by-hop atau dikelola proxy).
const FORBIDDEN_HEADERS: &[&str] = &[
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "keep-alive",
    "te",
    "trailer",
    "upgrade",
    "proxy-authorization",
    "proxy-connection",
    "expect",
    "x-forwarded-for",
    "x-real-ip",
];

/// State proxy. Semua komponen berat dibangun sekali; `Clone` hanya menaikkan
/// jumlah Arc (O(1)), bukan menyalin konfigurasi per request.
#[derive(Clone)]
pub struct ProxyState {
    pub config: Arc<AppConfig>,
    pub codec: Arc<Codec>,
    pub hmac_key: Arc<Vec<u8>>,
    pub replay_guard: ReplayGuard,
    pub rate_limiter: Arc<RateLimiter>,
    pub registry: Arc<ServiceRegistry>,
    /// Klien untuk target publik (resolver menolak alamat privat).
    pub public_client: reqwest::Client,
    /// Klien untuk target whitelist/bypass (hanya host whitelist yang boleh privat).
    pub internal_client: reqwest::Client,
    pub logger: Logger,
    pub allowed_methods: Arc<Vec<Method>>,
}

impl ProxyState {
    /// Membangun state dari konfigurasi. Gagal jika konfigurasi tidak valid.
    /// Membutuhkan runtime Tokio aktif.
    pub fn new(
        config: AppConfig,
        registry: Arc<ServiceRegistry>,
        logger: Logger,
    ) -> Result<Self, String> {
        validate(&config)?;

        let aes = decode_key(&config.encryption.aes_key, 32, "aes_key")?;
        let mut aes_key = [0u8; 32];
        aes_key.copy_from_slice(&aes[..32]);
        let codec = Codec::from_config(&config.encryption, aes_key)?;
        let hmac_key = decode_key(&config.encryption.hmac_key, 32, "hmac_key")?;

        let timeout = Duration::from_millis(config.security.upstream_timeout_ms);
        let public_client = build_client(timeout, SafeResolver::new(Vec::new()))?;
        let internal_client = build_client(
            timeout,
            SafeResolver::new(config.security.bypass_whitelist.clone()),
        )?;

        let allowed_methods = config
            .security
            .allowed_methods
            .iter()
            .filter_map(|m| Method::from_bytes(m.to_ascii_uppercase().as_bytes()).ok())
            .collect();

        Ok(Self {
            rate_limiter: Arc::new(RateLimiter::new(config.security.rate_limit_per_minute)),
            replay_guard: ReplayGuard::new(),
            codec: Arc::new(codec),
            hmac_key: Arc::new(hmac_key),
            public_client,
            internal_client,
            logger,
            registry,
            allowed_methods: Arc::new(allowed_methods),
            config: Arc::new(config),
        })
    }
}

fn build_client(timeout: Duration, resolver: SafeResolver) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(5))
        // Redirect dimatikan: backend tidak bisa mengalihkan proxy ke alamat internal.
        .redirect(reqwest::redirect::Policy::none())
        .dns_resolver(Arc::new(resolver))
        .pool_max_idle_per_host(16)
        .build()
        .map_err(|e| e.to_string())
}

pub fn router(state: ProxyState) -> Router {
    let max_body = state.config.security.max_body_bytes;
    Router::new()
        // Lima segmen URL: salah satunya membawa token terenkripsi
        .route("/proxy/1/:s1/:s2/:s3/:s4/:s5", post(handle_proxy))
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state)
}

fn client_ip(headers: &HeaderMap, peer: Option<IpAddr>, trust_proxy: bool) -> IpAddr {
    if trust_proxy {
        if let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<IpAddr>().ok())
        {
            return ip;
        }
    }
    peer.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({"error": message, "code": status.as_u16()})),
    )
        .into_response()
}

/// Mencatat penolakan ke log (agar muncul di dashboard) lalu membalas error.
fn reject(
    state: &ProxyState,
    ip: IpAddr,
    started: Instant,
    status: StatusCode,
    msg: &str,
) -> Response {
    state.logger.add_log(RequestLog {
        timestamp: chrono::Utc::now(),
        method: "POST".into(),
        path: "PROXY HIDDEN".into(),
        target_url: "-".into(),
        ip: ip.to_string(),
        status: status.as_u16(),
        execution_ms: started.elapsed().as_millis() as u64,
    });
    error_response(status, msg)
}

/// Mengambil field `x` dari body form-urlencoded (dengan percent-decoding).
fn extract_form_field(body: &[u8]) -> Option<String> {
    url::form_urlencoded::parse(body)
        .find(|(k, _)| k == "x")
        .map(|(_, v)| v.into_owned())
}

/// True jika SHA-256 body (hex) dari token sama dengan plaintext body.
fn body_hash_matches(expected_hex: Option<&str>, plain: &[u8]) -> bool {
    let Some(expected) = expected_hex else {
        return false;
    };
    let actual = hex::encode(Sha256::digest(plain));
    actual.eq_ignore_ascii_case(expected)
}

fn is_forbidden_header(name: &HeaderName) -> bool {
    FORBIDDEN_HEADERS.contains(&name.as_str())
}

/// Membaca body respons dengan batas ukuran; berhenti saat melampaui batas.
async fn read_limited(mut res: reqwest::Response, max: usize) -> Result<Vec<u8>, &'static str> {
    if res.content_length().is_some_and(|n| n as usize > max) {
        return Err("Response too large");
    }
    let mut out = Vec::new();
    loop {
        match res.chunk().await {
            Ok(Some(chunk)) => {
                if out.len() + chunk.len() > max {
                    return Err("Response too large");
                }
                out.extend_from_slice(&chunk);
            }
            Ok(None) => return Ok(out),
            Err(_) => return Err("Failed to read backend response"),
        }
    }
}

async fn handle_proxy(
    State(state): State<ProxyState>,
    peer: Option<ConnectInfo<SocketAddr>>,
    Path((s1, s2, s3, s4, s5)): Path<(String, String, String, String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let started = Instant::now();
    let ip = client_ip(
        &headers,
        peer.map(|c| c.0.ip()),
        state.config.security.trust_proxy_headers,
    );

    if !state.rate_limiter.check(ip) {
        return reject(
            &state,
            ip,
            started,
            StatusCode::TOO_MANY_REQUESTS,
            "Too many requests",
        );
    }

    // 1. Cari segmen URL yang berisi token valid (segmen lain adalah dekoy acak)
    let now = chrono::Utc::now().timestamp();
    let mut token: Option<HalimunToken> = None;
    let mut any_decrypted = false;
    let mut last_err: &'static str = "Decryption failed or invalid token";
    for seg in [&s1, &s2, &s3, &s4, &s5] {
        let Some(plain) = state.codec.open(seg) else {
            continue;
        };
        any_decrypted = true;
        match validate_payload(&plain, &state.hmac_key, &state.replay_guard, now) {
            Ok(t) => {
                token = Some(t);
                break;
            }
            Err(e) => last_err = e,
        }
    }

    let token = match token {
        Some(t) => t,
        None if !any_decrypted => {
            return reject(
                &state,
                ip,
                started,
                StatusCode::BAD_REQUEST,
                "Decryption failed or invalid token",
            );
        }
        None if last_err.starts_with("Server busy") => {
            return reject(
                &state,
                ip,
                started,
                StatusCode::SERVICE_UNAVAILABLE,
                last_err,
            );
        }
        None => return reject(&state, ip, started, StatusCode::FORBIDDEN, last_err),
    };

    // 2. Validasi target (SSRF) dan pilih klien yang sesuai
    let bypass = token.bypass_url.unwrap_or(false);
    let (client, allow_list) = if bypass {
        (
            &state.internal_client,
            Some(state.config.security.bypass_whitelist.as_slice()),
        )
    } else {
        (&state.public_client, None)
    };
    let target = match ssrf::check_target(&token.api_url, allow_list) {
        Ok(u) => u,
        Err(e) => return reject(&state, ip, started, StatusCode::FORBIDDEN, e),
    };

    // 3. Metode HTTP harus termasuk daftar yang diizinkan
    let method = match Method::from_bytes(token.method.to_ascii_uppercase().as_bytes()) {
        Ok(m) if state.allowed_methods.contains(&m) => m,
        _ => {
            return reject(
                &state,
                ip,
                started,
                StatusCode::METHOD_NOT_ALLOWED,
                "Method not allowed",
            )
        }
    };

    // 4. Body terenkripsi (opsional, field x=). Body wajib diikat ke token
    //    lewat `body_sha256` (yang ikut tertandatangani HMAC), karena AES-CBC
    //    saja tidak menjamin integritas.
    let body_plain = match extract_form_field(&body) {
        Some(v) if !v.is_empty() => {
            let Some(p) = state.codec.open(&v) else {
                return reject(
                    &state,
                    ip,
                    started,
                    StatusCode::BAD_REQUEST,
                    "Decryption failed for body",
                );
            };
            if !body_hash_matches(token.body_sha256.as_deref(), &p) {
                return reject(
                    &state,
                    ip,
                    started,
                    StatusCode::FORBIDDEN,
                    "Body integrity check failed",
                );
            }
            p
        }
        _ => Vec::new(),
    };

    // 5. Susun header yang diteruskan (hanya header aman)
    let mut req_headers = HeaderMap::new();
    for (k, v) in token.api_header.iter().take(MAX_FORWARDED_HEADERS) {
        let Ok(name) = HeaderName::from_bytes(k.as_bytes()) else {
            continue;
        };
        if is_forbidden_header(&name) {
            continue;
        }
        if let Ok(val) = HeaderValue::from_str(v) {
            req_headers.append(name, val);
        }
    }
    if let Ok(v) = HeaderValue::from_str(&ip.to_string()) {
        req_headers.insert("x-forwarded-for", v);
    }

    // 6. Teruskan ke backend
    let target_str = target.as_str().to_string();
    let upstream = client
        .request(method.clone(), target.clone())
        .headers(req_headers)
        .body(body_plain)
        .send()
        .await;

    let res = match upstream {
        Ok(r) => r,
        Err(e) => {
            eprintln!("upstream gagal ({}): {}", sanitize_url(&target_str), e);
            let (status, msg) = if e.is_timeout() {
                (StatusCode::GATEWAY_TIMEOUT, "Backend timeout")
            } else {
                (StatusCode::BAD_GATEWAY, "Failed to reach backend service")
            };
            state.logger.add_log(RequestLog {
                timestamp: chrono::Utc::now(),
                method: method.to_string(),
                path: "PROXY HIDDEN".into(),
                target_url: sanitize_url(&target_str),
                ip: ip.to_string(),
                status: status.as_u16(),
                execution_ms: started.elapsed().as_millis() as u64,
            });
            metrics::counter!("halimun_http_requests_total", "status" => status.as_u16().to_string()).increment(1);
            return error_response(status, msg);
        }
    };

    let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = res.headers().get(CONTENT_TYPE).cloned();
    let max_resp = state.config.security.max_response_bytes;

    let bytes = match read_limited(res, max_resp).await {
        Ok(b) => b,
        Err(msg) => {
            return reject(&state, ip, started, StatusCode::BAD_GATEWAY, msg);
        }
    };

    metrics::counter!("halimun_http_requests_total", "status" => status.as_u16().to_string())
        .increment(1);
    state.logger.add_log(RequestLog {
        timestamp: chrono::Utc::now(),
        method: method.to_string(),
        path: "PROXY HIDDEN".into(),
        target_url: sanitize_url(&target_str),
        ip: ip.to_string(),
        status: status.as_u16(),
        execution_ms: started.elapsed().as_millis() as u64,
    });

    let mut response = (status, bytes).into_response();
    let h = response.headers_mut();
    if let Some(ct) = content_type {
        h.insert(CONTENT_TYPE, ct);
    }
    h.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    h.insert("cache-control", HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_ip_ignores_header_unless_trusted() {
        let mut h = HeaderMap::new();
        h.insert("x-real-ip", HeaderValue::from_static("203.0.113.9"));
        let peer: IpAddr = "10.0.0.5".parse().unwrap();
        assert_eq!(client_ip(&h, Some(peer), false), peer);
        assert_eq!(
            client_ip(&h, Some(peer), true),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn form_field_is_percent_decoded() {
        assert_eq!(extract_form_field(b"a=1&x=ABC%3D").as_deref(), Some("ABC="));
        assert_eq!(extract_form_field(b"a=1"), None);
    }
}
