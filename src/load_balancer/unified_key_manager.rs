// src/load_balancer/unified_key_manager.rs
// 统一的负载均衡器状态管理器，消除 KeyManager 和 WeightedRoundRobin 的状态重复

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::load_balancer::key_manager::ApiKey;

/// 统一的 API 密钥结构，包含所有必要的状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedApiKey {
    // 基本配置信息
    pub id: String,
    pub key: String,
    pub weight: u32,
    pub max_requests_per_minute: u32,
    
    // 运行时状态（不序列化）
    #[serde(skip)]
    pub runtime_state: KeyRuntimeState,
    
    // 调度状态（不序列化）
    #[serde(skip)]
    pub scheduling_state: KeySchedulingState,
}

/// 密钥运行时状态
#[derive(Debug, Clone)]
pub struct KeyRuntimeState {
    pub current_requests: u32,
    pub last_reset: DateTime<Utc>,
    pub is_active: bool,
    pub failure_count: u32,
}

/// 密钥调度状态（用于加权轮询算法）
#[derive(Debug, Clone)]
pub struct KeySchedulingState {
    /// 当前权重值（用于平滑加权轮询算法）
    pub current_weight: i32,
    /// 有效权重值（配置的权重）
    pub effective_weight: i32,
}

impl Default for KeyRuntimeState {
    fn default() -> Self {
        Self {
            current_requests: 0,
            last_reset: Utc::now(),
            is_active: true,
            failure_count: 0,
        }
    }
}

impl Default for KeySchedulingState {
    fn default() -> Self {
        Self {
            current_weight: 0,
            effective_weight: 0,
        }
    }
}

impl UnifiedApiKey {
    /// 创建新的统一 API 密钥
    #[allow(dead_code)]
    pub fn new(id: String, key: String, weight: u32, max_requests_per_minute: u32) -> Self {
        Self {
            id,
            key,
            weight,
            max_requests_per_minute,
            runtime_state: KeyRuntimeState::default(),
            scheduling_state: KeySchedulingState {
                current_weight: 0,
                effective_weight: weight as i32,
            },
        }
    }
    
    /// 从旧的 ApiKey 创建统一密钥
    pub fn from_api_key(api_key: ApiKey) -> Self {
        Self {
            id: api_key.id,
            key: api_key.key,
            weight: api_key.weight,
            max_requests_per_minute: api_key.max_requests_per_minute,
            runtime_state: KeyRuntimeState {
                current_requests: api_key.current_requests,
                last_reset: api_key.last_reset,
                is_active: api_key.is_active,
                failure_count: api_key.failure_count,
            },
            scheduling_state: KeySchedulingState {
                current_weight: 0,
                effective_weight: api_key.weight as i32,
            },
        }
    }
    
    /// 转换为旧的 ApiKey 格式（兼容性）
    pub fn to_api_key(&self) -> ApiKey {
        ApiKey {
            id: self.id.clone(),
            key: self.key.clone(),
            weight: self.weight,
            max_requests_per_minute: self.max_requests_per_minute,
            current_requests: self.runtime_state.current_requests,
            last_reset: self.runtime_state.last_reset,
            is_active: self.runtime_state.is_active,
            failure_count: self.runtime_state.failure_count,
        }
    }
    
    /// 检查密钥是否可用
    pub fn is_available(&self) -> bool {
        self.runtime_state.is_active 
            && self.runtime_state.failure_count < 3
            && self.runtime_state.current_requests < self.max_requests_per_minute
    }
    
    /// 检查是否需要重置速率限制计数器
    pub fn should_reset_rate_limit(&self) -> bool {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.runtime_state.last_reset);
        elapsed.num_seconds() >= 60
    }
    
    /// 重置速率限制计数器
    pub fn reset_rate_limit(&mut self) {
        self.runtime_state.current_requests = 0;
        self.runtime_state.last_reset = Utc::now();
    }
    
    /// 增加请求计数
    pub fn increment_requests(&mut self) {
        self.runtime_state.current_requests += 1;
    }
    
    /// 标记密钥失败
    pub fn mark_failed(&mut self) {
        self.runtime_state.failure_count += 1;
        if self.runtime_state.failure_count >= 3 {
            self.runtime_state.is_active = false;
        }
        // 降低有效权重
        self.scheduling_state.effective_weight = 
            std::cmp::max(0, self.scheduling_state.effective_weight - 1);
    }
    
    /// 标记密钥成功
    pub fn mark_success(&mut self) {
        self.runtime_state.failure_count = 0;
        self.runtime_state.is_active = true;
        // 恢复有效权重
        self.scheduling_state.effective_weight = self.weight as i32;
    }
    
    /// 更新权重
    pub fn update_weight(&mut self, new_weight: u32) {
        self.weight = new_weight;
        self.scheduling_state.effective_weight = new_weight as i32;
    }
}

