//! Uji end-to-end: klien (enkripsi) -> Halimun (validasi + forward) -> backend mock.
//! Semua berjalan di loopback. Target 127.0.0.1 hanya diizinkan lewat whitelist bypass.

use axum::{body::Bytes, http::HeaderMap, routing::post, Router};
use halimun_proxy::config::AppConfig;
use halimun_proxy::proxy::handler::{router, ProxyState};
use halimun_proxy::services::logs::Logger;
use halimun_proxy::services::registry::ServiceRegistry;
use halimun_proxy::token::validator::sign_payload;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::Arc;

const AES_HEX: &str = "4d0f1e2a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6";
const HMAC_HEX: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";

fn config(whitelist: Vec<String>) -> AppConfig {
    let mut cfg = AppConfig::default();
    cfg.server.admin_api_key = "integration-test-admin-key".into();
    cfg.encryption.aes_key = AES_HEX.into();
    cfg.encryption.hmac_key = HMAC_HEX.into();
    cfg.security.bypass_whitelist = whitelist;
    cfg.security.rate_limit_per_minute = 1000;
    cfg
}

/// Backend palsu: mengembalikan method, header Content-Type, dan body yang diterima.
async fn spawn_backend() -> SocketAddr {
    async fn echo(headers: HeaderMap, body: Bytes) -> String {
        let ct = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        json!({"content_type": ct, "body": String::from_utf8_lossy(&body)}).to_string()
    }
    let app = Router::new().route("/echo", post(echo));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

async fn spawn_proxy(cfg: AppConfig) -> (SocketAddr, ProxyState) {
    let state = ProxyState::new(cfg, Arc::new(ServiceRegistry::new(vec![])), Logger::new(50))
        .expect("state valid");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(state.clone()).into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, state)
}

fn token(api_url: &str, nonce: &str, bypass: bool, now: i64) -> Value {
    json!({
        "api_url": api_url,
        "api_header": {"Content-Type": "application/json", "X-Forwarded-For": "1.2.3.4"},
        "method": "POST",
        "timestamp": now,
        "expired": 60,
        "nonce": nonce,
        "bypass_url": bypass
    })
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[tokio::test(flavor = "multi_thread")]
async fn encrypted_request_is_forwarded_and_replay_is_blocked() {
    let backend = spawn_backend().await;
    let (proxy, state) = spawn_proxy(config(vec!["127.0.0.1".into()])).await;
    let hmac_key = hex::decode(HMAC_HEX).unwrap();
    let client = reqwest::Client::new();

    let api_url = format!("http://{backend}/echo");
    let body_plain = br#"{"email":"user@example.com"}"#;
    let mut t = token(&api_url, "nonce-1", true, now());
    t["body_sha256"] = json!(hex::encode(Sha256::digest(body_plain)));
    let raw = sign_payload(t, &hmac_key);
    let seg = state.codec.seal(raw.as_bytes()).unwrap();
    let body_enc = state.codec.seal(body_plain).unwrap();
    let url = format!("http://{proxy}/proxy/1/{seg}/AAAA/BBBB/CCCC/DDDD");

    let ok = client
        .post(&url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(format!("x={body_enc}"))
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
    let echoed: Value = serde_json::from_str(&ok.text().await.unwrap()).unwrap();
    assert_eq!(echoed["body"], String::from_utf8_lossy(body_plain).as_ref());
    assert_eq!(echoed["content_type"], "application/json");

    // Body yang tidak diikat hash ke token -> ditolak (integritas body)
    let raw2 = sign_payload(token(&api_url, "nonce-1b", true, now()), &hmac_key);
    let seg2 = state.codec.seal(raw2.as_bytes()).unwrap();
    let url2 = format!("http://{proxy}/proxy/1/{seg2}/AAAA/BBBB/CCCC/DDDD");
    let no_hash = client
        .post(&url2)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(format!("x={body_enc}"))
        .send()
        .await
        .unwrap();
    assert_eq!(no_hash.status(), 403);

    // Token yang sama dikirim ulang -> ditolak sebagai replay
    let replay = client
        .post(&url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(format!("x={body_enc}"))
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), 403);
}

#[tokio::test(flavor = "multi_thread")]
async fn tampered_token_and_internal_targets_are_rejected() {
    let backend = spawn_backend().await;
    let (proxy, state) = spawn_proxy(config(vec![])).await;
    let hmac_key = hex::decode(HMAC_HEX).unwrap();
    let client = reqwest::Client::new();

    // 1. HMAC rusak (method diubah setelah ditandatangani)
    let raw = sign_payload(
        token(&format!("http://{backend}/echo"), "nonce-2", false, now()),
        &hmac_key,
    );
    let tampered = raw.replace("POST", "PUT");
    let seg = state.codec.seal(tampered.as_bytes()).unwrap();
    let res = client
        .post(format!("http://{proxy}/proxy/1/{seg}/A/B/C/D"))
        .body("")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    // 2. Token valid tetapi menuju alamat loopback tanpa bypass -> SSRF ditolak
    let raw = sign_payload(
        token(&format!("http://{backend}/echo"), "nonce-3", false, now()),
        &hmac_key,
    );
    let seg = state.codec.seal(raw.as_bytes()).unwrap();
    let res = client
        .post(format!("http://{proxy}/proxy/1/{seg}/A/B/C/D"))
        .body("")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
    let body: Value = res.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("internal"));

    // 3. Segmen acak / bukan token -> 400
    let res = client
        .post(format!(
            "http://{proxy}/proxy/1/AAAAAAAA/BBBB/CCCC/DDDD/EEEE"
        ))
        .body("")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
}
