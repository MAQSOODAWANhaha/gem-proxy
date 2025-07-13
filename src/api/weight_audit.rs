use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};

use crate::ConfigState;
use crate::load_balancer::{
    WeightAuditSystem, AuditQuery, WeightChangeRecord, WeightSnapshot,
    AuditStatistics, OperationType, ChangeSource, ExportFormat,
    WeightTrendPoint, AuditConfig
};

/// 权重审计状态管理
#[derive(Clone)]
pub struct WeightAuditState {
    pub audit_system: Arc<RwLock<WeightAuditSystem>>,
    pub config_state: ConfigState,
}

impl WeightAuditState {
    pub fn new(config_state: ConfigState) -> Self {
        let audit_config = AuditConfig::default();
        let audit_system = WeightAuditSystem::new(audit_config);
        
        Self {
            audit_system: Arc::new(RwLock::new(audit_system)),
            config_state,
        }
    }
}

/// 权重变更请求
#[derive(Debug, Deserialize)]
pub struct WeightChangeRequest {
    pub operator: String,
    pub operation_type: OperationType,
    pub target_key_id: String,
    pub old_weight: u32,
    pub new_weight: u32,
    pub reason: String,
    pub source: ChangeSource,
    pub metadata: Option<HashMap<String, String>>,
}

/// 快照创建请求
#[derive(Debug, Deserialize)]
pub struct SnapshotCreateRequest {
    pub description: String,
    pub created_by: String,
    pub weights: Option<HashMap<String, u32>>, // 如果为空则使用当前权重
}

/// 回滚请求
#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    pub snapshot_id: String,
    pub operator: String,
    pub reason: String,
}

/// 导出请求
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub query: AuditQuery,
    pub format: String, // "json" 或 "csv"
}

