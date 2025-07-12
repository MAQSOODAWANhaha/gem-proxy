use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub key: String,
    pub weight: u32,
    pub max_requests_per_minute: u32,
    #[serde(skip)]
    pub current_requests: u32,
    #[serde(skip)]
    pub last_reset: DateTime<Utc>,
    #[serde(skip)]
    pub is_active: bool,
    #[serde(skip)]
    pub failure_count: u32,
}

#[derive(Debug)]
pub struct KeyManager {
    keys: Arc<RwLock<Vec<ApiKey>>>,
    current_index: Arc<RwLock<usize>>,
}

impl KeyManager {
    pub fn new(keys: Vec<ApiKey>) -> Self {
        Self {
            keys: Arc::new(RwLock::new(keys)),
            current_index: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn get_next_key(&self) -> Option<ApiKey> {
        let mut keys = self.keys.write().await;
        let len = keys.len();
        if len == 0 {
            return None;
        }

        let mut index_guard = self.current_index.write().await;
        let start_index = *index_guard;

        for i in 0..len {
            let current_index = (start_index + i) % len;
            let key = &mut keys[current_index];

            if self.is_key_available(key).await {
                key.current_requests += 1;
                *index_guard = (current_index + 1) % len;
                return Some(key.clone());
            }
        }
        None
    }

    async fn is_key_available(&self, key: &mut ApiKey) -> bool {
        // Reset rate limit counter if needed
        if (Utc::now() - key.last_reset).num_seconds() >= 60 {
            key.current_requests = 0;
            key.last_reset = Utc::now();
        }

        key.is_active && key.current_requests < key.max_requests_per_minute && key.failure_count < 5
    }

    pub async fn mark_key_failed(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            key.failure_count += 1;
            if key.failure_count >= 5 {
                key.is_active = false;
            }
        }
    }

    pub async fn mark_key_success(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            key.failure_count = 0;
            key.is_active = true;
        }
    }
}
