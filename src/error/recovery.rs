// src/error/recovery.rs
//! 错误恢复机制
//! 
//! 提供自动错误恢复、重试逻辑和降级策略

use super::{GeminiProxyError, ErrorSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

/// 恢复策略类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// 重试策略
    Retry {
        max_attempts: u32,
        backoff: BackoffStrategy,
        retry_conditions: Vec<RetryCondition>,
    },
    /// 熔断器策略
    CircuitBreaker {
        failure_threshold: u32,
        recovery_timeout: Duration,
        half_open_max_calls: u32,
    },
    /// 降级策略
    Fallback {
        fallback_action: FallbackAction,
        degradation_level: DegradationLevel,
    },
    /// 自愈策略
    SelfHealing {
        healing_actions: Vec<HealingAction>,
        validation_check: Option<String>,
    },
    /// 无恢复
    None,
}

/// 退避策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// 固定延迟
    Fixed(Duration),
    /// 线性退避
    Linear { initial: Duration, increment: Duration },
    /// 指数退避
    Exponential { initial: Duration, multiplier: f64, max_delay: Duration },
    /// 自定义退避
    Custom(Vec<Duration>),
}

/// 重试条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    /// 按错误类型重试
    ErrorType(String),
    /// 按错误严重程度重试
    Severity(ErrorSeverity),
    /// 按组件重试
    Component(String),
    /// 按自定义条件重试
    Custom(String),
}

/// 降级操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackAction {
    /// 返回缓存的结果
    ReturnCached,
    /// 使用备用服务
    UseBackupService(String),
    /// 返回默认值
    ReturnDefault(String),
    /// 跳过操作
    Skip,
    /// 自定义处理
    Custom(String),
}

/// 降级级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DegradationLevel {
    /// 轻微降级
    Minor,
    /// 部分降级
    Partial,
    /// 严重降级
    Major,
    /// 完全停用
    Complete,
}

/// 自愈操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealingAction {
    /// 重启组件
    RestartComponent(String),
    /// 清理缓存
    ClearCache,
    /// 重新加载配置
    ReloadConfig,
    /// 重置连接
    ResetConnections,
    /// 回收资源
    GarbageCollect,
    /// 自定义操作
    Custom(String),
}

/// 熔断器状态
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    /// 关闭状态（正常工作）
    Closed,
    /// 开启状态（阻止请求）
    Open,
    /// 半开状态（尝试恢复）
    HalfOpen,
}

/// 恢复尝试记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAttempt {
    pub attempt_id: String,
    pub error_id: String,
    pub strategy: RecoveryStrategy,
    pub attempt_number: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub success: Option<bool>,
    pub result: Option<String>,
    pub next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 恢复统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecoveryStats {
    pub total_recovery_attempts: u64,
    pub successful_recoveries: u64,
    pub failed_recoveries: u64,
    pub recovery_rate: f64,
    pub average_recovery_time_ms: f64,
    pub strategies_used: HashMap<String, u64>,
    pub circuit_breaker_trips: u64,
    pub fallback_activations: u64,
}

/// 错误恢复管理器
pub struct ErrorRecoveryManager {
    /// 恢复策略映射
    recovery_strategies: Arc<RwLock<HashMap<String, RecoveryStrategy>>>,
    /// 活跃的恢复尝试
    active_attempts: Arc<RwLock<HashMap<String, RecoveryAttempt>>>,
    /// 熔断器状态
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreakerInfo>>>,
    /// 恢复统计
    stats: Arc<RwLock<RecoveryStats>>,
    /// 配置
    config: RecoveryConfig,
}

/// 熔断器信息
#[derive(Debug, Clone)]
struct CircuitBreakerInfo {
    state: CircuitBreakerState,
    failure_count: u32,
    last_failure_time: chrono::DateTime<chrono::Utc>,
    next_attempt_time: chrono::DateTime<chrono::Utc>,
    half_open_calls: u32,
}