/// 权重趋势查询请求
#[derive(Debug, Deserialize)]
pub struct TrendQuery {
    pub key_id: String,
    pub days: u32,
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

/// 记录权重变更
async fn record_weight_change_handler(
    request: WeightChangeRequest,
    state: WeightAuditState,
) -> Result<impl Reply, Rejection> {
    let audit_system = state.audit_system.read().await;
    
    match audit_system.record_weight_change(
        &request.operator,
        request.operation_type,
        &request.target_key_id,
        request.old_weight,
        request.new_weight,
        &request.reason,
        request.source,
        request.metadata,
    ).await {
        Ok(record_id) => {
            let response = ApiResponse::success(record_id);
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<String>::error(format!("记录权重变更失败: {}", e));
            Ok(warp::reply::json(&response))
        }
    }
}

/// 查询审计记录
async fn query_audit_records_handler(
    query: AuditQuery,
    state: WeightAuditState,
) -> Result<impl Reply, Rejection> {
    let audit_system = state.audit_system.read().await;
    let records = audit_system.query_audit_records(&query).await;
    
    let response = ApiResponse::success(records);
    Ok(warp::reply::json(&response))
}

/// 获取审计统计信息
async fn get_audit_statistics_handler(
    days: Option<u32>,
    state: WeightAuditState,
) -> Result<impl Reply, Rejection> {
    let audit_system = state.audit_system.read().await;
    let statistics = audit_system.get_audit_statistics(days).await;
    
    let response = ApiResponse::success(statistics);
    Ok(warp::reply::json(&response))
}

/// 创建权重快照
async fn create_snapshot_handler(
    request: SnapshotCreateRequest,
    state: WeightAuditState,
) -> Result<impl Reply, Rejection> {
    let audit_system = state.audit_system.read().await;
    
    // 如果没有提供权重，从配置中获取当前权重
    let weights = if let Some(weights) = request.weights {
        weights
    } else {
        let config = state.config_state.get_config().await;
        let mut current_weights = HashMap::new();
        for api_key in &config.gemini.api_keys {
            current_weights.insert(api_key.id.clone(), api_key.weight);
        }
        current_weights
    };
    
    match audit_system.create_snapshot(
        &weights,
        &request.description,
        &request.created_by,
    ).await {
        Ok(snapshot_id) => {
            let response = ApiResponse::success(snapshot_id);
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<String>::error(format!("创建快照失败: {}", e));
            Ok(warp::reply::json(&response))
        }
    }
}

/// 获取快照列表
async fn get_snapshots_handler(
    limit: Option<usize>,
    state: WeightAuditState,
) -> Result<impl Reply, Rejection> {
    let audit_system = state.audit_system.read().await;
    let snapshots = audit_system.get_snapshots(limit).await;
    
    let response = ApiResponse::success(snapshots);
    Ok(warp::reply::json(&response))
}

/// 获取单个快照
async fn get_snapshot_handler(
    snapshot_id: String,
    state: WeightAuditState,
) -> Result<impl Reply, Rejection> {
    let audit_system = state.audit_system.read().await;
    
    match audit_system.get_snapshot(&snapshot_id).await {
        Some(snapshot) => {
            let response = ApiResponse::success(snapshot);
            Ok(warp::reply::json(&response))
        }
        None => {
            let response = ApiResponse::<WeightSnapshot>::error("快照不存在".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 回滚到快照
async fn rollback_to_snapshot_handler(
    request: RollbackRequest,
    state: WeightAuditState,
) -> Result<impl Reply, Rejection> {
    let audit_system = state.audit_system.read().await;
    
    match audit_system.rollback_to_snapshot(
        &request.snapshot_id,
        &request.operator,
        &request.reason,
    ).await {
        Ok(weights) => {
            // 这里需要实际应用权重到系统中
            // 实际实现时需要调用 KeyManager 或配置更新
            drop(audit_system);
            
            // 更新配置文件中的权重
            let mut config = state.config_state.get_config().await;
            for api_key in &mut config.gemini.api_keys {
                if let Some(&new_weight) = weights.get(&api_key.id) {
                    api_key.weight = new_weight;
                }
            }
            
            if let Err(e) = state.config_state.update_config(config).await {
                let response = ApiResponse::<HashMap<String, u32>>::error(
                    format!("回滚失败，无法保存配置: {}", e)
                );
                return Ok(warp::reply::json(&response));
            }
            
            let response = ApiResponse::success(weights);
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<HashMap<String, u32>>::error(
                format!("回滚失败: {}", e)
            );
            Ok(warp::reply::json(&response))
        }
    }
}

/// 导出审计记录
async fn export_audit_records_handler(
    request: ExportRequest,
    state: WeightAuditState,
) -> Result<impl Reply, Rejection> {
    let audit_system = state.audit_system.read().await;
    
    let format = match request.format.as_str() {
        "json" => ExportFormat::Json,
        "csv" => ExportFormat::Csv,
        _ => {
            let response = ApiResponse::<String>::error("不支持的导出格式".to_string());
            return Ok(warp::reply::json(&response));
        }
    };
    
    match audit_system.export_audit_records(&request.query, format).await {
        Ok(content) => {
            let response = ApiResponse::success(content);
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<String>::error(format!("导出失败: {}", e));
            Ok(warp::reply::json(&response))
        }
    }
}

/// 获取权重变更趋势
async fn get_weight_trend_handler(
    query: TrendQuery,
    state: WeightAuditState,
) -> Result<impl Reply, Rejection> {
    let audit_system = state.audit_system.read().await;
    let trend = audit_system.get_weight_change_trend(&query.key_id, query.days).await;
    
    let response = ApiResponse::success(trend);
    Ok(warp::reply::json(&response))
}

/// 获取操作类型列表
async fn get_operation_types_handler() -> Result<impl Reply, Rejection> {
    let types = vec![
        OperationType::Manual,
        OperationType::Intelligent,
        OperationType::Batch,
        OperationType::Rollback,
        OperationType::Automatic,
    ];
    
    let response = ApiResponse::success(types);
    Ok(warp::reply::json(&response))
}

/// 获取变更来源列表
async fn get_change_sources_handler() -> Result<impl Reply, Rejection> {
    let sources = vec![
        ChangeSource::WebUI,
        ChangeSource::API,
        ChangeSource::ConfigFile,
        ChangeSource::Optimizer,
        ChangeSource::Monitor,
    ];
    
    let response = ApiResponse::success(sources);
    Ok(warp::reply::json(&response))
}

/// 权重审计 API 路由
pub fn weight_audit_routes(
    state: WeightAuditState,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let audit_state = warp::any().map(move || state.clone());

    // POST /api/audit/record - 记录权重变更
    let record_change = warp::path!("api" / "audit" / "record")
        .and(warp::post())
        .and(warp::body::json())
        .and(audit_state.clone())
        .and_then(record_weight_change_handler);

    // POST /api/audit/query - 查询审计记录
    let query_records = warp::path!("api" / "audit" / "query")
        .and(warp::post())
        .and(warp::body::json())
        .and(audit_state.clone())
        .and_then(query_audit_records_handler);

    // GET /api/audit/statistics?days=7 - 获取审计统计
    let get_statistics = warp::path!("api" / "audit" / "statistics")
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and(audit_state.clone())
        .and_then(|params: HashMap<String, String>, state| async move {
            let days = params.get("days").and_then(|d| d.parse().ok());
            get_audit_statistics_handler(days, state).await
        });

    // POST /api/audit/snapshot - 创建快照
    let create_snapshot = warp::path!("api" / "audit" / "snapshot")
        .and(warp::post())
        .and(warp::body::json())
        .and(audit_state.clone())
        .and_then(create_snapshot_handler);

    // GET /api/audit/snapshots?limit=10 - 获取快照列表
    let get_snapshots = warp::path!("api" / "audit" / "snapshots")
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and(audit_state.clone())
        .and_then(|params: HashMap<String, String>, state| async move {
            let limit = params.get("limit").and_then(|l| l.parse().ok());
            get_snapshots_handler(limit, state).await
        });

    // GET /api/audit/snapshot/{id} - 获取单个快照
    let get_snapshot = warp::path!("api" / "audit" / "snapshot" / String)
        .and(warp::get())
        .and(audit_state.clone())
        .and_then(get_snapshot_handler);

    // POST /api/audit/rollback - 回滚到快照
    let rollback = warp::path!("api" / "audit" / "rollback")
        .and(warp::post())
        .and(warp::body::json())
        .and(audit_state.clone())
        .and_then(rollback_to_snapshot_handler);

    // POST /api/audit/export - 导出审计记录
    let export_records = warp::path!("api" / "audit" / "export")
        .and(warp::post())
        .and(warp::body::json())
        .and(audit_state.clone())
        .and_then(export_audit_records_handler);

    // POST /api/audit/trend - 获取权重变更趋势
    let get_trend = warp::path!("api" / "audit" / "trend")
        .and(warp::post())
        .and(warp::body::json())
        .and(audit_state.clone())
        .and_then(get_weight_trend_handler);

    // GET /api/audit/operation-types - 获取操作类型列表
    let get_operation_types = warp::path!("api" / "audit" / "operation-types")
        .and(warp::get())
        .and_then(get_operation_types_handler);

    // GET /api/audit/change-sources - 获取变更来源列表
    let get_change_sources = warp::path!("api" / "audit" / "change-sources")
        .and(warp::get())
        .and_then(get_change_sources_handler);

    record_change
        .or(query_records)
        .or(get_statistics)
        .or(create_snapshot)
        .or(get_snapshots)
        .or(get_snapshot)
        .or(rollback)
        .or(export_records)
        .or(get_trend)
        .or(get_operation_types)
        .or(get_change_sources)
}