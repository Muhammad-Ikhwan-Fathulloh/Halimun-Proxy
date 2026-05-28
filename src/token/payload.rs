use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalimunToken {
    pub bypass_url: Option<bool>,
    pub api_url: String,
    pub api_header: HashMap<String, String>,
    pub method: String,
    pub timestamp: i64,
    pub expired: i64,
    pub offset: Option<String>,
    pub nonce: String,

    // Kept optional since we remove it before validation
    pub hmac: Option<String>,
}
