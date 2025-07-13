// src/error/mod.rs
//! 统一错误处理模块
//! 
//! 提供项目范围内的统一错误类型定义、错误传播机制和结构化日志

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub mod context;
pub mod logging;
pub mod migration;
pub mod recovery;

// 暂时注释未使用的导出，避免编译警告
// pub use context::{ErrorContextManager, OperationContext};
// pub use logging::{ErrorLogger, ErrorLoggingConfig, create_default_logger, create_production_logger};
// pub use recovery::{ErrorRecoveryManager, RecoveryStrategy, create_default_recovery_manager, create_production_recovery_manager};

/// 项目主要错误类型
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum GeminiProxyError {
    /// 配置相关错误
    #[error("配置错误: {message}")]
    Config {
        message: String,
        #[serde(skip)]
        source: Option<Box<GeminiProxyError>>,
        context: ErrorContext,
    },

    /// 负载均衡器错误
    #[error("负载均衡错误: {message}")]
    LoadBalancer {
        message: String,
        #[serde(skip)]
        source: Option<Box<GeminiProxyError>>,
        context: ErrorContext,
    },

    /// API 认证错误
    #[error("认证错误: {message}")]
    Authentication {
        message: String,
        #[serde(skip)]
        source: Option<Box<GeminiProxyError>>,
        context: ErrorContext,
    },

    /// 网络/代理错误
    #[error("网络错误: {message}")]
    Network {
        message: String,
        #[serde(skip)]
        source: Option<Box<GeminiProxyError>>,
        context: ErrorContext,
    },

    /// 持久化存储错误
    #[error("存储错误: {message}")]
    Storage {
        message: String,
        #[serde(skip)]
        source: Option<Box<GeminiProxyError>>,
        context: ErrorContext,
    },

    /// TLS/加密错误
    #[error("TLS错误: {message}")]
    Tls {
        message: String,
        #[serde(skip)]
        source: Option<Box<GeminiProxyError>>,
        context: ErrorContext,
    },

    /// 速率限制错误
    #[error("速率限制错误: {message}")]
    RateLimit {
        message: String,
        #[serde(skip)]
        source: Option<Box<GeminiProxyError>>,
        context: ErrorContext,
    },

    /// 验证错误
    #[error("验证错误: {message}")]
    Validation {
        message: String,
        fields: Vec<ValidationError>,
        context: ErrorContext,
    },

    /// 资源不存在错误
    #[error("资源不存在: {resource_type} '{resource_id}'")]
    NotFound {
        resource_type: String,
        resource_id: String,
        context: ErrorContext,
    },

    /// 权限错误
    #[error("权限不足: {operation}")]
    Permission {
        operation: String,
        required_permissions: Vec<String>,
        context: ErrorContext,
    },

    /// 外部服务错误
    #[error("外部服务错误: {service} - {message}")]
    ExternalService {
        service: String,
        message: String,
        status_code: Option<u16>,
        context: ErrorContext,
    },

    /// 内部系统错误
    #[error("内部错误: {message}")]
    Internal {
        message: String,
        #[serde(skip)]
        source: Option<Box<GeminiProxyError>>,
        context: ErrorContext,
    },
}

/// 错误上下文信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorContext {
    /// 错误ID（用于日志追踪）
    pub error_id: String,
    /// 错误发生时间戳
    pub timestamp: u64,
    /// 错误发生的组件/模块
    pub component: String,
    /// 操作名称
    pub operation: String,
    /// 用户ID（如果适用）
    pub user_id: Option<String>,
    /// 请求ID（如果适用）
    pub request_id: Option<String>,
    /// 会话ID（如果适用）
    pub session_id: Option<String>,
    /// 附加元数据
    pub metadata: HashMap<String, String>,
    /// 错误级别
    pub severity: ErrorSeverity,
    /// 是否可重试
    pub retryable: bool,
    /// 建议的恢复操作
    pub recovery_hint: Option<String>,
}

/// 错误严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorSeverity {
    /// 调试级别
    Debug,
    /// 信息级别
    Info,
    /// 警告级别
    Warning,
    /// 错误级别
    Error,
    /// 严重错误
    Critical,
    /// 系统致命错误
    Fatal,
}

/// 验证错误详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub value: Option<String>,
}

/// 错误分类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// 用户输入错误
    UserError,
    /// 系统配置错误
    ConfigurationError,
    /// 外部依赖错误
    DependencyError,
    /// 资源错误
    ResourceError,
    /// 系统内部错误
    SystemError,
}

