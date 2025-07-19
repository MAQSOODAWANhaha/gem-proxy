// src/api/config.rs
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};
use crate::config::ProxyConfig;

// API 响应结构
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

    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

// 配置管理状态
#[derive(Clone)]
pub struct ConfigState {
    config: Arc<RwLock<ProxyConfig>>,
    config_path: String,
}

impl ConfigState {
    pub fn new(config: ProxyConfig, config_path: String) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
        }
    }

    pub async fn get_config(&self) -> ProxyConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, new_config: ProxyConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 验证配置
        self.validate_config(&new_config)?;
        
        // 保存到文件
        let yaml_content = serde_yaml::to_string(&new_config)?;
        tokio::fs::write(&self.config_path, yaml_content).await?;
        
        // 更新内存中的配置
        *self.config.write().await = new_config;
        
        Ok(())
    }

    fn validate_config(&self, config: &ProxyConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 基本验证
        if config.server.port == 0 {
            return Err("无效的服务器端口".into());
        }

        if config.server.workers == 0 || config.server.workers > 16 {
            return Err("工作进程数必须在 1-16 之间".into());
        }

        if config.gemini.api_keys.is_empty() {
            return Err("至少需要一个 API 密钥".into());
        }

        for key in &config.gemini.api_keys {
            if key.key.is_empty() {
                return Err("API 密钥不能为空".into());
            }
            if key.max_requests_per_minute == 0 {
                return Err("每分钟请求限制必须大于 0".into());
            }
        }

        if config.auth.enabled && config.auth.jwt_secret.len() < 16 {
            return Err("JWT 密钥长度至少 16 个字符".into());
        }

        Ok(())
    }
}

// API 路由
pub fn config_routes(
    state: ConfigState,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let config_state = warp::any().map(move || state.clone());

    // GET /config - 获取配置
    let get_config = warp::path!("config")
        .and(warp::get())
        .and(config_state.clone())
        .and_then(get_config_handler);

    // PUT /config - 更新配置
    let put_config = warp::path!("config")
        .and(warp::put())
        .and(warp::body::json())
        .and(config_state.clone())
        .and_then(update_config_handler);

    // POST /config/reload - 重新加载配置
    let reload_config = warp::path!("config" / "reload")
        .and(warp::post())
        .and(config_state.clone())
        .and_then(reload_config_handler);

    get_config.or(put_config).or(reload_config)
}

// 处理函数
async fn get_config_handler(state: ConfigState) -> Result<impl Reply, Rejection> {
    let config = state.get_config().await;
    let response = ApiResponse::success(config);
    Ok(warp::reply::json(&response))
}

async fn update_config_handler(
    new_config: ProxyConfig,
    state: ConfigState,
) -> Result<impl Reply, Rejection> {
    match state.update_config(new_config).await {
        Ok(_) => {
            let response = ApiResponse::success(());
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<()>::error(e.to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

async fn reload_config_handler(state: ConfigState) -> Result<impl Reply, Rejection> {
    match ProxyConfig::from_file(&state.config_path) {
        Ok(new_config) => {
            *state.config.write().await = new_config;
            let response = ApiResponse::success(());
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = ApiResponse::<()>::error(format!("重新加载配置失败: {e}"));
            Ok(warp::reply::json(&response))
        }
    }
}