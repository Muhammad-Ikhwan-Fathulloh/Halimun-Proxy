use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Isi token yang didekripsi dari URL / body.
///
/// Protokol HMAC (lihat docs/PANDUAN.md):
/// `hmac` = HMAC-SHA256(hmac_key, JSON kanonik) dengan JSON kanonik adalah
/// objek tanpa field `hmac`, kunci diurutkan secara leksikografis, tanpa spasi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalimunToken {
    #[serde(default)]
    pub bypass_url: Option<bool>,
    pub api_url: String,
    #[serde(default)]
    pub api_header: HashMap<String, String>,
    pub method: String,
    pub timestamp: i64,
    pub expired: i64,
    #[serde(default)]
    pub offset: Option<String>,
    pub nonce: String,
    /// SHA-256 (hex) dari plaintext body. Wajib jika request mengirim body x=.
    #[serde(default)]
    pub body_sha256: Option<String>,
    #[serde(default)]
    pub hmac: Option<String>,
}