impl GeminiProxyError {
    /// 创建配置错误
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
            source: None,
            context: ErrorContext::new("config", "unknown"),
        }
    }

    /// 创建配置错误（带上下文）
    pub fn config_with_context<S: Into<String>>(
        message: S,
        component: &str,
        operation: &str,
    ) -> Self {
        Self::Config {
            message: message.into(),
            source: None,
            context: ErrorContext::new(component, operation),
        }
    }

    /// 创建负载均衡错误
    pub fn load_balancer<S: Into<String>>(message: S) -> Self {
        Self::LoadBalancer {
            message: message.into(),
            source: None,
            context: ErrorContext::new("load_balancer", "unknown"),
        }
    }

    /// 创建认证错误
    pub fn authentication<S: Into<String>>(message: S) -> Self {
        Self::Authentication {
            message: message.into(),
            source: None,
            context: ErrorContext::new("auth", "unknown"),
        }
    }

    /// 创建网络错误
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
            source: None,
            context: ErrorContext::new("network", "unknown"),
        }
    }

    /// 创建存储错误
    pub fn storage<S: Into<String>>(message: S) -> Self {
        Self::Storage {
            message: message.into(),
            source: None,
            context: ErrorContext::new("storage", "unknown"),
        }
    }

    /// 创建验证错误
    pub fn validation<S: Into<String>>(message: S, fields: Vec<ValidationError>) -> Self {
        Self::Validation {
            message: message.into(),
            fields,
            context: ErrorContext::new("validation", "validate"),
        }
    }

    /// 创建资源不存在错误
    pub fn not_found<S: Into<String>>(resource_type: S, resource_id: S) -> Self {
        Self::NotFound {
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            context: ErrorContext::new("resource", "find"),
        }
    }

    /// 创建权限错误
    pub fn permission<S: Into<String>>(operation: S, required_permissions: Vec<String>) -> Self {
        Self::Permission {
            operation: operation.into(),
            required_permissions,
            context: ErrorContext::new("auth", "check_permission"),
        }
    }

    /// 创建内部错误
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
            source: None,
            context: ErrorContext::new("system", "internal"),
        }
    }

    /// 添加错误上下文
    pub fn with_context(mut self, context: ErrorContext) -> Self {
        match &mut self {
            Self::Config { context: ctx, .. }
            | Self::LoadBalancer { context: ctx, .. }
            | Self::Authentication { context: ctx, .. }
            | Self::Network { context: ctx, .. }
            | Self::Storage { context: ctx, .. }
            | Self::Tls { context: ctx, .. }
            | Self::RateLimit { context: ctx, .. }
            | Self::Validation { context: ctx, .. }
            | Self::NotFound { context: ctx, .. }
            | Self::Permission { context: ctx, .. }
            | Self::ExternalService { context: ctx, .. }
            | Self::Internal { context: ctx, .. } => {
                *ctx = context;
            }
        }
        self
    }

    /// 添加用户ID到上下文
    pub fn with_user_id<S: Into<String>>(mut self, user_id: S) -> Self {
        self.get_context_mut().user_id = Some(user_id.into());
        self
    }

    /// 添加请求ID到上下文
    pub fn with_request_id<S: Into<String>>(mut self, request_id: S) -> Self {
        self.get_context_mut().request_id = Some(request_id.into());
        self
    }

    /// 添加会话ID到上下文
    pub fn with_session_id<S: Into<String>>(mut self, session_id: S) -> Self {
        self.get_context_mut().session_id = Some(session_id.into());
        self
    }

    /// 设置错误严重程度
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.get_context_mut().severity = severity;
        self
    }

    /// 设置是否可重试
    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.get_context_mut().retryable = retryable;
        self
    }

    /// 添加恢复提示
    pub fn with_recovery_hint<S: Into<String>>(mut self, hint: S) -> Self {
        self.get_context_mut().recovery_hint = Some(hint.into());
        self
    }

    /// 添加元数据
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.get_context_mut().metadata.insert(key.into(), value.into());
        self
    }

    /// 链式添加源错误
    pub fn caused_by(mut self, source: GeminiProxyError) -> Self {
        match &mut self {
            Self::Config { source: src, .. }
            | Self::LoadBalancer { source: src, .. }
            | Self::Authentication { source: src, .. }
            | Self::Network { source: src, .. }
            | Self::Storage { source: src, .. }
            | Self::Tls { source: src, .. }
            | Self::RateLimit { source: src, .. }
            | Self::Internal { source: src, .. } => {
                *src = Some(Box::new(source));
            }
            _ => {}
        }
        self
    }

    /// 获取错误上下文
    pub fn get_context(&self) -> &ErrorContext {
        match self {
            Self::Config { context, .. }
            | Self::LoadBalancer { context, .. }
            | Self::Authentication { context, .. }
            | Self::Network { context, .. }
            | Self::Storage { context, .. }
            | Self::Tls { context, .. }
            | Self::RateLimit { context, .. }
            | Self::Validation { context, .. }
            | Self::NotFound { context, .. }
            | Self::Permission { context, .. }
            | Self::ExternalService { context, .. }
            | Self::Internal { context, .. } => context,
        }
    }

    /// 获取可变错误上下文
    fn get_context_mut(&mut self) -> &mut ErrorContext {
        match self {
            Self::Config { context, .. }
            | Self::LoadBalancer { context, .. }
            | Self::Authentication { context, .. }
            | Self::Network { context, .. }
            | Self::Storage { context, .. }
            | Self::Tls { context, .. }
            | Self::RateLimit { context, .. }
            | Self::Validation { context, .. }
            | Self::NotFound { context, .. }
            | Self::Permission { context, .. }
            | Self::ExternalService { context, .. }
            | Self::Internal { context, .. } => context,
        }
    }

    /// 判断错误分类
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Config { .. } => ErrorCategory::ConfigurationError,
            Self::LoadBalancer { .. } => ErrorCategory::SystemError,
            Self::Authentication { .. } | Self::Permission { .. } => ErrorCategory::UserError,
            Self::Network { .. } | Self::ExternalService { .. } => ErrorCategory::DependencyError,
            Self::Storage { .. } => ErrorCategory::ResourceError,
            Self::Tls { .. } => ErrorCategory::ConfigurationError,
            Self::RateLimit { .. } => ErrorCategory::ResourceError,
            Self::Validation { .. } => ErrorCategory::UserError,
            Self::NotFound { .. } => ErrorCategory::ResourceError,
            Self::Internal { .. } => ErrorCategory::SystemError,
        }
    }

    /// 判断是否是用户错误
    pub fn is_user_error(&self) -> bool {
        matches!(self.category(), ErrorCategory::UserError)
    }

    /// 判断是否是系统错误
    pub fn is_system_error(&self) -> bool {
        matches!(self.category(), ErrorCategory::SystemError)
    }

    /// 获取错误链
    pub fn error_chain(&self) -> Vec<&Self> {
        let mut chain = vec![self];
        let mut current = self;
        
        while let Some(source) = self.get_source(current) {
            chain.push(source);
            current = source;
        }
        
        chain
    }

    /// 获取源错误
    fn get_source<'a>(&self, error: &'a Self) -> Option<&'a Self> {
        match error {
            Self::Config { source, .. }
            | Self::LoadBalancer { source, .. }
            | Self::Authentication { source, .. }
            | Self::Network { source, .. }
            | Self::Storage { source, .. }
            | Self::Tls { source, .. }
            | Self::RateLimit { source, .. }
            | Self::Internal { source, .. } => source.as_ref().map(|s| s.as_ref()),
            _ => None,
        }
    }

    /// 序列化为JSON用于日志记录
    pub fn to_log_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error_type": self.error_type_name(),
            "message": self.to_string(),
            "context": self.get_context(),
            "category": self.category(),
            "user_error": self.is_user_error(),
            "system_error": self.is_system_error(),
            "error_chain_length": self.error_chain().len(),
        })
    }

    /// 获取错误类型名称
    fn error_type_name(&self) -> &'static str {
        match self {
            Self::Config { .. } => "Config",
            Self::LoadBalancer { .. } => "LoadBalancer",
            Self::Authentication { .. } => "Authentication",
            Self::Network { .. } => "Network",
            Self::Storage { .. } => "Storage",
            Self::Tls { .. } => "Tls",
            Self::RateLimit { .. } => "RateLimit",
            Self::Validation { .. } => "Validation",
            Self::NotFound { .. } => "NotFound",
            Self::Permission { .. } => "Permission",
            Self::ExternalService { .. } => "ExternalService",
            Self::Internal { .. } => "Internal",
        }
    }
}

