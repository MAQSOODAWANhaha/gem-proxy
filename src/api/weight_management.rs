// src/api/weight_management.rs
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};
use crate::load_balancer::UnifiedKeyManager;
use crate::api::config::{ApiResponse, ConfigState};

/// 权重更新请求
#[derive(Debug, Deserialize)]
pub struct UpdateWeightRequest {
    pub weight: u32,
}

/// 批量权重更新请求
#[derive(Debug, Deserialize)]
pub struct BatchUpdateWeightRequest {
    pub updates: Vec<WeightUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct WeightUpdate {
    pub key_id: String,
    pub weight: u32,
}

/// 权重分配信息
#[derive(Debug, Serialize)]
pub struct WeightDistribution {
    pub key_id: String,
    pub weight: u32,
    pub percentage: f64,
    pub is_active: bool,
    pub current_requests: u32,
    pub max_requests_per_minute: u32,
    pub failure_count: u32,
}

/// 权重统计响应
#[derive(Debug, Serialize)]
pub struct WeightStatsResponse {
    pub total_weight: u32,
    pub active_keys_count: usize,
    pub total_keys_count: usize,
    pub distributions: Vec<WeightDistribution>,
    pub load_balance_effectiveness: f64, // 负载均衡有效性评分 (0-100)
}

/// 权重优化建议
#[derive(Debug, Serialize)]
pub struct WeightOptimizationSuggestion {
    pub key_id: String,
    pub current_weight: u32,
    pub suggested_weight: u32,
    pub reason: String,
    pub impact: String,
}

#[derive(Debug, Serialize)]
pub struct WeightOptimizationResponse {
    pub suggestions: Vec<WeightOptimizationSuggestion>,
    pub overall_score: f64,
    pub optimization_needed: bool,
}

/// 权重管理状态
#[derive(Clone)]
pub struct WeightManagementState {
    config_state: ConfigState,
    key_manager: Arc<RwLock<Option<Arc<UnifiedKeyManager>>>>,
}

impl WeightManagementState {
    pub fn new(config_state: ConfigState) -> Self {
        Self {
            config_state,
            key_manager: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_key_manager(&self, key_manager: Arc<UnifiedKeyManager>) {
        *self.key_manager.write().await = Some(key_manager);
    }

    pub async fn get_key_manager(&self) -> Option<Arc<UnifiedKeyManager>> {
        self.key_manager.read().await.clone()
    }
}

/// 权重管理 API 路由
pub fn weight_management_routes(
    state: WeightManagementState,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let weight_state = warp::any().map(move || state.clone());

    // GET /api/weights/stats - 获取权重统计
    let get_stats = warp::path!("api" / "weights" / "stats")
        .and(warp::get())
        .and(weight_state.clone())
        .and_then(get_weight_stats_handler);

    // PUT /api/weights/{key_id} - 更新单个密钥权重
    let update_weight = warp::path!("api" / "weights" / String)
        .and(warp::put())
        .and(warp::body::json())
        .and(weight_state.clone())
        .and_then(update_weight_handler);

    // POST /api/weights/batch - 批量更新权重
    let batch_update = warp::path!("api" / "weights" / "batch")
        .and(warp::post())
        .and(warp::body::json())
        .and(weight_state.clone())
        .and_then(batch_update_weights_handler);

    // POST /api/weights/rebalance - 智能权重重新平衡
    let rebalance = warp::path!("api" / "weights" / "rebalance")
        .and(warp::post())
        .and(weight_state.clone())
        .and_then(rebalance_weights_handler);

    // GET /api/weights/optimize - 获取权重优化建议
    let optimize = warp::path!("api" / "weights" / "optimize")
        .and(warp::get())
        .and(weight_state.clone())
        .and_then(get_optimization_suggestions_handler);

    // GET /api/weights/distribution - 获取权重分配详情
    let distribution = warp::path!("api" / "weights" / "distribution")
        .and(warp::get())
        .and(weight_state.clone())
        .and_then(get_weight_distribution_handler);

    get_stats
        .or(update_weight)
        .or(batch_update)
        .or(rebalance)
        .or(optimize)
        .or(distribution)
}

// API 处理函数

/// 获取权重统计
async fn get_weight_stats_handler(state: WeightManagementState) -> Result<impl Reply, Rejection> {
    match state.get_key_manager().await {
        Some(key_manager) => {
            let stats = key_manager.get_stats().await;
            let all_keys = key_manager.get_all_keys().await;
            
            let distributions: Vec<WeightDistribution> = all_keys
                .into_iter()
                .map(|key| {
                    let percentage = if stats.total_weight > 0 && key.is_active {
                        (key.weight as f64 / stats.total_weight as f64) * 100.0
                    } else {
                        0.0
                    };

                    WeightDistribution {
                        key_id: key.id,
                        weight: key.weight,
                        percentage,
                        is_active: key.is_active,
                        current_requests: key.current_requests,
                        max_requests_per_minute: key.max_requests_per_minute,
                        failure_count: key.failure_count,
                    }
                })
                .collect();

            let effectiveness = calculate_load_balance_effectiveness(&distributions);

            let response = WeightStatsResponse {
                total_weight: stats.total_weight,
                active_keys_count: stats.active_keys,
                total_keys_count: distributions.len(),
                distributions,
                load_balance_effectiveness: effectiveness,
            };

            Ok(warp::reply::json(&ApiResponse::success(response)))
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 更新单个密钥权重
async fn update_weight_handler(
    key_id: String,
    request: UpdateWeightRequest,
    state: WeightManagementState,
) -> Result<impl Reply, Rejection> {
    match state.get_key_manager().await {
        Some(key_manager) => {
            match key_manager.update_key_weight(&key_id, request.weight).await {
                Ok(()) => {
                    // 同时更新配置文件
                    if let Err(e) = update_config_weight(&state.config_state, &key_id, request.weight).await {
                        tracing::warn!("Failed to update config file: {}", e);
                    }

                    let response = ApiResponse::success(());
                    Ok(warp::reply::json(&response))
                }
                Err(e) => {
                    let response = ApiResponse::<()>::error(e);
                    Ok(warp::reply::json(&response))
                }
            }
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 批量更新权重
async fn batch_update_weights_handler(
    request: BatchUpdateWeightRequest,
    state: WeightManagementState,
) -> Result<impl Reply, Rejection> {
    match state.get_key_manager().await {
        Some(key_manager) => {
            let mut updated_count = 0;
            let mut errors = Vec::new();

            for update in request.updates {
                match key_manager.update_key_weight(&update.key_id, update.weight).await {
                    Ok(()) => {
                        updated_count += 1;
                        
                        // 同时更新配置文件
                        if let Err(e) = update_config_weight(&state.config_state, &update.key_id, update.weight).await {
                            tracing::warn!("Failed to update config file for key {}: {}", update.key_id, e);
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Failed to update key '{}': {}", update.key_id, e));
                    }
                }
            }

            if errors.is_empty() {
                let response = ApiResponse::success(format!("Updated {} keys", updated_count));
                Ok(warp::reply::json(&response))
            } else {
                let response = ApiResponse::<()>::error(format!(
                    "Updated {} keys, errors: {}", 
                    updated_count, 
                    errors.join(", ")
                ));
                Ok(warp::reply::json(&response))
            }
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 智能权重重新平衡
async fn rebalance_weights_handler(state: WeightManagementState) -> Result<impl Reply, Rejection> {
    match state.get_key_manager().await {
        Some(key_manager) => {
            let all_keys = key_manager.get_all_keys().await;
            let active_keys: Vec<_> = all_keys.iter().filter(|k| k.is_active).collect();

            if active_keys.is_empty() {
                let response = ApiResponse::<()>::error("No active keys to rebalance".to_string());
                return Ok(warp::reply::json(&response));
            }

            // 简单的平衡策略：给所有活跃密钥相等的权重
            let equal_weight = 100;
            let mut updated_count = 0;

            for key in active_keys {
                if let Ok(()) = key_manager.update_key_weight(&key.id, equal_weight).await {
                    updated_count += 1;
                    
                    // 同时更新配置文件
                    if let Err(e) = update_config_weight(&state.config_state, &key.id, equal_weight).await {
                        tracing::warn!("Failed to update config file for key {}: {}", key.id, e);
                    }
                }
            }

            let response = ApiResponse::success(format!("Rebalanced {} keys with equal weight ({})", updated_count, equal_weight));
            Ok(warp::reply::json(&response))
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 获取权重优化建议
async fn get_optimization_suggestions_handler(
    state: WeightManagementState,
) -> Result<impl Reply, Rejection> {
    match state.get_key_manager().await {
        Some(key_manager) => {
            let all_keys = key_manager.get_all_keys().await;
            let mut suggestions = Vec::new();
            let mut total_score: f64 = 100.0;
            let mut optimization_needed = false;

            for key in &all_keys {
                if !key.is_active && key.weight > 0 {
                    suggestions.push(WeightOptimizationSuggestion {
                        key_id: key.id.clone(),
                        current_weight: key.weight,
                        suggested_weight: 0,
                        reason: format!("Key is inactive (failure count: {})", key.failure_count),
                        impact: "Remove from load balancing".to_string(),
                    });
                    total_score -= 10.0;
                    optimization_needed = true;
                } else if key.is_active {
                    // 检查使用率
                    let usage_rate = key.current_requests as f64 / key.max_requests_per_minute as f64;
                    
                    if usage_rate > 0.8 {
                        let suggested_weight = (key.weight as f64 * 1.5) as u32;
                        suggestions.push(WeightOptimizationSuggestion {
                            key_id: key.id.clone(),
                            current_weight: key.weight,
                            suggested_weight,
                            reason: format!("High usage rate: {:.1}%", usage_rate * 100.0),
                            impact: "Increase weight to distribute load".to_string(),
                        });
                        total_score -= 5.0;
                        optimization_needed = true;
                    } else if usage_rate < 0.2 && key.weight > 50 {
                        let suggested_weight = (key.weight as f64 * 0.7) as u32;
                        suggestions.push(WeightOptimizationSuggestion {
                            key_id: key.id.clone(),
                            current_weight: key.weight,
                            suggested_weight,
                            reason: format!("Low usage rate: {:.1}%", usage_rate * 100.0),
                            impact: "Decrease weight to optimize resource usage".to_string(),
                        });
                        total_score -= 3.0;
                        optimization_needed = true;
                    }
                }
            }

            let response = WeightOptimizationResponse {
                suggestions,
                overall_score: total_score.max(0.0),
                optimization_needed,
            };

            Ok(warp::reply::json(&ApiResponse::success(response)))
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 获取权重分配详情
async fn get_weight_distribution_handler(
    state: WeightManagementState,
) -> Result<impl Reply, Rejection> {
    match state.get_key_manager().await {
        Some(key_manager) => {
            let stats = key_manager.get_stats().await;
            let response = ApiResponse::success(stats);
            Ok(warp::reply::json(&response))
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

// 辅助函数

/// 计算负载均衡有效性评分
fn calculate_load_balance_effectiveness(distributions: &[WeightDistribution]) -> f64 {
    let active_distributions: Vec<_> = distributions
        .iter()
        .filter(|d| d.is_active && d.weight > 0)
        .collect();

    if active_distributions.len() < 2 {
        return if active_distributions.len() == 1 { 50.0 } else { 0.0 };
    }

    // 计算权重分配的均匀性
    let weights: Vec<f64> = active_distributions.iter().map(|d| d.percentage).collect();
    let mean = weights.iter().sum::<f64>() / weights.len() as f64;
    let variance = weights.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / weights.len() as f64;
    let std_dev = variance.sqrt();

    // 标准差越小，分配越均匀，评分越高
    let uniformity_score = (100.0 - std_dev.min(50.0)).max(0.0);

    // 考虑失败密钥的影响
    let failure_penalty = active_distributions
        .iter()
        .map(|d| d.failure_count as f64 * 2.0)
        .sum::<f64>();

    (uniformity_score - failure_penalty).max(0.0).min(100.0)
}

/// 更新配置文件中的权重
async fn update_config_weight(
    config_state: &ConfigState,
    key_id: &str,
    new_weight: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = config_state.get_config().await;
    
    if let Some(api_key) = config.gemini.api_keys.iter_mut().find(|k| k.id == key_id) {
        api_key.weight = new_weight;
        config_state.update_config(config).await?;
    } else {
        return Err(format!("API key '{}' not found in config", key_id).into());
    }

    Ok(())
}