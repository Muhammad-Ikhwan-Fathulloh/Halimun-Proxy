use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

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

#[derive(Clone)]
pub struct Logger {
    // Ring buffer of last N logs
    logs: Arc<RwLock<VecDeque<RequestLog>>>,
    max_capacity: usize,
}

impl Logger {
    pub fn new(capacity: usize) -> Self {
        Self {
            logs: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            max_capacity: capacity,
        }
    }

    pub async fn add_log(&self, log: RequestLog) {
        let mut queue = self.logs.write().await;
        if queue.len() >= self.max_capacity {
            queue.pop_back();
        }
        queue.push_front(log);
    }

    pub async fn get_logs(&self) -> Vec<RequestLog> {
        let queue = self.logs.read().await;
        queue.iter().cloned().collect()
    }
}
