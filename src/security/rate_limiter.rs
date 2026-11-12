use dashmap::DashMap;
use std::net::IpAddr;

/// Batas jumlah IP yang dilacak. Saat terlampaui, entri kedaluwarsa dibuang.
const MAX_TRACKED_IPS: usize = 100_000;

#[derive(Clone, Copy)]
struct Window {
    minute: i64,
    count: u32,
}

/// Fixed-window per IP: O(1) per request dan memori terbatas
/// (satu entri per IP, bukan satu entri per IP per menit seperti sebelumnya).
pub struct RateLimiter {
    windows: DashMap<IpAddr, Window>,
    limit: u32,
}

impl RateLimiter {
    pub fn new(limit_per_minute: u32) -> Self {
        Self {
            windows: DashMap::new(),
            limit: limit_per_minute,
        }
    }

    pub fn check(&self, ip: IpAddr) -> bool {
        let now_minute = chrono::Utc::now().timestamp() / 60;

        if self.windows.len() > MAX_TRACKED_IPS {
            self.windows.retain(|_, w| w.minute == now_minute);
        }

        let mut entry = self.windows.entry(ip).or_insert(Window {
            minute: now_minute,
            count: 0,
        });
        if entry.minute != now_minute {
            entry.minute = now_minute;
            entry.count = 0;
        }
        if entry.count >= self.limit {
            return false;
        }
        entry.count += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_limit_per_ip() {
        let rl = RateLimiter::new(2);
        let a: IpAddr = "10.0.0.1".parse().unwrap();
        let b: IpAddr = "10.0.0.2".parse().unwrap();
        assert!(rl.check(a));
        assert!(rl.check(a));
        assert!(!rl.check(a));
        assert!(rl.check(b));
    }
}
