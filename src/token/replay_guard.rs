use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;

/// Batas jumlah nonce aktif di memori (mencegah kehabisan RAM saat banjir).
pub const MAX_NONCES: usize = 1_000_000;

#[derive(Debug, PartialEq, Eq)]
pub enum NonceCheck {
    Fresh,
    Replayed,
    Full,
}

/// Menyimpan nonce -> waktu kedaluwarsa (unix detik).
/// Pemeriksaan dan penyisipan dilakukan atomik lewat `entry` (tanpa celah TOCTOU).
#[derive(Clone)]
pub struct ReplayGuard {
    pub cache: Arc<DashMap<String, i64>>,
}

impl Default for ReplayGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayGuard {
    /// Membutuhkan runtime Tokio aktif (membuat task pembersih berkala).
    pub fn new() -> Self {
        let cache = Arc::new(DashMap::new());
        Self::start_cleaner(cache.clone());
        Self { cache }
    }

    pub fn check(&self, nonce: &str, ttl_seconds: i64) -> NonceCheck {
        let now = chrono::Utc::now().timestamp();
        if let Some(expiry) = self.cache.get(nonce) {
            if *expiry > now {
                return NonceCheck::Replayed;
            }
        }
        if self.cache.len() >= MAX_NONCES {
            return NonceCheck::Full;
        }
        let expires_at = now + ttl_seconds;
        let mut slot = self.cache.entry(nonce.to_string()).or_insert(0);
        if *slot > now {
            return NonceCheck::Replayed;
        }
        *slot = expires_at;
        NonceCheck::Fresh
    }

    fn start_cleaner(cache: Arc<DashMap<String, i64>>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = chrono::Utc::now().timestamp();
                cache.retain(|_, expires_at| *expires_at > now);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fresh_then_replayed() {
        let g = ReplayGuard::new();
        assert_eq!(g.check("abc", 60), NonceCheck::Fresh);
        assert_eq!(g.check("abc", 60), NonceCheck::Replayed);
        assert_eq!(g.check("xyz", 60), NonceCheck::Fresh);
    }
}
