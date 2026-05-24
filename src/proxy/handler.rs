use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Request},
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use std::sync::Arc;
use serde_json::json;

use crate::crypto::{aes_cbc, base32, obfuscation};
use crate::token::payload::HalimunToken;
use crate::token::validator;
use crate::token::replay_guard::ReplayGuard;
use crate::security::{ssrf, rate_limiter::RateLimiter};
use crate::services::registry::ServiceRegistry;
use crate::config::AppConfig;

use crate::services::logs::Logger;

#[derive(Clone)]
pub struct ProxyState {
    pub config: AppConfig,
    pub replay_guard: ReplayGuard,
    pub rate_limiter: Arc<RateLimiter>,
    pub registry: Arc<ServiceRegistry>,
    pub http_client: reqwest::Client,
    pub logger: Logger,
}

pub fn router(state: ProxyState) -> Router {
    Router::new()
        // Support up to 5 dummy URL segments for obfuscation
        .route("/proxy/1/:s1/:s2/:s3/:s4/:s5", post(handle_proxy))
        // And an endpoint for keygen sync from frontend
        .with_state(state) // Fallback routes omitted for clarity
}

async fn handle_proxy(
    State(state): State<ProxyState>,
    Path((s1, s2, s3, s4, s5)): Path<(String, String, String, String, String)>,
    headers: HeaderMap,
    body_string: String,
) -> impl IntoResponse {
    
    // 0. Rate limiting (Simplified using arbitrary IP for demo)
    let ip = headers.get("x-real-ip").and_then(|v| v.to_str().ok()).unwrap_or("127.0.0.1");
    if !state.rate_limiter.check(ip) {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many requests").into_response();
    }

    let segments = vec![s1, s2, s3, s4, s5];
    let alphabet = &state.config.encryption.base32_alphabet;
    let xor_key = state.config.encryption.xor_key;
    let aes_key = hex::decode(&state.config.encryption.aes_key).unwrap_or_default();
    let hmac_key = hex::decode(&state.config.encryption.hmac_key).unwrap_or_default();

    let mut decrypted_token: Option<HalimunToken> = None;

    // 1. Loop mencari segmen URL mana yang berisi Token Halimun asli
    for seg in segments {
        if let Ok(token_bytes) = base32::decode(&seg, alphabet) {
            let deobfuscated = obfuscation::custom_deobfuscate(&token_bytes, xor_key);
            if let Some(decrypted_json) = aes_cbc::decrypt(&deobfuscated, &aes_key) {
                if let Ok(parsed_token) = serde_json::from_slice::<HalimunToken>(&decrypted_json) {
                    decrypted_token = Some(parsed_token);
                    break;
                }
            }
        }
    }

    let token = match decrypted_token {
        Some(t) => t,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Decryption failed or invalid token", "code": 400}))).into_response(),
    };

    // 2. Initial Validation (HMAC, Expiry, Nonce)
    let token = match validator::validate_token(token, &hmac_key, &state.replay_guard) {
        Ok(t) => t,
        Err(e) => return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": e, "code": 403}))).into_response(),
    };

    // 3. SSRF validation
    let bypass = token.bypass_url.unwrap_or(false);
    if let Err(e) = ssrf::validate_proxy_url(&token.api_url, bypass, &state.config.security.bypass_whitelist, &state.config.security.strict_domain) {
         return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": e, "code": 403}))).into_response();
    }

    // 4. Ekstrak & Dekripsi Body Data (x=ENCRYPTED_BODY_BASE32)
    // Optional encrypted body in form x=
    let mut encrypted_body_b32 = String::new();
    for pair in body_string.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some("x"), Some(val)) = (parts.next(), parts.next()) {
            // URL decode val could be needed
            encrypted_body_b32 = val.to_string();
            break;
        }
    }

    let decrypted_body = if !encrypted_body_b32.is_empty() {
        if let Ok(body_bytes) = base32::decode(&encrypted_body_b32, alphabet) {
            let deob = obfuscation::custom_deobfuscate(&body_bytes, xor_key);
            aes_cbc::decrypt(&deob, &aes_key).unwrap_or_default()
        } else { Vec::new() }
    } else { Vec::new() };

    // 5. FORWARD REQUEST
    // We dynamically send the request to the backend microservice
    let method = Method::from_bytes(token.method.as_bytes()).unwrap_or(Method::POST);
    let mut req_builder = state.http_client.request(method, &token.api_url);

    let mut req_headers = HeaderMap::new();
    for (k, v) in token.api_header {
        if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(&v)) {
            req_headers.insert(name, val);
        }
    }
    
    // Add real ip propagation
    req_headers.insert("X-Forwarded-For", HeaderValue::from_str(ip).unwrap());

    req_builder = req_builder.headers(req_headers).body(decrypted_body);

    let start_time = chrono::Utc::now();
    let res = match req_builder.send().await {
        Ok(res) => {
            let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::OK);
            
            // Extract response type, set to application/json by default
            let mut response_headers = HeaderMap::new();
            if let Some(content_type) = res.headers().get("content-type") {
                 response_headers.insert("content-type", content_type.clone());
            }

            let res_bytes = res.bytes().await.unwrap_or_default();
            state.logger.add_log(crate::services::logs::RequestLog {
                timestamp: start_time,
                method: method.to_string(),
                path: "PROXY HIDDEN".to_string(),
                target_url: token.api_url.clone(),
                ip: ip.to_string(),
                status: status.as_u16(),
                execution_ms: (chrono::Utc::now() - start_time).num_milliseconds() as u64,
            }).await;
            (status, response_headers, res_bytes.to_vec()).into_response()
        }
        Err(e) => {
            state.logger.add_log(crate::services::logs::RequestLog {
                timestamp: start_time,
                method: method.to_string(),
                path: "PROXY HIDDEN".to_string(),
                target_url: token.api_url.clone(),
                ip: ip.to_string(),
                status: 502,
                execution_ms: (chrono::Utc::now() - start_time).num_milliseconds() as u64,
            }).await;
            (StatusCode::BAD_GATEWAY, Json(json!({"error": "Failed to reach backend service", "details": e.to_string()}))).into_response()
        }
    };
    
    res
}
