use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone, serde::Serialize)]
pub struct RequestLog {
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub target_url: String,
    pub ip: String,
    pub status: u16,
    pub execution_ms: u64,
}

/// Ring buffer N log terakhir. Memakai `std::sync::Mutex` karena critical section
/// hanya satu push/pop (lebih ringan daripada RwLock async).
#[derive(Clone)]
pub struct Logger {
    logs: Arc<Mutex<VecDeque<RequestLog>>>,
    max_capacity: usize,
}

impl Logger {
    pub fn new(capacity: usize) -> Self {
        Self {
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            max_capacity: capacity.max(1),
        }
    }

    pub fn add_log(&self, log: RequestLog) {
        let mut queue = match self.logs.lock() {
            Ok(q) => q,
            Err(poisoned) => poisoned.into_inner(),
        };
        if queue.len() >= self.max_capacity {
            queue.pop_back();
        }
        queue.push_front(log);
    }

    pub fn get_logs(&self) -> Vec<RequestLog> {
        match self.logs.lock() {
            Ok(q) => q.iter().cloned().collect(),
            Err(poisoned) => poisoned.into_inner().iter().cloned().collect(),
        }
    }
}

/// Menghapus query string dan userinfo sebelum URL dicatat, agar token/rahasia
/// yang kebetulan ada di URL tidak bocor ke log atau dashboard.
pub fn sanitize_url(raw: &str) -> String {
    match url::Url::parse(raw) {
        Ok(mut u) => {
            let _ = u.set_username("");
            let _ = u.set_password(None);
            u.set_query(None);
            u.set_fragment(None);
            u.to_string()
        }
        Err(_) => "invalid-url".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_secrets_from_url() {
        let s = sanitize_url("https://u:p@api.example.com/v1/login?token=abc#frag");
        assert_eq!(s, "https://api.example.com/v1/login");
    }
}
