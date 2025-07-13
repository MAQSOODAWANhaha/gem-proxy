use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::load_balancer::weighted_round_robin::{WeightedRoundRobin, WeightStats};

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
#[allow(dead_code)]
pub struct KeyManager {
    /// 加权轮询调度器
    scheduler: Arc<WeightedRoundRobin>,
    /// 原始密钥存储，用于速率限制和状态管理
    keys: Arc<RwLock<Vec<ApiKey>>>,
}

impl KeyManager {
    #[allow(dead_code)]
    pub fn new(keys: Vec<ApiKey>) -> Self {
        let scheduler = Arc::new(WeightedRoundRobin::new(keys.clone()));
        Self {
            scheduler,
            keys: Arc::new(RwLock::new(keys)),
        }
    }

    /// 获取下一个可用的 API 密钥（使用加权轮询算法）
    pub async fn get_next_key(&self) -> Option<ApiKey> {
        // 首先更新密钥的可用状态和速率限制
        self.update_keys_availability().await;
        
        // 使用加权轮询调度器选择密钥
        if let Some(mut selected_key) = self.scheduler.select_key().await {
            // 更新该密钥的请求计数
            let mut keys = self.keys.write().await;
            if let Some(key) = keys.iter_mut().find(|k| k.id == selected_key.id) {
                key.current_requests += 1;
                // 更新选中的密钥信息
                selected_key = key.clone();
            }
            Some(selected_key)
        } else {
            None
        }
    }

    /// 更新所有密钥的可用状态和速率限制
    async fn update_keys_availability(&self) {
        let mut keys = self.keys.write().await;
        let now = Utc::now();
        
        for key in keys.iter_mut() {
            // 重置速率限制计数器
            if (now - key.last_reset).num_seconds() >= 60 {
                key.current_requests = 0;
                key.last_reset = now;
            }

            // 检查密钥是否应该被标记为可用
            let should_be_active = key.failure_count < 5 
                && key.current_requests < key.max_requests_per_minute;

            if key.is_active != should_be_active {
                key.is_active = should_be_active;
                
                // 同步到调度器
                if should_be_active {
                    self.scheduler.mark_key_success(&key.id).await;
                } else {
                    self.scheduler.mark_key_failed(&key.id).await;
                }
            }
        }
    }

    /// 标记密钥失败
    pub async fn mark_key_failed(&self, key_id: &str) {
        // 更新本地密钥状态
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            key.failure_count += 1;
            if key.failure_count >= 5 {
                key.is_active = false;
            }
        }
        