/// 恢复配置
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// 启用错误恢复
    pub enabled: bool,
    /// 最大并发恢复尝试数
    pub max_concurrent_attempts: u32,
    /// 默认恢复策略
    pub default_strategy: RecoveryStrategy,
    /// 恢复历史保留时间（小时）
    pub history_retention_hours: u64,
    /// 启用自动清理
    pub auto_cleanup_enabled: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_attempts: 10,
            default_strategy: RecoveryStrategy::Retry {
                max_attempts: 3,
                backoff: BackoffStrategy::Exponential {
                    initial: Duration::from_millis(100),
                    multiplier: 2.0,
                    max_delay: Duration::from_secs(10),
                },
                retry_conditions: vec![
                    RetryCondition::Severity(ErrorSeverity::Error),
                    RetryCondition::ErrorType("Network".to_string()),
                ],
            },
            history_retention_hours: 24,
            auto_cleanup_enabled: true,
        }
    }
}

impl ErrorRecoveryManager {
    /// 创建恢复管理器
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            recovery_strategies: Arc::new(RwLock::new(HashMap::new())),
            active_attempts: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RecoveryStats::default())),
            config,
        }
    }

    /// 注册恢复策略
    pub async fn register_strategy(&self, key: String, strategy: RecoveryStrategy) {
        let mut strategies = self.recovery_strategies.write().await;
        strategies.insert(key, strategy);
    }

    /// 尝试恢复错误
    pub async fn attempt_recovery<F, Fut, T>(&self, error: &GeminiProxyError, operation: F) -> Result<T, GeminiProxyError>
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<T, GeminiProxyError>> + Send,
        T: Send + 'static + Default,
    {
        if !self.config.enabled {
            return operation().await;
        }

        // 获取恢复策略
        let strategy = self.get_recovery_strategy_for_error(error).await;
        
        match strategy {
            RecoveryStrategy::Retry { max_attempts, backoff, retry_conditions } => {
                self.execute_retry_strategy(error, operation, max_attempts, backoff, retry_conditions).await
            }
            RecoveryStrategy::CircuitBreaker { failure_threshold, recovery_timeout, half_open_max_calls } => {
                self.execute_circuit_breaker_strategy(error, operation, failure_threshold, recovery_timeout, half_open_max_calls).await
            }
            RecoveryStrategy::Fallback { fallback_action, degradation_level } => {
                self.execute_fallback_strategy(error, fallback_action, degradation_level).await
            }
            RecoveryStrategy::SelfHealing { healing_actions, validation_check } => {
                self.execute_self_healing_strategy(error, operation, healing_actions, validation_check).await
            }
            RecoveryStrategy::None => {
                operation().await
            }
        }
    }

    /// 检查熔断器状态
    pub async fn check_circuit_breaker(&self, component: &str) -> bool {
        let circuit_breakers = self.circuit_breakers.read().await;
        if let Some(cb_info) = circuit_breakers.get(component) {
            match cb_info.state {
                CircuitBreakerState::Open => {
                    // 检查是否可以尝试半开
                    chrono::Utc::now() >= cb_info.next_attempt_time
                }
                CircuitBreakerState::HalfOpen => {
                    cb_info.half_open_calls < 3 // 限制半开状态下的调用次数
                }
                CircuitBreakerState::Closed => true,
            }
        } else {
            true // 如果没有熔断器信息，允许通过
        }
    }

    /// 报告操作结果
    pub async fn report_operation_result(&self, component: &str, success: bool) {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        let now = chrono::Utc::now();
        
        let cb_info = circuit_breakers.entry(component.to_string()).or_insert(CircuitBreakerInfo {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            last_failure_time: now,
            next_attempt_time: now,
            half_open_calls: 0,
        });

        if success {
            // 成功操作
            match cb_info.state {
                CircuitBreakerState::HalfOpen => {
                    // 半开状态下成功，可以关闭熔断器
                    cb_info.state = CircuitBreakerState::Closed;
                    cb_info.failure_count = 0;
                    cb_info.half_open_calls = 0;
                }
                CircuitBreakerState::Closed => {
                    // 保持关闭状态，重置失败计数
                    cb_info.failure_count = 0;
                }
                _ => {}
            }
        } else {
            // 失败操作
            cb_info.failure_count += 1;
            cb_info.last_failure_time = now;
            
            match cb_info.state {
                CircuitBreakerState::Closed => {
                    if cb_info.failure_count >= 5 { // 默认失败阈值
                        cb_info.state = CircuitBreakerState::Open;
                        cb_info.next_attempt_time = now + chrono::Duration::seconds(60); // 60秒后尝试半开
                        self.increment_stat("circuit_breaker_trips").await;
                    }
                }
                CircuitBreakerState::HalfOpen => {
                    // 半开状态下失败，重新开启熔断器
                    cb_info.state = CircuitBreakerState::Open;
                    cb_info.next_attempt_time = now + chrono::Duration::seconds(60);
                    cb_info.half_open_calls = 0;
                }
                _ => {}
            }
        }
    }

    /// 获取恢复统计
    pub async fn get_statistics(&self) -> RecoveryStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        
        // 计算恢复率
        if result.total_recovery_attempts > 0 {
            result.recovery_rate = result.successful_recoveries as f64 / result.total_recovery_attempts as f64;
        }
        
        result
    }

    /// 清理过期的恢复记录
    pub async fn cleanup_expired_records(&self) -> Result<(), String> {
        if !self.config.auto_cleanup_enabled {
            return Ok(());
        }

        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(self.config.history_retention_hours as i64);
        
        let mut attempts = self.active_attempts.write().await;
        attempts.retain(|_, attempt| {
            if let Some(completed_at) = attempt.completed_at {
                completed_at > cutoff_time
            } else {
                attempt.started_at > cutoff_time
            }
        });

        Ok(())
    }

    // 私有方法实现

    async fn get_recovery_strategy_for_error(&self, error: &GeminiProxyError) -> RecoveryStrategy {
        let strategies = self.recovery_strategies.read().await;
        
        // 按优先级查找策略
        let search_keys = vec![
            format!("{}:{}", error.error_type_name(), error.get_context().component),
            error.error_type_name().to_string(),
            error.get_context().component.clone(),
            "default".to_string(),
        ];
        
        for key in search_keys {
            if let Some(strategy) = strategies.get(&key) {
                return strategy.clone();
            }
        }
        
        self.config.default_strategy.clone()
    }

    async fn execute_retry_strategy<F, Fut, T>(
        &self,
        error: &GeminiProxyError,
        operation: F,
        max_attempts: u32,
        backoff: BackoffStrategy,
        retry_conditions: Vec<RetryCondition>,
    ) -> Result<T, GeminiProxyError>
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = Result<T, GeminiProxyError>> + Send,
        T: Send,
    {
        // 检查是否应该重试
        if !self.should_retry(error, &retry_conditions) {
            return operation().await;
        }

        let attempt_id = uuid::Uuid::new_v4().to_string();
        let mut attempt = RecoveryAttempt {
            attempt_id: attempt_id.clone(),
            error_id: error.get_context().error_id.clone(),
            strategy: RecoveryStrategy::Retry { max_attempts, backoff: backoff.clone(), retry_conditions },
            attempt_number: 0,
            started_at: chrono::Utc::now(),
            completed_at: None,
            success: None,
            result: None,
            next_retry_at: None,
        };

        // 记录尝试
        {
            let mut attempts = self.active_attempts.write().await;
            attempts.insert(attempt_id.clone(), attempt.clone());
        }

        let mut last_error = None;
        
        for attempt_num in 1..=max_attempts {
            attempt.attempt_number = attempt_num;
            
            match operation().await {
                Ok(result) => {
                    // 成功，记录并返回
                    attempt.success = Some(true);
                    attempt.completed_at = Some(chrono::Utc::now());
                    attempt.result = Some("成功".to_string());
                    
                    self.update_attempt(&attempt_id, attempt).await;
                    self.increment_stat("successful_recoveries").await;
                    
                    return Ok(result);
                }
                Err(err) => {
                    last_error = Some(err);
                    
                    if attempt_num < max_attempts {
                        // 计算延迟时间
                        let delay = self.calculate_backoff_delay(&backoff, attempt_num - 1);
                        attempt.next_retry_at = Some(chrono::Utc::now() + chrono::Duration::from_std(delay).unwrap());
                        
                        self.update_attempt(&attempt_id, attempt.clone()).await;
                        
                        // 等待后重试
                        sleep(delay).await;
                    }
                }
            }
        }

        // 所有重试都失败了
        attempt.success = Some(false);
        attempt.completed_at = Some(chrono::Utc::now());
        attempt.result = Some("重试失败".to_string());
        
        self.update_attempt(&attempt_id, attempt).await;
        self.increment_stat("failed_recoveries").await;
        
        Err(last_error.unwrap())
    }

    async fn execute_circuit_breaker_strategy<F, Fut, T>(
        &self,
        _error: &GeminiProxyError,
        operation: F,
        _failure_threshold: u32,
        _recovery_timeout: Duration,
        _half_open_max_calls: u32,
    ) -> Result<T, GeminiProxyError>
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = Result<T, GeminiProxyError>> + Send,
        T: Send,
    {
        // 简化实现，直接执行操作
        // 实际实现应该检查熔断器状态
        operation().await
    }

    async fn execute_fallback_strategy<T>(
        &self,
        error: &GeminiProxyError,
        fallback_action: FallbackAction,
        _degradation_level: DegradationLevel,
    ) -> Result<T, GeminiProxyError>
    where
        T: Send + Default,
    {
        self.increment_stat("fallback_activations").await;
        
        match fallback_action {
            FallbackAction::ReturnDefault(_) => {
                Ok(T::default())
            }
            FallbackAction::Skip => {
                Ok(T::default())
            }
            _ => {
                // 其他降级操作暂时返回错误
                Err(error.clone())
            }
        }
    }

    async fn execute_self_healing_strategy<F, Fut, T>(
        &self,
        _error: &GeminiProxyError,
        operation: F,
        healing_actions: Vec<HealingAction>,
        _validation_check: Option<String>,
    ) -> Result<T, GeminiProxyError>
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = Result<T, GeminiProxyError>> + Send,
        T: Send,
    {
        // 执行自愈操作
        for action in healing_actions {
            self.execute_healing_action(action).await;
        }
        
        // 重新尝试操作
        operation().await
    }

    async fn execute_healing_action(&self, action: HealingAction) {
        match action {
            HealingAction::ClearCache => {
                // 实现缓存清理逻辑
                tracing::info!("执行自愈操作：清理缓存");
            }
            HealingAction::ReloadConfig => {
                // 实现配置重加载逻辑
                tracing::info!("执行自愈操作：重新加载配置");
            }
            HealingAction::ResetConnections => {
                // 实现连接重置逻辑
                tracing::info!("执行自愈操作：重置连接");
            }
            HealingAction::GarbageCollect => {
                // 实现垃圾回收逻辑
                tracing::info!("执行自愈操作：垃圾回收");
            }
            HealingAction::RestartComponent(component) => {
                tracing::info!("执行自愈操作：重启组件 {}", component);
            }
            HealingAction::Custom(action) => {
                tracing::info!("执行自定义自愈操作：{}", action);
            }
        }
    }

    fn should_retry(&self, error: &GeminiProxyError, conditions: &[RetryCondition]) -> bool {
        if conditions.is_empty() {
            return true; // 如果没有条件，默认重试
        }
        
        for condition in conditions {
            match condition {
                RetryCondition::ErrorType(error_type) => {
                    if error.error_type_name() == error_type {
                        return true;
                    }
                }
                RetryCondition::Severity(severity) => {
                    if error.get_context().severity == *severity {
                        return true;
                    }
                }
                RetryCondition::Component(component) => {
                    if error.get_context().component == *component {
                        return true;
                    }
                }
                RetryCondition::Custom(_) => {
                    // 自定义条件的实现
                    return true;
                }
            }
        }
        
        false
    }

    fn calculate_backoff_delay(&self, strategy: &BackoffStrategy, attempt: u32) -> Duration {
        match strategy {
            BackoffStrategy::Fixed(delay) => *delay,
            BackoffStrategy::Linear { initial, increment } => {
                *initial + *increment * attempt
            }
            BackoffStrategy::Exponential { initial, multiplier, max_delay } => {
                let delay = initial.as_millis() as f64 * multiplier.powi(attempt as i32);
                Duration::from_millis(delay as u64).min(*max_delay)
            }
            BackoffStrategy::Custom(delays) => {
                if attempt < delays.len() as u32 {
                    delays[attempt as usize]
                } else {
                    delays.last().copied().unwrap_or(Duration::from_secs(1))
                }
            }
        }
    }

    async fn update_attempt(&self, attempt_id: &str, attempt: RecoveryAttempt) {
        let mut attempts = self.active_attempts.write().await;
        attempts.insert(attempt_id.to_string(), attempt);
    }

    async fn increment_stat(&self, stat_name: &str) {
        let mut stats = self.stats.write().await;
        stats.total_recovery_attempts += 1;
        
        match stat_name {
            "successful_recoveries" => stats.successful_recoveries += 1,
            "failed_recoveries" => stats.failed_recoveries += 1,
            "circuit_breaker_trips" => stats.circuit_breaker_trips += 1,
            "fallback_activations" => stats.fallback_activations += 1,
            _ => {}
        }
    }
}

