use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};

use crate::ConfigState;
use crate::load_balancer::{
    KeyManager, WeightOptimizer, OptimizerConfig, OptimizationStrategy, 
    PerformanceMetric
};

/// 智能优化状态管理
#[derive(Clone)]
pub struct IntelligentOptimizationState {
    pub key_manager: Option<Arc<KeyManager>>,
    pub config_state: ConfigState,
    pub optimizer: Arc<RwLock<WeightOptimizer>>,
}

impl IntelligentOptimizationState {
    pub fn new(config_state: ConfigState) -> Self {
        let optimizer_config = OptimizerConfig::default();
        let optimizer = WeightOptimizer::new(optimizer_config);
        
        Self {
            key_manager: None,
            config_state,
            optimizer: Arc::new(RwLock::new(optimizer)),
        }
    }

    pub async fn set_key_manager(&self, _key_manager: Arc<KeyManager>) {
        // 这里可以后续扩展
    }

    pub async fn get_key_manager(&self) -> Option<Arc<KeyManager>> {
        self.key_manager.clone()
    }
}

/// 优化请求
#[derive(Debug, Deserialize)]
pub struct OptimizationRequest {
    pub strategy: OptimizationStrategy,
    pub dry_run: Option<bool>, // 是否仅预览不实际应用
}

/// 性能数据提交请求
#[derive(Debug, Deserialize)]
pub struct PerformanceDataRequest {
    pub key_id: String,
    pub metrics: Vec<PerformanceMetric>,
}

/// 优化器配置更新请求
#[derive(Debug, Deserialize)]
pub struct OptimizerConfigRequest {
    pub history_days: Option<u32>,
    pub min_samples: Option<usize>,
    pub response_time_weight: Option<f64>,
    pub success_rate_weight: Option<f64>,
    pub throughput_weight: Option<f64>,
    pub max_adjustment_percent: Option<f64>,
    pub sensitivity: Option<f64>,
}

/// 通用API响应结构
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

