use std::sync::Arc;
use dashmap::DashMap;

pub struct RateLimiter {
    // Limits based on IP
    // Format: "ip_addr_with_minute:count"
    requests: Arc<DashMap<String, u32>>,
    limit: u32,
}

impl RateLimiter {
    pub fn new(limit_per_minute: u32) -> Self {
        Self {
            requests: Arc::new(DashMap::new()),
            limit: limit_per_minute,
        }
    }

    pub fn check(&self, ip: &str) -> bool {
        let now = chrono::Utc::now();
        let minute_key = format!("{}-{}", ip, now.format("%Y-%m-%d-%H-%M"));
        
        let mut count = self.requests.entry(minute_key).or_insert(0);
        if *count >= self.limit {
            return false;
        }
        *count += 1;
        true
    }
}
