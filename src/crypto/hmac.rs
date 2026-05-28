use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn compute_hmac(data: &str, key: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn verify_hmac(data: &str, key: &[u8], expected_mac_hex: &str) -> bool {
    // We compute and do string comparison, but constant-time compare is better
    // Since expected is hex string, we can compute ours and compare hex strings
    // using constant time equality or just bytes.
    if let Ok(expected_bytes) = hex::decode(expected_mac_hex) {
        if let Ok(mut mac) = HmacSha256::new_from_slice(key) {
            mac.update(data.as_bytes());
            if mac.verify_slice(&expected_bytes).is_ok() {
                return true;
            }
        }
    }
    false
}
