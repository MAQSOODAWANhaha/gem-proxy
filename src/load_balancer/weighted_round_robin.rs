use crate::load_balancer::key_manager::ApiKey;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 加权轮询调度器
/// 
/// 使用平滑加权轮询算法(Smooth Weighted Round Robin)，
/// 确保请求按照权重比例分配，同时避免权重大的服务器连续被选中
#[derive(Debug)]
#[allow(dead_code)]
pub struct WeightedRoundRobin {
    /// 存储 API 密钥及其调度状态
    keys: Arc<RwLock<Vec<WeightedApiKey>>>,
    /// 总权重，用于优化计算
    total_weight: Arc<RwLock<i32>>,
}

/// 带权重调度信息的 API 密钥
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WeightedApiKey {
    /// 原始 API 密钥信息
    pub api_key: ApiKey,
    /// 当前权重值（用于平滑加权轮询算法）
    pub current_weight: i32,
    /// 有效权重值（静态配置的权重）
    pub effective_weight: i32,
}

impl WeightedApiKey {
    /// 创建新的带权重 API 密钥
    pub fn new(api_key: ApiKey) -> Self {
        let effective_weight = api_key.weight as i32;
        Self {
            api_key,
            current_weight: 0,
            effective_weight,
        }
    }

    /// 检查密钥是否可用
    pub fn is_available(&self) -> bool {
        self.api_key.is_active 
            && self.api_key.failure_count < 5 
            && self.effective_weight > 0
    }

    /// 增加当前权重
    pub fn increase_current_weight(&mut self) {
        self.current_weight += self.effective_weight;
    }

    /// 减少当前权重
    pub fn decrease_current_weight(&mut self, total_weight: i32) {
        self.current_weight -= total_weight;
    }

    /// 更新有效权重（用于动态调整）
    pub fn update_effective_weight(&mut self, new_weight: u32) {
        self.effective_weight = new_weight as i32;
        self.api_key.weight = new_weight;
    }
}

impl WeightedRoundRobin {
    /// 创建新的加权轮询调度器
    pub fn new(api_keys: Vec<ApiKey>) -> Self {
        let weighted_keys: Vec<WeightedApiKey> = api_keys
            .into_iter()
            .map(WeightedApiKey::new)
            .collect();

        let total_weight = weighted_keys
            .iter()
            .filter(|k| k.is_available())
            .map(|k| k.effective_weight)
            .sum();

        Self {
            keys: Arc::new(RwLock::new(weighted_keys)),
            total_weight: Arc::new(RwLock::new(total_weight)),
        }
    }

    /// 根据加权轮询算法选择下一个 API 密钥
    /// 
    /// 使用平滑加权轮询算法：
    /// 1. 所有可用密钥的 current_weight += effective_weight
    /// 2. 选择 current_weight 最大的密钥
    /// 3. 被选中的密钥的 current_weight -= total_weight
    pub async fn select_key(&self) -> Option<ApiKey> {
        let mut keys = self.keys.write().await;
        
        // 过滤可用的密钥
        let available_indices: Vec<usize> = keys
            .iter()
            .enumerate()
            .filter(|(_, key)| key.is_available())
            .map(|(i, _)| i)
            .collect();

        if available_indices.is_empty() {
            return None;
        }

        // 重新计算总权重
        let current_total_weight: i32 = available_indices
            .iter()
            .map(|&i| keys[i].effective_weight)
            .sum();

        if current_total_weight <= 0 {
            return None;
        }

        // 更新总权重
        *self.total_weight.write().await = current_total_weight;

        // 第一步：增加所有可用密钥的当前权重
        for &i in &available_indices {
            keys[i].increase_current_weight();
        }

        // 第二步：找到当前权重最大的密钥
        let selected_index = available_indices
            .iter()
            .max_by_key(|&&i| keys[i].current_weight)
            .copied()?;

        // 第三步：减少被选中密钥的当前权重
        keys[selected_index].decrease_current_weight(current_total_weight);

        // 返回选中的 API 密钥
        Some(keys[selected_index].api_key.clone())
    }

    /// 添加新的 API 密钥
    pub async fn add_key(&self, api_key: ApiKey) {
        let mut keys = self.keys.write().await;
        let weighted_key = WeightedApiKey::new(api_key);
        
        // 更新总权重
        if weighted_key.is_available() {
            let mut total = self.total_weight.write().await;
            *total += weighted_key.effective_weight;
        }
        
        keys.push(weighted_key);
    }

    /// 移除 API 密钥
    pub async fn remove_key(&self, key_id: &str) -> bool {
        let mut keys = self.keys.write().await;
        
        if let Some(pos) = keys.iter().position(|k| k.api_key.id == key_id) {
            let removed_key = keys.remove(pos);
            
            // 更新总权重
            if removed_key.is_available() {
                let mut total = self.total_weight.write().await;
                *total -= removed_key.effective_weight;
            }
            
            true
        } else {
            false
        }
    }

    /// 更新 API 密钥的权重
    pub async fn update_key_weight(&self, key_id: &str, new_weight: u32) -> bool {
        let mut keys = self.keys.write().await;
        
        if let Some(key) = keys.iter_mut().find(|k| k.api_key.id == key_id) {
            let old_weight = key.effective_weight;
            key.update_effective_weight(new_weight);
            
            // 更新总权重
            if key.is_available() {
                let mut total = self.total_weight.write().await;
                *total = *total - old_weight + key.effective_weight;
            }
            
            true
        } else {
            false
        }
    }