/// 创建默认的恢复管理器
pub fn create_default_recovery_manager() -> ErrorRecoveryManager {
    ErrorRecoveryManager::new(RecoveryConfig::default())
}

/// 创建生产环境的恢复管理器
pub fn create_production_recovery_manager() -> ErrorRecoveryManager {
    let config = RecoveryConfig {
        enabled: true,
        max_concurrent_attempts: 20,
        default_strategy: RecoveryStrategy::Retry {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                initial: Duration::from_millis(200),
                multiplier: 2.0,
                max_delay: Duration::from_secs(30),
            },
            retry_conditions: vec![
                RetryCondition::ErrorType("Network".to_string()),
                RetryCondition::ErrorType("LoadBalancer".to_string()),
                RetryCondition::Severity(ErrorSeverity::Error),
            ],
        },
        history_retention_hours: 72, // 3天
        auto_cleanup_enabled: true,
    };
    
    ErrorRecoveryManager::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GeminiProxyError;

    #[tokio::test]
    async fn test_retry_strategy() {
        let manager = create_default_recovery_manager();
        
        use std::sync::atomic::{AtomicU32, Ordering};
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        
        let operation = move || {
            let count = call_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            async move {
                if count < 3 {
                    Err(GeminiProxyError::network("网络错误"))
                } else {
                    Ok("成功".to_string())
                }
            }
        };
        
        let error = GeminiProxyError::network("初始错误");
        let result = manager.attempt_recovery(&error, operation).await;
        
        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let manager = create_default_recovery_manager();
        
        // 模拟多次失败触发熔断器
        for _ in 0..6 {
            manager.report_operation_result("test_component", false).await;
        }
        
        // 检查熔断器状态
        let can_proceed = manager.check_circuit_breaker("test_component").await;
        assert!(!can_proceed);
    }

    #[tokio::test]
    async fn test_backoff_calculation() {
        let manager = create_default_recovery_manager();
        
        let strategy = BackoffStrategy::Exponential {
            initial: Duration::from_millis(100),
            multiplier: 2.0,
            max_delay: Duration::from_secs(10),
        };
        
        let delay1 = manager.calculate_backoff_delay(&strategy, 0);
        let delay2 = manager.calculate_backoff_delay(&strategy, 1);
        let delay3 = manager.calculate_backoff_delay(&strategy, 2);
        
        assert_eq!(delay1, Duration::from_millis(100));
        assert_eq!(delay2, Duration::from_millis(200));
        assert_eq!(delay3, Duration::from_millis(400));
    }
}