        // 同步到调度器
        self.scheduler.mark_key_failed(key_id).await;
    }

    /// 标记密钥成功
    pub async fn mark_key_success(&self, key_id: &str) {
        // 更新本地密钥状态
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            key.failure_count = 0;
            key.is_active = true;
        }
        
        // 同步到调度器
        self.scheduler.mark_key_success(key_id).await;
    }

    /// 添加新的 API 密钥
    pub async fn add_key(&self, api_key: ApiKey) {
        // 添加到本地存储
        self.keys.write().await.push(api_key.clone());
        
        // 添加到调度器
        self.scheduler.add_key(api_key).await;
    }

    /// 移除 API 密钥
    pub async fn remove_key(&self, key_id: &str) -> bool {
        // 从本地存储移除
        let mut keys = self.keys.write().await;
        let removed = if let Some(pos) = keys.iter().position(|k| k.id == key_id) {
            keys.remove(pos);
            true
        } else {
            false
        };
        
        // 从调度器移除
        if removed {
            self.scheduler.remove_key(key_id).await;
        }
        
        removed
    }

    /// 更新密钥权重
    pub async fn update_key_weight(&self, key_id: &str, new_weight: u32) -> bool {
        // 更新本地存储
        let mut keys = self.keys.write().await;
        let updated = if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            key.weight = new_weight;
            true
        } else {
            false
        };
        
        // 更新调度器
        if updated {
            self.scheduler.update_key_weight(key_id, new_weight).await;
        }
        
        updated
    }

    /// 获取所有密钥状态
    pub async fn get_all_keys(&self) -> Vec<ApiKey> {
        self.keys.read().await.clone()
    }

    /// 获取权重分配统计
    pub async fn get_weight_stats(&self) -> WeightStats {
        self.scheduler.get_weight_stats().await
    }

    /// 获取活跃密钥数量
    pub async fn get_active_keys_count(&self) -> usize {
        self.keys
            .read()
            .await
            .iter()
            .filter(|k| k.is_active && k.weight > 0)
            .count()
    }

    /// 检查是否有可用的密钥
    pub async fn has_available_keys(&self) -> bool {
        self.get_active_keys_count().await > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_api_key(id: &str, weight: u32) -> ApiKey {
        ApiKey {
            id: id.to_string(),
            key: format!("test-key-{}", id),
            weight,
            max_requests_per_minute: 60,
            current_requests: 0,
            last_reset: Utc::now(),
            is_active: true,
            failure_count: 0,
        }
    }

    #[tokio::test]
    async fn test_weighted_key_selection() {
        let keys = vec![
            create_test_api_key("key1", 100),
            create_test_api_key("key2", 200),
            create_test_api_key("key3", 300),
        ];

        let key_manager = KeyManager::new(keys);

        // 测试多次选择，验证权重分配
        let mut selections = std::collections::HashMap::new();
        for _ in 0..600 {
            if let Some(key) = key_manager.get_next_key().await {
                *selections.entry(key.id).or_insert(0) += 1;
            }
        }

        // 验证权重比例大致正确（允许一定误差）
        let key1_count = selections.get("key1").unwrap_or(&0);
        let key2_count = selections.get("key2").unwrap_or(&0);
        let key3_count = selections.get("key3").unwrap_or(&0);

        println!("选择统计: key1={}, key2={}, key3={}", key1_count, key2_count, key3_count);

        // 权重比例应该大约是 1:2:3
        assert!(*key1_count > 80 && *key1_count < 120); // 约100次
        assert!(*key2_count > 180 && *key2_count < 220); // 约200次  
        assert!(*key3_count > 280 && *key3_count < 320); // 约300次
    }

    #[tokio::test]
    async fn test_weight_update() {
        let keys = vec![
            create_test_api_key("key1", 100),
            create_test_api_key("key2", 100),
        ];

        let key_manager = KeyManager::new(keys);

        // 更新权重
        assert!(key_manager.update_key_weight("key1", 300).await);
        
        // 验证权重统计
        let stats = key_manager.get_weight_stats().await;
        assert_eq!(stats.total_weight, 400);
        assert_eq!(stats.active_keys_count, 2);
        
        let key1_info = stats.key_distributions
            .iter()
            .find(|k| k.key_id == "key1")
            .unwrap();
        assert_eq!(key1_info.weight, 300);
        assert!((key1_info.percentage - 75.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_key_failure_and_recovery() {
        let keys = vec![
            create_test_api_key("key1", 100),
            create_test_api_key("key2", 100),
        ];

        let key_manager = KeyManager::new(keys);

        // 标记密钥失败多次
        for _ in 0..5 {
            key_manager.mark_key_failed("key1").await;
        }

        // 验证密钥被标记为不活跃
        let stats = key_manager.get_weight_stats().await;
        assert_eq!(stats.active_keys_count, 1);
        assert_eq!(stats.total_weight, 100);

        // 恢复密钥
        key_manager.mark_key_success("key1").await;
        let stats = key_manager.get_weight_stats().await;
        assert_eq!(stats.active_keys_count, 2);
        assert_eq!(stats.total_weight, 200);
    }

    #[tokio::test]
    async fn test_add_and_remove_keys() {
        let keys = vec![create_test_api_key("key1", 100)];
        let key_manager = KeyManager::new(keys);

        // 添加新密钥
        let new_key = create_test_api_key("key2", 200);
        key_manager.add_key(new_key).await;

        let stats = key_manager.get_weight_stats().await;
        assert_eq!(stats.active_keys_count, 2);
        assert_eq!(stats.total_weight, 300);

        // 移除密钥
        assert!(key_manager.remove_key("key1").await);
        let stats = key_manager.get_weight_stats().await;
        assert_eq!(stats.active_keys_count, 1);
        assert_eq!(stats.total_weight, 200);

        // 尝试移除不存在的密钥
        assert!(!key_manager.remove_key("non_existent").await);
    }

    #[tokio::test]
    async fn test_zero_weight_key() {
        let keys = vec![
            create_test_api_key("key1", 0),  // 零权重
            create_test_api_key("key2", 100),
        ];

        let key_manager = KeyManager::new(keys);

        // 只有key2应该被选中
        for _ in 0..10 {
            let selected = key_manager.get_next_key().await.unwrap();
            assert_eq!(selected.id, "key2");
        }
    }
}