    /// 标记密钥失败
    pub async fn mark_key_failed(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        
        if let Some(key) = keys.iter_mut().find(|k| k.api_key.id == key_id) {
            key.api_key.failure_count += 1;
            
            // 如果失败次数过多，标记为不活跃
            if key.api_key.failure_count >= 5 {
                key.api_key.is_active = false;
                
                // 从总权重中移除
                let mut total = self.total_weight.write().await;
                *total -= key.effective_weight;
            }
        }
    }

    /// 标记密钥成功
    pub async fn mark_key_success(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        
        if let Some(key) = keys.iter_mut().find(|k| k.api_key.id == key_id) {
            let was_inactive = !key.api_key.is_active;
            
            key.api_key.failure_count = 0;
            key.api_key.is_active = true;
            
            // 如果之前不活跃，现在重新激活，需要加回总权重
            if was_inactive {
                let mut total = self.total_weight.write().await;
                *total += key.effective_weight;
            }
        }
    }

    /// 获取所有密钥的状态
    pub async fn get_keys_status(&self) -> Vec<WeightedApiKey> {
        self.keys.read().await.clone()
    }

    /// 获取权重分配统计
    pub async fn get_weight_stats(&self) -> WeightStats {
        let keys = self.keys.read().await;
        let total_weight = *self.total_weight.read().await;
        
        let active_keys: Vec<_> = keys
            .iter()
            .filter(|k| k.is_available())
            .collect();

        let key_distributions: Vec<KeyWeightInfo> = active_keys
            .iter()
            .map(|k| KeyWeightInfo {
                key_id: k.api_key.id.clone(),
                weight: k.effective_weight as u32,
                percentage: if total_weight > 0 {
                    (k.effective_weight as f64 / total_weight as f64) * 100.0
                } else {
                    0.0
                },
                current_weight: k.current_weight,
            })
            .collect();

        WeightStats {
            total_weight: total_weight as u32,
            active_keys_count: active_keys.len(),
            total_keys_count: keys.len(),
            key_distributions,
        }
    }
}

/// 权重统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct WeightStats {
    pub total_weight: u32,
    pub active_keys_count: usize,
    pub total_keys_count: usize,
    pub key_distributions: Vec<KeyWeightInfo>,
}

/// 单个密钥的权重信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyWeightInfo {
    pub key_id: String,
    pub weight: u32,
    pub percentage: f64,
    pub current_weight: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

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
    async fn test_weighted_round_robin_basic() {
        let keys = vec![
            create_test_api_key("key1", 100),
            create_test_api_key("key2", 200),
            create_test_api_key("key3", 300),
        ];

        let scheduler = WeightedRoundRobin::new(keys);

        // 测试多次选择，验证权重分配
        let mut selections = std::collections::HashMap::new();
        for _ in 0..600 {
            if let Some(key) = scheduler.select_key().await {
                *selections.entry(key.id).or_insert(0) += 1;
            }
        }

        // 验证权重比例大致正确（允许一定误差）
        let key1_count = selections.get("key1").unwrap_or(&0);
        let key2_count = selections.get("key2").unwrap_or(&0);
        let key3_count = selections.get("key3").unwrap_or(&0);

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

        let scheduler = WeightedRoundRobin::new(keys);

        // 更新权重
        assert!(scheduler.update_key_weight("key1", 300).await);
        
        // 验证权重统计
        let stats = scheduler.get_weight_stats().await;
        assert_eq!(stats.total_weight, 400);
        assert_eq!(stats.key_distributions.len(), 2);
        
        let key1_info = stats.key_distributions
            .iter()
            .find(|k| k.key_id == "key1")
            .unwrap();
        assert_eq!(key1_info.weight, 300);
        assert!((key1_info.percentage - 75.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_key_failure_handling() {
        let keys = vec![
            create_test_api_key("key1", 100),
            create_test_api_key("key2", 100),
        ];

        let scheduler = WeightedRoundRobin::new(keys);

        // 标记密钥失败多次
        for _ in 0..5 {
            scheduler.mark_key_failed("key1").await;
        }

        // 验证密钥被标记为不活跃
        let stats = scheduler.get_weight_stats().await;
        assert_eq!(stats.active_keys_count, 1);
        assert_eq!(stats.total_weight, 100);

        // 恢复密钥
        scheduler.mark_key_success("key1").await;
        let stats = scheduler.get_weight_stats().await;
        assert_eq!(stats.active_keys_count, 2);
        assert_eq!(stats.total_weight, 200);
    }

    #[tokio::test]
    async fn test_empty_keys() {
        let scheduler = WeightedRoundRobin::new(vec![]);
        assert!(scheduler.select_key().await.is_none());
    }

    #[tokio::test]
    async fn test_zero_weight_keys() {
        let keys = vec![
            create_test_api_key("key1", 0),
            create_test_api_key("key2", 100),
        ];

        let scheduler = WeightedRoundRobin::new(keys);
        
        // 只有key2应该被选中
        for _ in 0..10 {
            let selected = scheduler.select_key().await.unwrap();
            assert_eq!(selected.id, "key2");
        }
    }
}