/// 负载均衡统计信息
#[derive(Debug, Clone, Serialize)]
pub struct LoadBalancingStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub total_weight: u32,
    pub total_requests: u32,
    pub failed_keys: usize,
}

/// 统一的负载均衡器状态管理器
/// 
/// 这个结构替代了原来的 KeyManager + WeightedRoundRobin 双重状态管理
/// 提供单一数据源和原子操作，确保状态一致性
#[derive(Debug)]
pub struct UnifiedKeyManager {
    /// 单一数据源：所有密钥的统一状态
    keys: Arc<RwLock<Vec<UnifiedApiKey>>>,
    /// 总权重缓存，避免重复计算
    total_weight: Arc<RwLock<i32>>,
}

impl UnifiedKeyManager {
    /// 创建新的统一密钥管理器
    pub fn new(keys: Vec<ApiKey>) -> Self {
        let unified_keys: Vec<UnifiedApiKey> = keys.into_iter()
            .map(UnifiedApiKey::from_api_key)
            .collect();
        
        let total_weight = unified_keys.iter()
            .map(|k| k.scheduling_state.effective_weight)
            .sum();
        
        Self {
            keys: Arc::new(RwLock::new(unified_keys)),
            total_weight: Arc::new(RwLock::new(total_weight)),
        }
    }
    
    /// 获取下一个可用的 API 密钥（使用平滑加权轮询算法）
    pub async fn get_next_key(&self) -> Option<ApiKey> {
        let mut keys = self.keys.write().await;
        
        // 更新所有密钥的可用状态
        self.update_keys_availability(&mut keys).await;
        
        // 使用平滑加权轮询算法选择密钥
        let selected_key = self.select_key_with_smooth_wrr(&mut keys).await;
        
        if let Some(selected) = selected_key {
            // 增加请求计数
            if let Some(key) = keys.iter_mut().find(|k| k.id == selected.id) {
                key.increment_requests();
            }
            Some(selected)
        } else {
            None
        }
    }
    
    /// 更新密钥的可用状态（内部方法，已持有写锁）
    async fn update_keys_availability(&self, keys: &mut Vec<UnifiedApiKey>) {
        let now = Utc::now();
        let mut total_weight_changed = false;
        
        for key in keys.iter_mut() {
            // 重置速率限制计数器
            if key.should_reset_rate_limit() {
                key.reset_rate_limit();
            }
            
            // 检查密钥是否应该重新激活
            if !key.runtime_state.is_active && key.runtime_state.failure_count < 3 {
                // 简单的恢复策略：5分钟后重试失败的密钥
                let elapsed = now.signed_duration_since(key.runtime_state.last_reset);
                if elapsed.num_minutes() >= 5 {
                    key.mark_success();
                    total_weight_changed = true;
                }
            }
        }
        
        // 如果总权重发生变化，更新缓存
        if total_weight_changed {
            let new_total_weight = keys.iter()
                .map(|k| k.scheduling_state.effective_weight)
                .sum();
            *self.total_weight.write().await = new_total_weight;
        }
    }
    
    /// 使用平滑加权轮询算法选择密钥（内部方法，已持有写锁）
    async fn select_key_with_smooth_wrr(&self, keys: &mut Vec<UnifiedApiKey>) -> Option<ApiKey> {
        // 过滤出可用的密钥
        let available_keys: Vec<usize> = keys.iter()
            .enumerate()
            .filter(|(_, key)| key.is_available())
            .map(|(i, _)| i)
            .collect();
        
        if available_keys.is_empty() {
            return None;
        }
        
        // 计算总有效权重
        let total_effective_weight: i32 = available_keys.iter()
            .map(|&i| keys[i].scheduling_state.effective_weight)
            .sum();
        
        if total_effective_weight <= 0 {
            return None;
        }
        
        // 平滑加权轮询算法
        // 1. 为每个可用密钥增加其有效权重到当前权重
        for &i in &available_keys {
            keys[i].scheduling_state.current_weight += keys[i].scheduling_state.effective_weight;
        }
        
        // 2. 选择当前权重最大的密钥
        let selected_index = available_keys.iter()
            .max_by_key(|&&i| keys[i].scheduling_state.current_weight)
            .copied()?;
        
        // 3. 将选中密钥的当前权重减去总有效权重
        keys[selected_index].scheduling_state.current_weight -= total_effective_weight;
        
        Some(keys[selected_index].to_api_key())
    }
    
