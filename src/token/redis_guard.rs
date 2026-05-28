use dashmap::DashMap;
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct RedisGuard {
    local_cache: DashMap<String, u64>,
    redis_client: Option<redis::Client>,
    ttl_seconds: u64,
}

impl RedisGuard {
    pub fn new(ttl_seconds: u64, redis_url: Option<String>) -> Self {
        let redis_client = redis_url.and_then(|url| redis::Client::open(url).ok());

        Self {
            local_cache: DashMap::new(),
            redis_client,
            ttl_seconds,
        }
    }

    pub async fn check_and_store_nonce(&self, nonce: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 1. Check Redis if available
        if let Some(client) = &self.redis_client {
            if let Ok(mut con) = client.get_multiplexed_async_connection().await {
                let redis_key = format!("halimun:nonce:{}", nonce);
                let is_new: bool = con.set_nx(&redis_key, now).await.unwrap_or(false);
                if is_new {
                    let _: () = con
                        .expire(&redis_key, self.ttl_seconds as i64)
                        .await
                        .unwrap_or(());
                }
                return is_new; // If true, it was set (new). If false, it already existed (replay attack)
            }
        }

        // 2. Fallback to Local DashMap if Redis is unreachable or not configured
        if self.local_cache.contains_key(nonce) {
            return false;
        }
        self.local_cache
            .insert(nonce.to_string(), now + self.ttl_seconds);
        true
    }

    pub fn clean_expired_local(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.local_cache.retain(|_, expiry| *expiry > now);
    }
}
