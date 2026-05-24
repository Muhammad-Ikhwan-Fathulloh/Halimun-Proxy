use std::sync::Arc;
use dashmap::DashMap;
use tokio::time::{self, Duration};

#[derive(Clone)]
pub struct ReplayGuard {
    // Stores nonce -> expiration timestamp
    pub cache: Arc<DashMap<String, i64>>,
}

impl ReplayGuard {
    pub fn new() -> Self {
        let cache = Arc::new(DashMap::new());
        Self::start_cleaner(cache.clone());
        Self { cache }
    }

    pub fn check(&self, nonce: &str, ttl_seconds: i64) -> bool {
        if self.cache.contains_key(nonce) {
            return false;
        }
        
        let expires_at = chrono::Utc::now().timestamp() + ttl_seconds;
        self.cache.insert(nonce.to_string(), expires_at);
        true
    }

    fn start_cleaner(cache: Arc<DashMap<String, i64>>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = chrono::Utc::now().timestamp();
                // Retain elements that haven't expired yet
                cache.retain(|_, expires_at| *expires_at > now);
            }
        });
    }
}
