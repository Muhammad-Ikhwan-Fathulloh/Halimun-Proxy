use crate::token::payload::HalimunToken;
use crate::crypto::hmac;

pub fn validate_token(
    mut token: HalimunToken,
    hmac_key: &[u8],
    replay_guard: &crate::token::replay_guard::ReplayGuard,
) -> Result<HalimunToken, &'static str> {
    // 1. Extract and validate HMAC
    let expected_hmac = token.hmac.take().ok_or("Missing HMAC")?;
    
    // Convert back to canonical JSON without HMAC field
    let canonical = serde_json::to_string(&token).map_err(|_| "Failed to serialize canonical payload")?;
    
    if !hmac::verify_hmac(&canonical, hmac_key, &expected_hmac) {
        return Err("Invalid HMAC");
    }
    
    // Restore HMAC if needed later, but we authenticated the contents
    token.hmac = Some(expected_hmac);

    // 2. Validate Expiration
    let now = chrono::Utc::now().timestamp();
    if now > (token.timestamp + token.expired) {
        return Err("Expired");
    }

    // 3. Validate Nonce (Replay Attack Prevention)
    if !replay_guard.check(&token.nonce, token.expired) {
        return Err("Nonce replayed");
    }

    Ok(token)
}
