use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use crate::services::registry::ServiceRegistry;
use crate::config::AppConfig;
use serde_json::json;

use crate::services::logs::Logger;

#[derive(Clone)]
pub struct AdminState {
    pub registry: Arc<ServiceRegistry>,
    pub api_key: String,
    pub logger: Logger,
}

pub fn router(registry: Arc<ServiceRegistry>, api_key: String, logger: Logger) -> Router {
    let state = AdminState { registry, api_key, logger };
    
    Router::new()
        .route("/services", get(list_services))
        .route("/stats", get(get_stats))
        .route("/logs", get(get_logs))
        .route("/keys/generate", get(generate_keys_api))
        .with_state(state)
}

async fn validate_admin(headers: &HeaderMap, expected_key: &str) -> bool {
    headers.get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .map(|k| k == expected_key)
        .unwrap_or(false)
}

async fn list_services(
    State(state): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !validate_admin(&headers, &state.api_key).await {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    
    let services = state.registry.all_services();
    Json(services).into_response()
}

async fn get_stats(
    State(state): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !validate_admin(&headers, &state.api_key).await {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    
    // Add real stats here
    Json(json!({"status": "healthy", "uptime_minutes": 10})).into_response()
}

async fn get_logs(
    State(state): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !validate_admin(&headers, &state.api_key).await {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    
    let logs = state.logger.get_logs().await;
    Json(logs).into_response()
}

async fn generate_keys_api(
    State(state): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !validate_admin(&headers, &state.api_key).await {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    
    // Call keygen generator inline locally for the API response.
    // Instead of printing, return as JSON.
    use rand::RngCore;
    let mut aes_key_bytes = [0u8; 32];
    let mut hmac_key_bytes = [0u8; 32];
    
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut aes_key_bytes);
    rng.fill_bytes(&mut hmac_key_bytes);
    
    let output = json!({
        "HALIMUN_AES_KEY": hex::encode(aes_key_bytes),
        "HALIMUN_HMAC_KEY": hex::encode(hmac_key_bytes),
        "HALIMUN_XOR_KEY": 172,
        "HALIMUN_BASE32_ALPHABET": "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
    });
    
    Json(output).into_response()
}
