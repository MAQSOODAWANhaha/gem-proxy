use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};

use crate::load_balancer::KeyManager;

/// 负载均衡统计信息
#[derive(Debug, Serialize, Clone)]
pub struct LoadBalancingStats {
    pub request_distribution: HashMap<String, RequestStats>,
    pub total_requests: u64,
    pub total_successful_requests: u64,
    pub total_failed_requests: u64,
    pub average_response_time: f64,
    pub current_qps: f64,
    pub peak_qps: f64,
    pub uptime_seconds: u64,
    pub distribution_effectiveness: f64,
}

/// 单个密钥的请求统计
#[derive(Debug, Serialize, Clone)]
pub struct RequestStats {
    pub key_id: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: f64,
    pub last_request_time: Option<u64>,
    pub current_weight: u32,
    pub expected_percentage: f64,
    pub actual_percentage: f64,
    pub effectiveness_score: f64,
}

/// 时间段统计
#[derive(Debug, Serialize, Clone)]
pub struct TimeBasedStats {
    pub hourly_stats: Vec<HourlyStats>,
    pub daily_summary: DailyStats,
    pub peak_hours: Vec<PeakHour>,
}

#[derive(Debug, Serialize, Clone)]
pub struct HourlyStats {
    pub hour: u8,
    pub total_requests: u64,
    pub average_response_time: f64,
    pub qps: f64,
    pub error_rate: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct DailyStats {
    pub date: String,
    pub total_requests: u64,
    pub unique_keys_used: u32,
    pub average_effectiveness: f64,
    pub weight_changes: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct PeakHour {
    pub hour: u8,
    pub qps: f64,
    pub concurrent_requests: u32,
}

/// 响应时间统计
#[derive(Debug, Serialize, Clone)]
pub struct ResponseTimeStats {
    pub percentiles: ResponseTimePercentiles,
    pub distribution: Vec<ResponseTimeBucket>,
    pub trends: Vec<ResponseTimeTrend>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ResponseTimePercentiles {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ResponseTimeBucket {
    pub range: String,
    pub count: u64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ResponseTimeTrend {
    pub timestamp: u64,
    pub average_ms: f64,
    pub p95_ms: f64,
}

/// 统计状态管理
#[derive(Clone)]
pub struct StatsState {
    pub key_manager: Option<Arc<KeyManager>>,
    pub stats_data: Arc<RwLock<LoadBalancingStats>>,
    pub start_time: SystemTime,
}

impl StatsState {
    pub fn new(key_manager: Option<Arc<KeyManager>>) -> Self {
        Self {
            key_manager,
            stats_data: Arc::new(RwLock::new(LoadBalancingStats::default())),
            start_time: SystemTime::now(),
        }
    }

    pub async fn get_key_manager(&self) -> Option<Arc<KeyManager>> {
        self.key_manager.clone()
    }
}

impl Default for LoadBalancingStats {
    fn default() -> Self {
        Self {
            request_distribution: HashMap::new(),
            total_requests: 0,
            total_successful_requests: 0,
            total_failed_requests: 0,
            average_response_time: 0.0,
            current_qps: 0.0,
            peak_qps: 0.0,
            uptime_seconds: 0,
            distribution_effectiveness: 100.0,
        }
    }
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

/// 获取负载均衡统计信息
async fn get_load_balancing_stats_handler(
    state: StatsState,
) -> Result<impl Reply, Rejection> {
    match state.get_key_manager().await {
        Some(key_manager) => {
            let weight_stats = key_manager.get_weight_stats().await;
            
            // 生成模拟的负载均衡统计数据
            let mut request_distribution = HashMap::new();
            let total_requests = 15000u64;
            
            for distribution in &weight_stats.key_distributions {
                let expected_percentage = distribution.percentage;
                let expected_requests = (total_requests as f64 * expected_percentage / 100.0) as u64;
                
                // 模拟实际请求分布（略有偏差）
                let variance = (rand::random::<f64>() - 0.5) * 0.1; // ±5% 偏差
                let actual_requests = ((expected_requests as f64) * (1.0 + variance)) as u64;
                let actual_percentage = (actual_requests as f64 / total_requests as f64) * 100.0;
                
                // 计算有效性评分
                let effectiveness_score: f64 = 100.0 - ((expected_percentage - actual_percentage).abs() / expected_percentage * 100.0);
                
                let stats = RequestStats {
                    key_id: distribution.key_id.clone(),
                    total_requests: actual_requests,
                    successful_requests: (actual_requests as f64 * 0.95) as u64, // 95% 成功率
                    failed_requests: (actual_requests as f64 * 0.05) as u64,
                    average_response_time: 150.0 + rand::random::<f64>() * 100.0, // 150-250ms
                    last_request_time: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
                    current_weight: distribution.weight,
                    expected_percentage,
                    actual_percentage,
                    effectiveness_score: effectiveness_score.max(0.0),
                };
                
                request_distribution.insert(distribution.key_id.clone(), stats);
            }
            
            // 计算整体统计
            let total_successful = request_distribution.values().map(|s| s.successful_requests).sum::<u64>();
            let total_failed = request_distribution.values().map(|s| s.failed_requests).sum::<u64>();
            let avg_response_time = request_distribution.values()
                .map(|s| s.average_response_time)
                .sum::<f64>() / request_distribution.len() as f64;
            
            let uptime_seconds = state.start_time.elapsed().unwrap_or(Duration::ZERO).as_secs();
            let current_qps = if uptime_seconds > 0 { total_requests as f64 / uptime_seconds as f64 } else { 0.0 };
            
            // 计算分布有效性
            let distribution_effectiveness = request_distribution.values()
                .map(|s| s.effectiveness_score)
                .sum::<f64>() / request_distribution.len() as f64;
            
            let stats = LoadBalancingStats {
                request_distribution,
                total_requests,
                total_successful_requests: total_successful,
                total_failed_requests: total_failed,
                average_response_time: avg_response_time,
                current_qps,
                peak_qps: current_qps * 1.5, // 模拟峰值
                uptime_seconds,
                distribution_effectiveness,
            };
            
            let response = ApiResponse::success(stats);
            Ok(warp::reply::json(&response))
        }
        None => {
            let response = ApiResponse::<()>::error("KeyManager not initialized".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

/// 获取时间段统计
async fn get_time_based_stats_handler(
    _state: StatsState,
) -> Result<impl Reply, Rejection> {
    // 生成模拟的时间段统计数据
    let mut hourly_stats = Vec::new();
    for hour in 0..24 {
        let base_qps = 50.0;
        let variation = (hour as f64 - 12.0).abs() / 12.0; // 中午峰值模式
        let qps = base_qps * (1.0 + variation);
        
        hourly_stats.push(HourlyStats {
            hour,
            total_requests: (qps * 3600.0) as u64,
            average_response_time: 180.0 + rand::random::<f64>() * 60.0,
            qps,
            error_rate: 0.02 + rand::random::<f64>() * 0.03, // 2-5% 错误率
        });
    }
    
    let daily_summary = DailyStats {
        date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        total_requests: hourly_stats.iter().map(|h| h.total_requests).sum(),
        unique_keys_used: 3,
        average_effectiveness: 87.5,
        weight_changes: 5,
    };
    
    let peak_hours = vec![
        PeakHour { hour: 12, qps: 125.0, concurrent_requests: 45 },
        PeakHour { hour: 14, qps: 118.0, concurrent_requests: 42 },
        PeakHour { hour: 16, qps: 110.0, concurrent_requests: 38 },
    ];
    
    let time_stats = TimeBasedStats {
        hourly_stats,
        daily_summary,
        peak_hours,
    };
    
    let response = ApiResponse::success(time_stats);
    Ok(warp::reply::json(&response))
}

/// 获取响应时间统计
async fn get_response_time_stats_handler(
    _state: StatsState,
) -> Result<impl Reply, Rejection> {
    // 生成模拟的响应时间统计
    let percentiles = ResponseTimePercentiles {
        p50: 165.0,
        p90: 240.0,
        p95: 280.0,
        p99: 450.0,
        p999: 1200.0,
    };
    
    let distribution = vec![
        ResponseTimeBucket { range: "0-100ms".to_string(), count: 2500, percentage: 16.7 },
        ResponseTimeBucket { range: "100-200ms".to_string(), count: 8000, percentage: 53.3 },
        ResponseTimeBucket { range: "200-300ms".to_string(), count: 3500, percentage: 23.3 },
        ResponseTimeBucket { range: "300-500ms".to_string(), count: 800, percentage: 5.3 },
        ResponseTimeBucket { range: "500ms+".to_string(), count: 200, percentage: 1.3 },
    ];
    
    let mut trends = Vec::new();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    for i in 0..24 {
        trends.push(ResponseTimeTrend {
            timestamp: now - (24 - i) * 3600,
            average_ms: 170.0 + (i as f64 * 5.0),
            p95_ms: 280.0 + (i as f64 * 8.0),
        });
    }
    
    let response_time_stats = ResponseTimeStats {
        percentiles,
        distribution,
        trends,
    };
    
    let response = ApiResponse::success(response_time_stats);
    Ok(warp::reply::json(&response))
}

/// 负载均衡统计 API 路由
pub fn load_balancing_stats_routes(
    state: StatsState,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let stats_state = warp::any().map(move || state.clone());

    // GET /api/stats/load-balancing - 获取负载均衡统计
    let get_load_balancing_stats = warp::path!("api" / "stats" / "load-balancing")
        .and(warp::get())
        .and(stats_state.clone())
        .and_then(get_load_balancing_stats_handler);

    // GET /api/stats/time-based - 获取时间段统计
    let get_time_based_stats = warp::path!("api" / "stats" / "time-based")
        .and(warp::get())
        .and(stats_state.clone())
        .and_then(get_time_based_stats_handler);

    // GET /api/stats/response-time - 获取响应时间统计
    let get_response_time_stats = warp::path!("api" / "stats" / "response-time")
        .and(warp::get())
        .and(stats_state.clone())
        .and_then(get_response_time_stats_handler);

    get_load_balancing_stats
        .or(get_time_based_stats)
        .or(get_response_time_stats)
}

// 需要在 Cargo.toml 中添加这些依赖
// chrono = { version = "0.4", features = ["serde"] }
// rand = "0.8"