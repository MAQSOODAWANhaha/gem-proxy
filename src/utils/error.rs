// src/utils/error.rs
use serde::{Deserialize, Serialize};
use std::fmt;
use std::error::Error as StdError;

/// 自定义错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProxyError {
    /// 配置相关错误
    Config { message: String, details: Option<String> },
    /// API密钥相关错误
    ApiKey { key_id: String, message: String },
    /// 认证错误
    Authentication { message: String },
    /// 网络连接错误
    Network { endpoint: String, message: String },
    /// 速率限制错误
    RateLimit { client_id: String, limit: u32 },
    /// TLS相关错误
    Tls { message: String, cert_path: Option<String> },
    /// ACME证书错误
    Acme { domain: String, message: String },
    /// 健康检查错误
    HealthCheck { check_name: String, message: String },
    /// 内部服务器错误
    Internal { message: String, source: Option<String> },
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyError::Config { message, details } => {
                write!(f, "配置错误: {}", message)?;
                if let Some(details) = details {
                    write!(f, " (详情: {})", details)?;
                }
                Ok(())
            }
            ProxyError::ApiKey { key_id, message } => {
                write!(f, "API密钥错误 [{}]: {}", key_id, message)
            }
            ProxyError::Authentication { message } => {
                write!(f, "认证错误: {}", message)
            }
            ProxyError::Network { endpoint, message } => {
                write!(f, "网络错误 [{}]: {}", endpoint, message)
            }
            ProxyError::RateLimit { client_id, limit } => {
                write!(f, "速率限制 [{}]: 超过每分钟{}次限制", client_id, limit)
            }
            ProxyError::Tls { message, cert_path } => {
                write!(f, "TLS错误: {}", message)?;
                if let Some(path) = cert_path {
                    write!(f, " (证书路径: {})", path)?;
                }
                Ok(())
            }
            ProxyError::Acme { domain, message } => {
                write!(f, "ACME证书错误 [{}]: {}", domain, message)
            }
            ProxyError::HealthCheck { check_name, message } => {
                write!(f, "健康检查失败 [{}]: {}", check_name, message)
            }
            ProxyError::Internal { message, source } => {
                write!(f, "内部错误: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (来源: {})", source)?;
                }
                Ok(())
            }
        }
    }
}

impl StdError for ProxyError {}

/// 错误严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,      // 轻微错误，不影响服务
    Medium,   // 中等错误，可能影响部分功能
    High,     // 严重错误，影响服务质量
    Critical, // 致命错误，服务不可用
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "轻微"),
            ErrorSeverity::Medium => write!(f, "中等"),
            ErrorSeverity::High => write!(f, "严重"),
            ErrorSeverity::Critical => write!(f, "致命"),
        }
    }
}

