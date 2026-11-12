use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// HMAC-SHA256 dalam format hex.
pub fn compute_hmac(data: &str, key: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(key).expect("HMAC menerima kunci berukuran berapa pun");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Verifikasi HMAC dengan perbandingan konstan-waktu (`verify_slice`).
pub fn verify_hmac(data: &str, key: &[u8], expected_mac_hex: &str) -> bool {
    let Ok(expected) = hex::decode(expected_mac_hex) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(key) else {
        return false;
    };
    mac.update(data.as_bytes());
    mac.verify_slice(&expected).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_roundtrip() {
        let mac = compute_hmac("{\"a\":1}", b"key");
        assert!(verify_hmac("{\"a\":1}", b"key", &mac));
        assert!(!verify_hmac("{\"a\":2}", b"key", &mac));
        assert!(!verify_hmac("{\"a\":1}", b"key", "zz"));
    }
}