impl ErrorContext {
    /// 创建新的错误上下文
    pub fn new(component: &str, operation: &str) -> Self {
        Self {
            error_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            component: component.to_string(),
            operation: operation.to_string(),
            user_id: None,
            request_id: None,
            session_id: None,
            metadata: HashMap::new(),
            severity: ErrorSeverity::Error,
            retryable: false,
            recovery_hint: None,
        }
    }

    /// 创建带ID的上下文
    pub fn with_id(component: &str, operation: &str, error_id: String) -> Self {
        Self {
            error_id,
            timestamp: chrono::Utc::now().timestamp() as u64,
            component: component.to_string(),
            operation: operation.to_string(),
            user_id: None,
            request_id: None,
            session_id: None,
            metadata: HashMap::new(),
            severity: ErrorSeverity::Error,
            retryable: false,
            recovery_hint: None,
        }
    }
}

impl Default for ErrorSeverity {
    fn default() -> Self {
        Self::Error
    }
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, GeminiProxyError>;

/// 错误转换宏
#[macro_export]
macro_rules! impl_from_error {
    ($error_type:ty, $variant:ident) => {
        impl From<$error_type> for GeminiProxyError {
            fn from(err: $error_type) -> Self {
                GeminiProxyError::$variant {
                    message: err.to_string(),
                    source: None,
                    context: ErrorContext::new(stringify!($variant), "conversion"),
                }
            }
        }
    };
}

// 实现常见错误类型的转换
impl_from_error!(std::io::Error, Internal);
impl_from_error!(serde_json::Error, Internal);
impl_from_error!(serde_yaml::Error, Config);
impl_from_error!(pingora_error::Error, Network);

impl From<crate::persistence::PersistenceError> for GeminiProxyError {
    fn from(err: crate::persistence::PersistenceError) -> Self {
        match err {
            crate::persistence::PersistenceError::IoError(io_err) => {
                GeminiProxyError::Storage {
                    message: format!("IO错误: {}", io_err),
                    source: None,
                    context: ErrorContext::new("storage", "io_operation"),
                }
            }
            crate::persistence::PersistenceError::SerializationError(ser_err) => {
                GeminiProxyError::Storage {
                    message: format!("序列化错误: {}", ser_err),
                    source: None,
                    context: ErrorContext::new("storage", "serialization"),
                }
            }
            crate::persistence::PersistenceError::DataNotFound(resource) => {
                GeminiProxyError::NotFound {
                    resource_type: "data".to_string(),
                    resource_id: resource,
                    context: ErrorContext::new("storage", "lookup"),
                }
            }
            crate::persistence::PersistenceError::InvalidFormat(msg) => {
                GeminiProxyError::Validation {
                    message: format!("数据格式错误: {}", msg),
                    fields: vec![],
                    context: ErrorContext::new("storage", "validation"),
                }
            }
            crate::persistence::PersistenceError::PermissionError(msg) => {
                GeminiProxyError::Permission {
                    operation: "storage_access".to_string(),
                    required_permissions: vec!["storage_read".to_string(), "storage_write".to_string()],
                    context: ErrorContext::new("storage", "permission_check")
                        .with_metadata("details", msg),
                }
            }
        }
    }
}

impl ErrorContext {
    /// 添加元数据的便捷方法
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = GeminiProxyError::config("配置文件无效")
            .with_severity(ErrorSeverity::Critical)
            .with_retryable(false)
            .with_metadata("config_file", "/etc/proxy.yaml");

