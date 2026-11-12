use crate::config::MAX_TOKEN_TTL_SECS;
use crate::crypto::hmac;
use crate::token::payload::HalimunToken;
use crate::token::replay_guard::{NonceCheck, ReplayGuard};
use serde_json::Value;

/// Toleransi jam untuk token dari klien yang jamnya sedikit maju.
const CLOCK_SKEW_SECS: i64 = 60;
const MAX_NONCE_LEN: usize = 128;

/// Memvalidasi plaintext JSON token: struktur, HMAC, masa berlaku, dan nonce.
/// Urutan: HMAC dulu (agar penyerang tanpa kunci tidak bisa membakar nonce),
/// lalu waktu, lalu replay.
pub fn validate_payload(
    raw_json: &[u8],
    hmac_key: &[u8],
    replay_guard: &ReplayGuard,
    now: i64,
) -> Result<HalimunToken, &'static str> {
    let mut value: Value = serde_json::from_slice(raw_json).map_err(|_| "Invalid token format")?;
    let obj = value.as_object_mut().ok_or("Invalid token format")?;

    let expected_hmac = obj
        .remove("hmac")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or("Missing HMAC")?;

    // JSON kanonik: serde_json menyimpan objek sebagai BTreeMap (kunci terurut).
    let canonical = serde_json::to_string(&value).map_err(|_| "Invalid token format")?;
    if !hmac::verify_hmac(&canonical, hmac_key, &expected_hmac) {
        return Err("Invalid HMAC");
    }

    let mut token: HalimunToken =
        serde_json::from_value(value).map_err(|_| "Invalid token format")?;

    if token.nonce.is_empty() || token.nonce.len() > MAX_NONCE_LEN {
        return Err("Invalid nonce");
    }
    if token.expired <= 0 || token.expired > MAX_TOKEN_TTL_SECS {
        return Err("Invalid expiry");
    }
    if token.timestamp > now + CLOCK_SKEW_SECS {
        return Err("Token timestamp is in the future");
    }
    if now > token.timestamp + token.expired {
        return Err("Expired");
    }

    match replay_guard.check(&token.nonce, token.expired) {
        NonceCheck::Fresh => {}
        NonceCheck::Replayed => return Err("Nonce replayed"),
        NonceCheck::Full => return Err("Server busy, try again later"),
    }

    token.hmac = Some(expected_hmac);
    Ok(token)
}

/// Menandatangani objek JSON: menambahkan field `hmac` dan mengembalikan
/// JSON final. Dipakai untuk pengujian dan sebagai referensi bagi klien.
pub fn sign_payload(mut value: Value, hmac_key: &[u8]) -> String {
    if let Some(obj) = value.as_object_mut() {
        obj.remove("hmac");
    }
    let canonical = serde_json::to_string(&value).expect("JSON valid");
    let mac = hmac::compute_hmac(&canonical, hmac_key);
    value
        .as_object_mut()
        .expect("objek")
        .insert("hmac".into(), Value::String(mac));
    serde_json::to_string(&value).expect("JSON valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn payload(nonce: &str, ts: i64) -> Value {
        json!({
            "api_url": "http://127.0.0.1:1/x",
            "api_header": {"Content-Type": "application/json"},
            "method": "POST",
            "timestamp": ts,
            "expired": 60,
            "nonce": nonce
        })
    }

    #[tokio::test]
    async fn valid_token_then_replay() {
        let key = [3u8; 32];
        let guard = ReplayGuard::new();
        let now = 1_700_000_000;
        let raw = sign_payload(payload("n1", now), &key);
        assert!(validate_payload(raw.as_bytes(), &key, &guard, now).is_ok());
        assert_eq!(
            validate_payload(raw.as_bytes(), &key, &guard, now).unwrap_err(),
            "Nonce replayed"
        );
    }

    #[tokio::test]
    async fn tampered_token_fails_hmac() {
        let key = [3u8; 32];
        let guard = ReplayGuard::new();
        let now = 1_700_000_000;
        let raw = sign_payload(payload("n2", now), &key);
        let tampered = raw.replace("POST", "GET");
        assert_eq!(
            validate_payload(tampered.as_bytes(), &key, &guard, now).unwrap_err(),
            "Invalid HMAC"
        );
    }

    #[tokio::test]
    async fn expired_token_rejected() {
        let key = [3u8; 32];
        let guard = ReplayGuard::new();
        let now = 1_700_000_000;
        let raw = sign_payload(payload("n3", now - 1000), &key);
        assert_eq!(
            validate_payload(raw.as_bytes(), &key, &guard, now).unwrap_err(),
            "Expired"
        );
    }
}
