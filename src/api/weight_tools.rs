use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};

use crate::ConfigState;
use crate::load_balancer::{
    WeightManagementToolkit, WeightPreset,
    PerformanceMetrics, ToolkitConfig, WeightAuditSystem, AuditConfig
};
// 移除未使用的导入：WeightAnalysis, HealthCheckResult

/// 权重工具状态管理
#[derive(Clone)]
pub struct WeightToolsState {
    pub toolkit: Arc<RwLock<WeightManagementToolkit>>,
    pub config_state: ConfigState,
}

impl WeightToolsState {
    pub fn new(config_state: ConfigState) -> Self {
        let audit_system = Arc::new(RwLock::new(
            WeightAuditSystem::new(AuditConfig::default())
        ));
        let toolkit_config = ToolkitConfig::default();
        let toolkit = WeightManagementToolkit::new(audit_system, toolkit_config);
        
        Self {
            toolkit: Arc::new(RwLock::new(toolkit)),
            config_state,
        }
    }
}

/// 创建预设请求
#[derive(Debug, Deserialize)]
pub struct CreatePresetRequest {
    pub name: String,
    pub description: String,
    pub weights: HashMap<String, u32>,
    pub created_by: String,
    pub tags: Vec<String>,
}

/// 应用预设请求
#[derive(Debug, Deserialize)]
pub struct ApplyPresetRequest {
    pub preset_id: String,
    pub operator: String,
}

/// 权重标准化请求
#[derive(Debug, Deserialize)]
pub struct NormalizeWeightsRequest {
    pub target_total: u32,
    pub operator: String,
}

/// 权重均分请求
#[derive(Debug, Deserialize)]
pub struct DistributeWeightsRequest {
    pub total_weight: u32,
    pub operator: String,
}

/// 自动调整权重请求
#[derive(Debug, Deserialize)]
pub struct AutoAdjustRequest {
    pub performance_data: HashMap<String, PerformanceMetrics>,
    pub operator: String,
}

/// 通用API响应
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

/// 创建权重预设
async fn create_preset_handler(
    request: CreatePresetRequest,
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    
    match toolkit.create_preset(
        request.name,
        request.description,
        request.weights,
        request.created_by,
        request.tags,
    ).await {
        Ok(preset_id) => {
            let response = ApiResponse::success(preset_id);
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<String>::error(format!("创建预设失败: {}", e));
            Ok(warp::reply::json(&response))
        }
    }
}

/// 获取所有预设
async fn get_presets_handler(
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    let presets = toolkit.get_presets().await;
    
    let response = ApiResponse::success(presets);
    Ok(warp::reply::json(&response))
}