/// 获取优化建议
async fn get_optimization_recommendations_handler(
    request: OptimizationRequest,
    state: IntelligentOptimizationState,
) -> Result<warp::reply::Json, Rejection> {
    match state.get_key_manager().await {
        Some(key_manager) => {
            // 获取当前权重
            let weight_stats = key_manager.get_weight_stats().await;
            let mut current_weights = std::collections::HashMap::new();
            
            for distribution in &weight_stats.key_distributions {
                current_weights.insert(distribution.key_id.clone(), distribution.weight);
            }
            
            // 生成优化建议
            let optimizer = state.optimizer.read().await;
            let recommendations = optimizer
                .generate_recommendations(&current_weights, request.strategy)
                .await;
            
            let response = ApiResponse::success(recommendations);
            Ok(warp::reply::json(&response))
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 应用优化建议
async fn apply_optimization_handler(
    request: OptimizationRequest,
    state: IntelligentOptimizationState,
) -> Result<warp::reply::Json, Rejection> {
    if request.dry_run.unwrap_or(false) {
        // 如果是预览模式，只返回建议不实际应用
        return get_optimization_recommendations_handler(request, state).await;
    }
    
    match state.get_key_manager().await {
        Some(key_manager) => {
            // 获取当前配置
            let mut config = state.config_state.get_config().await;
            
            // 获取当前权重
            let weight_stats = key_manager.get_weight_stats().await;
            let mut current_weights = std::collections::HashMap::new();
            
            for distribution in &weight_stats.key_distributions {
                current_weights.insert(distribution.key_id.clone(), distribution.weight);
            }
            
            // 生成并应用优化建议
            let optimizer = state.optimizer.read().await;
            let recommendations = optimizer
                .generate_recommendations(&current_weights, request.strategy)
                .await;
            
            // 应用到配置
            for recommendation in &recommendations.recommendations {
                for api_key in &mut config.gemini.api_keys {
                    if api_key.id == recommendation.key_id {
                        api_key.weight = recommendation.recommended_weight;
                        break;
                    }
                }
            }
            
            // 保存配置
            if let Err(e) = state.config_state.update_config(config).await {
                let response = ApiResponse::<()>::error(format!("Failed to save config: {}", e));
                return Ok(warp::reply::json(&response));
            }
            
            let response = ApiResponse::success(recommendations);
            Ok(warp::reply::json(&response))
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 提交性能数据
async fn submit_performance_data_handler(
    request: PerformanceDataRequest,
    state: IntelligentOptimizationState,
) -> Result<impl Reply, Rejection> {
    let optimizer = state.optimizer.read().await;
    
    for metric in request.metrics {
        optimizer.record_performance(&request.key_id, metric).await;
    }
    
    let response = ApiResponse::success("Performance data recorded successfully");
    Ok(warp::reply::json(&response))
}

/// 获取性能评分
async fn get_performance_scores_handler(
    state: IntelligentOptimizationState,
) -> Result<impl Reply, Rejection> {
    match state.get_key_manager().await {
        Some(key_manager) => {
            let weight_stats = key_manager.get_weight_stats().await;
            let optimizer = state.optimizer.read().await;
            let mut scores = std::collections::HashMap::new();
            
            for distribution in &weight_stats.key_distributions {
                if let Some(score) = optimizer.calculate_performance_score(&distribution.key_id).await {
                    scores.insert(distribution.key_id.clone(), score);
                }
            }
            
            let response = ApiResponse::success(scores);
            Ok(warp::reply::json(&response))
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 更新优化器配置
async fn update_optimizer_config_handler(
    request: OptimizerConfigRequest,
    state: IntelligentOptimizationState,
) -> Result<impl Reply, Rejection> {
    let current_optimizer = state.optimizer.read().await;
    let mut new_config = current_optimizer.get_config().clone();
    
    // 更新配置
    if let Some(history_days) = request.history_days {
        new_config.history_days = history_days;
    }
    if let Some(min_samples) = request.min_samples {
        new_config.min_samples = min_samples;
    }
    if let Some(response_time_weight) = request.response_time_weight {
        new_config.response_time_weight = response_time_weight;
    }
    if let Some(success_rate_weight) = request.success_rate_weight {
        new_config.success_rate_weight = success_rate_weight;
    }
    if let Some(throughput_weight) = request.throughput_weight {
        new_config.throughput_weight = throughput_weight;
    }
    if let Some(max_adjustment_percent) = request.max_adjustment_percent {
        new_config.max_adjustment_percent = max_adjustment_percent;
    }
    if let Some(sensitivity) = request.sensitivity {
        new_config.sensitivity = sensitivity;
    }
    
    drop(current_optimizer); // 释放读锁
    
    // 创建新的优化器
    let new_optimizer = WeightOptimizer::new(new_config);
    *state.optimizer.write().await = new_optimizer;
    
    let response = ApiResponse::success("Optimizer configuration updated successfully");
    Ok(warp::reply::json(&response))
}

/// 获取优化器配置
async fn get_optimizer_config_handler(
    state: IntelligentOptimizationState,
) -> Result<impl Reply, Rejection> {
    let optimizer = state.optimizer.read().await;
    let config = optimizer.get_config();
    
    let response = ApiResponse::success(config.clone());
    Ok(warp::reply::json(&response))
}

/// 获取优化策略列表
async fn get_optimization_strategies_handler() -> Result<impl Reply, Rejection> {
    let strategies = vec![
        OptimizationStrategy::ResponseTimeOptimized,
        OptimizationStrategy::ReliabilityOptimized,
        OptimizationStrategy::ThroughputOptimized,
        OptimizationStrategy::Balanced,
        OptimizationStrategy::Conservative,
        OptimizationStrategy::Aggressive,
    ];
    
    let response = ApiResponse::success(strategies);
    Ok(warp::reply::json(&response))
}

/// 智能优化 API 路由
pub fn intelligent_optimization_routes(
    state: IntelligentOptimizationState,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let optimization_state = warp::any().map(move || state.clone());

    // GET /api/optimization/recommendations - 获取优化建议
    let get_recommendations = warp::path!("api" / "optimization" / "recommendations")
        .and(warp::post())
        .and(warp::body::json())
        .and(optimization_state.clone())
        .and_then(get_optimization_recommendations_handler);

    // POST /api/optimization/apply - 应用优化建议
    let apply_optimization = warp::path!("api" / "optimization" / "apply")
        .and(warp::post())
        .and(warp::body::json())
        .and(optimization_state.clone())
        .and_then(apply_optimization_handler);

    // POST /api/optimization/performance - 提交性能数据
    let submit_performance = warp::path!("api" / "optimization" / "performance")
        .and(warp::post())
        .and(warp::body::json())
        .and(optimization_state.clone())
        .and_then(submit_performance_data_handler);

    // GET /api/optimization/scores - 获取性能评分
    let get_scores = warp::path!("api" / "optimization" / "scores")
        .and(warp::get())
        .and(optimization_state.clone())
        .and_then(get_performance_scores_handler);

    // PUT /api/optimization/config - 更新优化器配置
    let update_config = warp::path!("api" / "optimization" / "config")
        .and(warp::put())
        .and(warp::body::json())
        .and(optimization_state.clone())
        .and_then(update_optimizer_config_handler);

    // GET /api/optimization/config - 获取优化器配置
    let get_config = warp::path!("api" / "optimization" / "config")
        .and(warp::get())
        .and(optimization_state.clone())
        .and_then(get_optimizer_config_handler);

    // GET /api/optimization/strategies - 获取优化策略列表
    let get_strategies = warp::path!("api" / "optimization" / "strategies")
        .and(warp::get())
        .and_then(get_optimization_strategies_handler);

    get_recommendations
        .or(apply_optimization)
        .or(submit_performance)
        .or(get_scores)
        .or(update_config)
        .or(get_config)
        .or(get_strategies)
}