        assert_eq!(error.get_context().severity, ErrorSeverity::Critical);
        assert!(!error.get_context().retryable);
        assert_eq!(error.get_context().metadata.get("config_file"), Some(&"/etc/proxy.yaml".to_string()));
    }

    #[test]
    fn test_error_chain() {
        let root_error = GeminiProxyError::storage("磁盘空间不足");
        let middle_error = GeminiProxyError::config("配置保存失败")
            .caused_by(root_error);
        let top_error = GeminiProxyError::internal("系统初始化失败")
            .caused_by(middle_error);

        let chain = top_error.error_chain();
        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn test_error_categorization() {
        let config_error = GeminiProxyError::config("配置错误");
        let auth_error = GeminiProxyError::authentication("认证失败");
        let storage_error = GeminiProxyError::storage("存储错误");

        assert!(matches!(config_error.category(), ErrorCategory::ConfigurationError));
        assert!(auth_error.is_user_error());
        assert!(!storage_error.is_user_error());
    }

    #[test]
    fn test_error_json_serialization() {
        let error = GeminiProxyError::validation(
            "输入验证失败",
            vec![ValidationError {
                field: "email".to_string(),
                message: "邮箱格式无效".to_string(),
                value: Some("invalid-email".to_string()),
            }]
        );

        let json = error.to_log_json();
        assert!(json.get("error_type").is_some());
        assert!(json.get("message").is_some());
        assert!(json.get("context").is_some());
    }
}