/// 获取单个预设
async fn get_preset_handler(
    preset_id: String,
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    
    match toolkit.get_preset(&preset_id).await {
        Some(preset) => {
            let response = ApiResponse::success(preset);
            Ok(warp::reply::json(&response))
        }
        None => {
            let response = ApiResponse::<WeightPreset>::error("预设不存在".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 应用权重预设
async fn apply_preset_handler(
    request: ApplyPresetRequest,
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    
    // 获取当前配置
    let mut config = state.config_state.get_config().await;
    
    match toolkit.apply_preset_config(
        &request.preset_id,
        &request.operator,
        &mut config.gemini.api_keys,
    ).await {
        Ok(()) => {
            // 保存配置
            drop(toolkit);
            if let Err(e) = state.config_state.update_config(config).await {
                let response = ApiResponse::<()>::error(format!("保存配置失败: {}", e));
                return Ok(warp::reply::json(&response));
            }
            
            let response = ApiResponse::success("预设应用成功");
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<String>::error(format!("应用预设失败: {}", e));
            Ok(warp::reply::json(&response))
        }
    }
}

/// 删除预设
async fn delete_preset_handler(
    preset_id: String,
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    
    match toolkit.delete_preset(&preset_id).await {
        Ok(()) => {
            let response = ApiResponse::success("预设删除成功");
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<String>::error(format!("删除预设失败: {}", e));
            Ok(warp::reply::json(&response))
        }
    }
}

/// 分析权重配置
async fn analyze_weights_handler(
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    let config = state.config_state.get_config().await;
    
    let analysis = toolkit.analyze_weights_config(&config.gemini.api_keys).await;
    
    let response = ApiResponse::success(analysis);
    Ok(warp::reply::json(&response))
}

/// 权重标准化
async fn normalize_weights_handler(
    request: NormalizeWeightsRequest,
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    let mut config = state.config_state.get_config().await;
    
    match toolkit.normalize_weights_config(
        &mut config.gemini.api_keys,
        request.target_total,
        &request.operator,
    ).await {
        Ok(()) => {
            drop(toolkit);
            // 保存配置
            if let Err(e) = state.config_state.update_config(config).await {
                let response = ApiResponse::<()>::error(format!("保存配置失败: {}", e));
                return Ok(warp::reply::json(&response));
            }
            
            let response = ApiResponse::success("权重标准化成功");
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<String>::error(format!("权重标准化失败: {}", e));
            Ok(warp::reply::json(&response))
        }
    }
}

/// 权重均分
async fn distribute_weights_handler(
    request: DistributeWeightsRequest,
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    let mut config = state.config_state.get_config().await;
    
    match toolkit.distribute_weights_evenly_config(
        &mut config.gemini.api_keys,
        request.total_weight,
        &request.operator,
    ).await {
        Ok(()) => {
            drop(toolkit);
            // 保存配置
            if let Err(e) = state.config_state.update_config(config).await {
                let response = ApiResponse::<()>::error(format!("保存配置失败: {}", e));
                return Ok(warp::reply::json(&response));
            }
            
            let response = ApiResponse::success("权重均分成功");
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<String>::error(format!("权重均分失败: {}", e));
            Ok(warp::reply::json(&response))
        }
    }
}

/// 自动调整权重
async fn auto_adjust_weights_handler(
    request: AutoAdjustRequest,
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    let mut config = state.config_state.get_config().await;
    
    match toolkit.auto_adjust_weights_config(
        &mut config.gemini.api_keys,
        &request.performance_data,
        &request.operator,
    ).await {
        Ok(()) => {
            drop(toolkit);
            // 保存配置
            if let Err(e) = state.config_state.update_config(config).await {
                let response = ApiResponse::<()>::error(format!("保存配置失败: {}", e));
                return Ok(warp::reply::json(&response));
            }
            
            let response = ApiResponse::success("权重自动调整成功");
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<String>::error(format!("权重自动调整失败: {}", e));
            Ok(warp::reply::json(&response))
        }
    }
}

/// 权重健康检查
async fn health_check_handler(
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    let config = state.config_state.get_config().await;
    
    let health_result = toolkit.health_check_config(&config.gemini.api_keys).await;
    
    let response = ApiResponse::success(health_result);
    Ok(warp::reply::json(&response))
}

/// 获取工具统计信息
async fn get_tool_stats_handler(
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    let config = state.config_state.get_config().await;
    
    let presets = toolkit.get_presets().await;
    let analysis = toolkit.analyze_weights_config(&config.gemini.api_keys).await;
    let health = toolkit.health_check_config(&config.gemini.api_keys).await;
    
    #[derive(Serialize)]
    struct ToolStats {
        total_presets: usize,
        load_balance_score: f64,
        health_status: String,
        total_weight: u32,
        active_keys: usize,
        recommendations_count: usize,
    }
    
    let stats = ToolStats {
        total_presets: presets.len(),
        load_balance_score: analysis.load_balance_score,
        health_status: format!("{:?}", health.status),
        total_weight: config.gemini.api_keys.iter().map(|k| k.weight).sum(),
        active_keys: config.gemini.api_keys.len(), // ApiKeyConfig 没有 enabled 字段，假设都是活跃的
        recommendations_count: analysis.recommended_adjustments.len(),
    };
    
    let response = ApiResponse::success(stats);
    Ok(warp::reply::json(&response))
}

/// 生成推荐预设
async fn generate_recommended_presets_handler(
    state: WeightToolsState,
) -> Result<impl Reply, Rejection> {
    let toolkit = state.toolkit.read().await;
    let config = state.config_state.get_config().await;
    
    let mut recommended_presets = Vec::new();
    
    // 生成均分预设
    let total_weight: u32 = config.gemini.api_keys.iter().map(|k| k.weight).sum();
    if total_weight > 0 && !config.gemini.api_keys.is_empty() {
        let weight_per_key = total_weight / config.gemini.api_keys.len() as u32;
        let mut even_weights = HashMap::new();
        for api_key in &config.gemini.api_keys {
            even_weights.insert(api_key.id.clone(), weight_per_key);
        }
        
        recommended_presets.push(WeightPreset {
            id: "recommended_even".to_string(),
            name: "均分权重".to_string(),
            description: "所有API密钥权重均分".to_string(),
            weights: even_weights,
            created_by: "system".to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tags: vec!["推荐".to_string(), "均分".to_string()],
        });
    }
    
    // 生成基于分析的预设
    let analysis = toolkit.analyze_weights_config(&config.gemini.api_keys).await;
    if !analysis.recommended_adjustments.is_empty() {
        let mut optimized_weights = HashMap::new();
        for api_key in &config.gemini.api_keys {
            let mut weight = api_key.weight;
            
            // 应用推荐调整
            for rec in &analysis.recommended_adjustments {
                if rec.key_id == api_key.id {
                    weight = rec.recommended_weight;
                    break;
                }
            }
            
            optimized_weights.insert(api_key.id.clone(), weight);
        }
        
        recommended_presets.push(WeightPreset {
            id: "recommended_optimized".to_string(),
            name: "优化权重".to_string(),
            description: "基于当前配置分析的优化建议".to_string(),
            weights: optimized_weights,
            created_by: "system".to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tags: vec!["推荐".to_string(), "优化".to_string()],
        });
    }
    
    let response = ApiResponse::success(recommended_presets);
    Ok(warp::reply::json(&response))
}

/// 权重工具 API 路由
pub fn weight_tools_routes(
    state: WeightToolsState,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let tools_state = warp::any().map(move || state.clone());

    // POST /api/tools/presets - 创建权重预设
    let create_preset = warp::path!("api" / "tools" / "presets")
        .and(warp::post())
        .and(warp::body::json())
        .and(tools_state.clone())
        .and_then(create_preset_handler);

    // GET /api/tools/presets - 获取所有预设
    let get_presets = warp::path!("api" / "tools" / "presets")
        .and(warp::get())
        .and(tools_state.clone())
        .and_then(get_presets_handler);

    // GET /api/tools/presets/{id} - 获取单个预设
    let get_preset = warp::path!("api" / "tools" / "presets" / String)
        .and(warp::get())
        .and(tools_state.clone())
        .and_then(get_preset_handler);

    // POST /api/tools/apply-preset - 应用预设
    let apply_preset = warp::path!("api" / "tools" / "apply-preset")
        .and(warp::post())
        .and(warp::body::json())
        .and(tools_state.clone())
        .and_then(apply_preset_handler);

    // DELETE /api/tools/presets/{id} - 删除预设
    let delete_preset = warp::path!("api" / "tools" / "presets" / String)
        .and(warp::delete())
        .and(tools_state.clone())
        .and_then(delete_preset_handler);

    // GET /api/tools/analyze - 分析权重配置
    let analyze_weights = warp::path!("api" / "tools" / "analyze")
        .and(warp::get())
        .and(tools_state.clone())
        .and_then(analyze_weights_handler);

    // POST /api/tools/normalize - 权重标准化
    let normalize_weights = warp::path!("api" / "tools" / "normalize")
        .and(warp::post())
        .and(warp::body::json())
        .and(tools_state.clone())
        .and_then(normalize_weights_handler);

    // POST /api/tools/distribute - 权重均分
    let distribute_weights = warp::path!("api" / "tools" / "distribute")
        .and(warp::post())
        .and(warp::body::json())
        .and(tools_state.clone())
        .and_then(distribute_weights_handler);

    // POST /api/tools/auto-adjust - 自动调整权重
    let auto_adjust = warp::path!("api" / "tools" / "auto-adjust")
        .and(warp::post())
        .and(warp::body::json())
        .and(tools_state.clone())
        .and_then(auto_adjust_weights_handler);

    // GET /api/tools/health-check - 权重健康检查
    let health_check = warp::path!("api" / "tools" / "health-check")
        .and(warp::get())
        .and(tools_state.clone())
        .and_then(health_check_handler);

    // GET /api/tools/stats - 获取工具统计信息
    let get_stats = warp::path!("api" / "tools" / "stats")
        .and(warp::get())
        .and(tools_state.clone())
        .and_then(get_tool_stats_handler);

    // GET /api/tools/recommended-presets - 生成推荐预设
    let recommended_presets = warp::path!("api" / "tools" / "recommended-presets")
        .and(warp::get())
        .and(tools_state.clone())
        .and_then(generate_recommended_presets_handler);

    create_preset
        .or(get_presets)
        .or(get_preset)
        .or(apply_preset)
        .or(delete_preset)
        .or(analyze_weights)
        .or(normalize_weights)
        .or(distribute_weights)
        .or(auto_adjust)
        .or(health_check)
        .or(get_stats)
        .or(recommended_presets)
}