/// 错误上下文信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ErrorContext {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub error: ProxyError,
    pub severity: ErrorSeverity,
    pub component: String,
    pub request_id: Option<String>,
    pub client_ip: Option<String>,
    pub additional_info: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    #[allow(dead_code)]
    pub fn new(error: ProxyError, severity: ErrorSeverity, component: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            error,
            severity,
            component: component.into(),
            request_id: None,
            client_ip: None,
            additional_info: std::collections::HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_client_ip(mut self, client_ip: impl Into<String>) -> Self {
        self.client_ip = Some(client_ip.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_info.insert(key.into(), value.into());
        self
    }
}

/// 错误处理器
#[allow(dead_code)]
pub struct ErrorHandler {
    error_log: tokio::sync::RwLock<Vec<ErrorContext>>,
    max_log_size: usize,
}

impl ErrorHandler {
    #[allow(dead_code)]
    pub fn new(max_log_size: usize) -> Self {
        Self {
            error_log: tokio::sync::RwLock::new(Vec::with_capacity(max_log_size)),
            max_log_size,
        }
    }

    #[allow(dead_code)]
    pub async fn handle_error(&self, context: ErrorContext) {
        // 记录错误日志
        self.log_error(&context).await;
        
        // 存储错误到内存中（用于监控）
        self.store_error(context).await;
    }

    #[allow(dead_code)]
    async fn log_error(&self, context: &ErrorContext) {
        let _log_level = match context.severity {
            ErrorSeverity::Low => tracing::Level::WARN,
            ErrorSeverity::Medium => tracing::Level::ERROR,
            ErrorSeverity::High => tracing::Level::ERROR,
            ErrorSeverity::Critical => tracing::Level::ERROR,
        };

        match context.severity {
            ErrorSeverity::Low => tracing::warn!(
                error = %context.error,
                severity = %context.severity,
                component = %context.component,
                request_id = ?context.request_id,
                client_ip = ?context.client_ip,
                timestamp = %context.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
                additional_info = ?context.additional_info,
                "代理服务错误"
            ),
            _ => tracing::error!(
                error = %context.error,
                severity = %context.severity,
                component = %context.component,
                request_id = ?context.request_id,
                client_ip = ?context.client_ip,
                timestamp = %context.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
                additional_info = ?context.additional_info,
                "代理服务错误"
            ),
        }
    }

    #[allow(dead_code)]
    async fn store_error(&self, context: ErrorContext) {
        let mut log = self.error_log.write().await;
        
        // 如果日志过大，移除最旧的条目
        if log.len() >= self.max_log_size {
            log.remove(0);
        }
        
        log.push(context);
    }

    #[allow(dead_code)]
    pub async fn get_recent_errors(&self, limit: usize) -> Vec<ErrorContext> {
        let log = self.error_log.read().await;
        let start = if log.len() > limit { log.len() - limit } else { 0 };
        log[start..].to_vec()
    }

    pub async fn get_error_statistics(&self) -> ErrorStatistics {
        let log = self.error_log.read().await;
        let total_errors = log.len();
        
        let mut by_severity = std::collections::HashMap::new();
        let mut by_component = std::collections::HashMap::new();
        
        for error in log.iter() {
            *by_severity.entry(error.severity).or_insert(0) += 1;
            *by_component.entry(error.component.clone()).or_insert(0) += 1;
        }

        // 计算最近1小时的错误数
        let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
        let recent_errors = log.iter()
            .filter(|e| e.timestamp > one_hour_ago)
            .count();

        ErrorStatistics {
            total_errors,
            recent_errors,
            by_severity,
            by_component,
        }
    }

    #[allow(dead_code)]
    pub async fn clear_old_errors(&self, max_age: chrono::Duration) {
        let cutoff_time = chrono::Utc::now() - max_age;
        let mut log = self.error_log.write().await;
        log.retain(|error| error.timestamp > cutoff_time);
    }
}

/// 错误统计信息
#[derive(Debug, Clone, Serialize)]
pub struct ErrorStatistics {
    pub total_errors: usize,
    pub recent_errors: usize, // 最近1小时
    pub by_severity: std::collections::HashMap<ErrorSeverity, usize>,
    pub by_component: std::collections::HashMap<String, usize>,
}

/// 错误处理工具函数
#[allow(dead_code)]
pub mod utils {
    use super::*;
    
    #[allow(dead_code)]
    pub fn map_io_error(err: std::io::Error, component: &str) -> ErrorContext {
        let error = ProxyError::Internal {
            message: format!("IO错误: {}", err),
            source: Some(component.to_string()),
        };
        ErrorContext::new(error, ErrorSeverity::Medium, component)
    }

    #[allow(dead_code)]
    pub fn map_network_error(err: impl StdError, endpoint: &str) -> ErrorContext {
        let error = ProxyError::Network {
            endpoint: endpoint.to_string(),
            message: err.to_string(),
        };
        ErrorContext::new(error, ErrorSeverity::High, "network")
    }

    #[allow(dead_code)]
    pub fn map_tls_error(err: impl StdError, cert_path: Option<&str>) -> ErrorContext {
        let error = ProxyError::Tls {
            message: err.to_string(),
            cert_path: cert_path.map(|s| s.to_string()),
        };
        ErrorContext::new(error, ErrorSeverity::High, "tls")
    }

    #[allow(dead_code)]
    pub fn create_config_error(message: impl Into<String>, details: Option<String>) -> ErrorContext {
        let error = ProxyError::Config {
            message: message.into(),
            details,
        };
        ErrorContext::new(error, ErrorSeverity::Critical, "config")
    }

    #[allow(dead_code)]
    pub fn create_auth_error(message: impl Into<String>) -> ErrorContext {
        let error = ProxyError::Authentication {
            message: message.into(),
        };
        ErrorContext::new(error, ErrorSeverity::Medium, "auth")
    }

    #[allow(dead_code)]
    pub fn create_rate_limit_error(client_id: impl Into<String>, limit: u32) -> ErrorContext {
        let error = ProxyError::RateLimit {
            client_id: client_id.into(),
            limit,
        };
        ErrorContext::new(error, ErrorSeverity::Low, "rate_limit")
    }
}