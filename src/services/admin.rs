use crate::config::{validate_service, ServiceConfig};
use crate::services::logs::Logger;
use crate::services::registry::ServiceRegistry;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{delete, get},
    Router,
};
use rand::RngCore;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AdminState {
    pub registry: Arc<ServiceRegistry>,
    api_key: Arc<String>,
    pub logger: Logger,
    config_path: Arc<String>,
    allow_keygen: bool,
    /// Serialisasi penulisan config.yaml agar dua request admin tidak saling menimpa.
    write_lock: Arc<Mutex<()>>,
    started: Instant,
    xor_key: u8,
    alphabet: Arc<String>,
}

pub struct AdminOptions {
    pub allow_keygen: bool,
    pub xor_key: u8,
    pub alphabet: String,
}

pub fn router(
    registry: Arc<ServiceRegistry>,
    api_key: String,
    logger: Logger,
    config_path: String,
    opts: AdminOptions,
) -> Router {
    let state = AdminState {
        registry,
        api_key: Arc::new(api_key),
        logger,
        config_path: Arc::new(config_path),
        allow_keygen: opts.allow_keygen,
        write_lock: Arc::new(Mutex::new(())),
        started: Instant::now(),
        xor_key: opts.xor_key,
        alphabet: Arc::new(opts.alphabet),
    };

    Router::new()
        .route("/services", get(list_services).post(add_service))
        .route("/services/:name", delete(delete_service))
        .route("/stats", get(get_stats))
        .route("/logs", get(get_logs))
        .route("/keys/generate", get(generate_keys_api))
        .with_state(state)
}

/// Perbandingan konstan-waktu untuk mencegah timing attack pada kunci admin.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn is_authorized(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .map(|k| ct_eq(k.as_bytes(), expected.as_bytes()))
        .unwrap_or(false)
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "Unauthorized", "code": 401})),
    )
        .into_response()
}

/// Menulis ulang config.yaml hanya pada bagian `services`.
/// Nilai dari environment (kunci rahasia) tidak ikut tertulis ke file.
async fn persist_services(state: &AdminState) -> Result<(), String> {
    let _guard = state.write_lock.lock().await;
    let path = state.config_path.as_str();
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut doc: serde_yaml::Value = serde_yaml::from_str(&raw).map_err(|e| e.to_string())?;
    let services =
        serde_yaml::to_value(state.registry.all_services()).map_err(|e| e.to_string())?;
    let map = doc
        .as_mapping_mut()
        .ok_or_else(|| "config.yaml harus berupa mapping".to_string())?;
    map.insert(serde_yaml::Value::String("services".into()), services);

    let yaml = serde_yaml::to_string(&doc).map_err(|e| e.to_string())?;
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, yaml).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

async fn add_service(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(config): Json<ServiceConfig>,
) -> Response {
    if !is_authorized(&headers, &state.api_key) {
        return unauthorized();
    }
    if let Err(e) = validate_service(&config) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e, "code": 400})),
        )
            .into_response();
    }

    let name = config.name.clone();
    let previous = state.registry.get_service(&name);
    state.registry.register_service(config);
    if let Err(e) = persist_services(&state).await {
        // Kembalikan keadaan registry jika penyimpanan gagal
        match previous {
            Some(prev) => state.registry.register_service(prev),
            None => state.registry.remove_service(&name),
        }
        eprintln!("gagal menyimpan config: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to persist configuration", "code": 500})),
        )
            .into_response();
    }
    (StatusCode::CREATED, Json(json!({"status": "success"}))).into_response()
}

async fn delete_service(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !is_authorized(&headers, &state.api_key) {
        return unauthorized();
    }
    let Some(previous) = state.registry.get_service(&name) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Service not found", "code": 404})),
        )
            .into_response();
    };
    state.registry.remove_service(&name);
    if let Err(e) = persist_services(&state).await {
        state.registry.register_service(previous);
        eprintln!("gagal menyimpan config: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to persist configuration", "code": 500})),
        )
            .into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn list_services(State(state): State<AdminState>, headers: HeaderMap) -> Response {
    if !is_authorized(&headers, &state.api_key) {
        return unauthorized();
    }
    Json(state.registry.all_services()).into_response()
}

async fn get_stats(State(state): State<AdminState>, headers: HeaderMap) -> Response {
    if !is_authorized(&headers, &state.api_key) {
        return unauthorized();
    }
    Json(json!({
        "status": "healthy",
        "uptime_seconds": state.started.elapsed().as_secs(),
        "services": state.registry.all_services().len(),
        "log_entries": state.logger.get_logs().len(),
    }))
    .into_response()
}

async fn get_logs(State(state): State<AdminState>, headers: HeaderMap) -> Response {
    if !is_authorized(&headers, &state.api_key) {
        return unauthorized();
    }
    Json(state.logger.get_logs()).into_response()
}

/// Membuat kunci acak baru. Nonaktif kecuali `security.allow_key_generation: true`,
/// karena endpoint ini mengembalikan rahasia dalam bentuk plaintext.
async fn generate_keys_api(State(state): State<AdminState>, headers: HeaderMap) -> Response {
    if !is_authorized(&headers, &state.api_key) {
        return unauthorized();
    }
    if !state.allow_keygen {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Key generation disabled (security.allow_key_generation)", "code": 404})),
        )
            .into_response();
    }

    let mut aes_key_bytes = [0u8; 32];
    let mut hmac_key_bytes = [0u8; 32];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut aes_key_bytes);
    rng.fill_bytes(&mut hmac_key_bytes);

    Json(json!({
        "HALIMUN_AES_KEY": hex::encode(aes_key_bytes),
        "HALIMUN_HMAC_KEY": hex::encode(hmac_key_bytes),
        "HALIMUN_XOR_KEY": state.xor_key,
        "HALIMUN_BASE32_ALPHABET": state.alphabet.as_str(),
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::ct_eq;

    #[test]
    fn constant_time_eq_works() {
        assert!(ct_eq(b"secret", b"secret"));
        assert!(!ct_eq(b"secret", b"secreT"));
        assert!(!ct_eq(b"secret", b"secre"));
    }
}