    /// 标记密钥为失败状态
    pub async fn mark_key_failed(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            let old_effective_weight = key.scheduling_state.effective_weight;
            key.mark_failed();
            
            // 如果有效权重发生变化，更新总权重缓存
            if old_effective_weight != key.scheduling_state.effective_weight {
                let new_total_weight = keys.iter()
                    .map(|k| k.scheduling_state.effective_weight)
                    .sum();
                *self.total_weight.write().await = new_total_weight;
            }
        }
    }
    
    /// 标记密钥为成功状态
    pub async fn mark_key_success(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            let old_effective_weight = key.scheduling_state.effective_weight;
            key.mark_success();
            
            // 如果有效权重发生变化，更新总权重缓存
            if old_effective_weight != key.scheduling_state.effective_weight {
                let new_total_weight = keys.iter()
                    .map(|k| k.scheduling_state.effective_weight)
                    .sum();
                *self.total_weight.write().await = new_total_weight;
            }
        }
    }
    
    /// 更新密钥权重（原子操作）
    pub async fn update_key_weight(&self, key_id: &str, new_weight: u32) -> Result<(), String> {
        let mut keys = self.keys.write().await;
        
        if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            key.update_weight(new_weight);
            
            // 更新总权重缓存
            let new_total_weight = keys.iter()
                .map(|k| k.scheduling_state.effective_weight)
                .sum();
            *self.total_weight.write().await = new_total_weight;
            
            Ok(())
        } else {
            Err(format!("密钥 {} 不存在", key_id))
        }
    }
    
    /// 批量更新密钥权重（原子操作）
    #[allow(dead_code)]
    pub async fn batch_update_weights(&self, updates: &[(String, u32)]) -> Result<(), String> {
        let mut keys = self.keys.write().await;
        
        // 验证所有密钥是否存在
        for (key_id, _) in updates {
            if !keys.iter().any(|k| &k.id == key_id) {
                return Err(format!("密钥 {} 不存在", key_id));
            }
        }
        
        // 批量更新权重
        for (key_id, new_weight) in updates {
            if let Some(key) = keys.iter_mut().find(|k| &k.id == key_id) {
                key.update_weight(*new_weight);
            }
        }
        
        // 更新总权重缓存
        let new_total_weight = keys.iter()
            .map(|k| k.scheduling_state.effective_weight)
            .sum();
        *self.total_weight.write().await = new_total_weight;
        
        Ok(())
    }
    
    /// 获取所有密钥的状态（兼容性方法）
    pub async fn get_all_keys(&self) -> Vec<ApiKey> {
        let keys = self.keys.read().await;
        keys.iter().map(|k| k.to_api_key()).collect()
    }
    
    /// 获取负载均衡统计信息
    pub async fn get_stats(&self) -> LoadBalancingStats {
        let keys = self.keys.read().await;
        
        LoadBalancingStats {
            total_keys: keys.len(),
            active_keys: keys.iter().filter(|k| k.runtime_state.is_active).count(),
            total_weight: keys.iter().map(|k| k.weight).sum(),
            total_requests: keys.iter().map(|k| k.runtime_state.current_requests).sum(),
            failed_keys: keys.iter().filter(|k| k.runtime_state.failure_count >= 3).count(),
        }
    }
    
    /// 获取活跃密钥数量
    #[allow(dead_code)]
    pub async fn get_active_keys_count(&self) -> usize {
        let keys = self.keys.read().await;
        keys.iter().filter(|k| k.runtime_state.is_active).count()
    }
    
    /// 检查是否有可用密钥
    #[allow(dead_code)]
    pub async fn has_available_keys(&self) -> bool {
        let keys = self.keys.read().await;
        keys.iter().any(|k| k.is_available())
    }
}

// 移除 WeightStats 类型别名以避免与 weighted_round_robin 模块冲突
// 使用 LoadBalancingStats 作为统一的统计类型