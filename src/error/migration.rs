// src/error/migration.rs
//! 错误系统迁移助手
//! 
//! 提供从旧错误系统到新统一错误系统的平滑迁移

use super::{GeminiProxyError, ErrorContext, ErrorSeverity};
use crate::utils::error::{ProxyError, ErrorSeverity as OldErrorSeverity};

/// 错误迁移工具
pub struct ErrorMigrationHelper;

impl ErrorMigrationHelper {
    /// 将旧的 ProxyError 转换为新的 GeminiProxyError
    pub fn migrate_proxy_error(old_error: ProxyError, component: &str, operation: &str) -> GeminiProxyError {
        let mut context = ErrorContext::new(component, operation);
        context.severity = Self::migrate_severity(&Self::infer_old_severity(&old_error));

        match old_error {
            ProxyError::Config { message, details } => {
                let full_message = if let Some(details) = details {
                    format!("{} (详情: {})", message, details)
                } else {
                    message
                };
                GeminiProxyError::config_with_context(full_message, component, operation)
                    .with_context(context)
            }
            ProxyError::ApiKey { key_id, message } => {
                let mut ctx = context.clone();
                ctx.metadata.insert("key_id".to_string(), key_id.clone());
                GeminiProxyError::load_balancer(format!("API密钥错误 [{}]: {}", key_id, message))
                    .with_context(ctx)
            }
            ProxyError::Authentication { message } => {
                GeminiProxyError::authentication(message)
                    .with_context(context)
            }
            ProxyError::Network { endpoint, message } => {
                let mut ctx = context.clone();
                ctx.metadata.insert("endpoint".to_string(), endpoint.clone());
                GeminiProxyError::network(format!("网络错误 [{}]: {}", endpoint, message))
                    .with_context(ctx)
            }
            ProxyError::RateLimit { client_id, limit } => {
                GeminiProxyError::RateLimit {
                    message: format!("速率限制 [{}]: 超过每分钟{}次限制", client_id, limit),
                    source: None,
                    context: {
                        let mut ctx = context.clone();
                        ctx.metadata.insert("client_id".to_string(), client_id.clone());
                        ctx.metadata.insert("limit".to_string(), limit.to_string());
                        ctx
                    },
                }
            }
            ProxyError::Tls { message, cert_path } => {
                let mut ctx = context.clone();
                ctx.metadata.insert("tls_error".to_string(), "true".to_string());
                if let Some(path) = cert_path {
                    ctx.metadata.insert("cert_path".to_string(), path.clone());
                }
                GeminiProxyError::Tls {
                    message,
                    source: None,
                    context: ctx,
                }
            }
            ProxyError::Acme { domain, message } => {
                GeminiProxyError::Tls {
                    message: format!("ACME证书错误 [{}]: {}", domain, message),
                    source: None,
                    context: {
                        let mut ctx = context.clone();
                        ctx.metadata.insert("acme_domain".to_string(), domain.clone());
                        ctx.metadata.insert("acme_error".to_string(), "true".to_string());
                        ctx
                    },
                }
            }
            ProxyError::HealthCheck { check_name, message } => {
                GeminiProxyError::internal(format!("健康检查失败 [{}]: {}", check_name, message))
                    .with_context({
                        let mut ctx = context.clone();
                        ctx.metadata.insert("health_check".to_string(), check_name.clone());
                        ctx
                    })
            }
            ProxyError::Internal { message, source } => {
                let mut ctx = context;
                if let Some(source) = source {
                    ctx.metadata.insert("error_source".to_string(), source.clone());
                }
                GeminiProxyError::internal(message).with_context(ctx)
            }
        }
    }

    /// 将旧的错误严重程度转换为新的严重程度
    pub fn migrate_severity(old_severity: &OldErrorSeverity) -> ErrorSeverity {
        match old_severity {
            OldErrorSeverity::Low => ErrorSeverity::Warning,
            OldErrorSeverity::Medium => ErrorSeverity::Error,
            OldErrorSeverity::High => ErrorSeverity::Critical,
            OldErrorSeverity::Critical => ErrorSeverity::Fatal,
        }
    }

    /// 推断旧错误的严重程度
    fn infer_old_severity(error: &ProxyError) -> OldErrorSeverity {
        match error {
            ProxyError::Config { .. } => OldErrorSeverity::Critical,
            ProxyError::ApiKey { .. } => OldErrorSeverity::High,
            ProxyError::Authentication { .. } => OldErrorSeverity::Medium,
            ProxyError::Network { .. } => OldErrorSeverity::High,
            ProxyError::RateLimit { .. } => OldErrorSeverity::Low,
            ProxyError::Tls { .. } => OldErrorSeverity::High,
            ProxyError::Acme { .. } => OldErrorSeverity::High,
            ProxyError::HealthCheck { .. } => OldErrorSeverity::Medium,
            ProxyError::Internal { .. } => OldErrorSeverity::Critical,
        }
    }


    /// 创建包装函数，用于兼容旧的Result类型
    pub fn wrap_legacy_result<T>(
        result: Result<T, ProxyError>,
        component: &str,
        operation: &str,
    ) -> Result<T, GeminiProxyError> {
        match result {
            Ok(value) => Ok(value),
            Err(old_error) => Err(Self::migrate_proxy_error(old_error, component, operation)),
        }
    }
}

/// 兼容性适配器，用于将新错误系统集成到现有代码中
pub struct ErrorCompatibilityAdapter;

impl ErrorCompatibilityAdapter {
    /// 将 GeminiProxyError 转换为旧的 ErrorContext (用于向后兼容)
    pub fn to_legacy_context(error: &GeminiProxyError) -> crate::utils::error::ErrorContext {
        let proxy_error = Self::to_legacy_proxy_error(error);
        let severity = Self::to_legacy_severity(&error.get_context().severity);
        
        crate::utils::error::ErrorContext::new(
            proxy_error,
            severity,
            &error.get_context().component,
        )
        .with_request_id(error.get_context().request_id.as_deref().unwrap_or("unknown"))
    }

    /// 将 GeminiProxyError 转换为旧的 ProxyError
    fn to_legacy_proxy_error(error: &GeminiProxyError) -> ProxyError {
        match error {
            GeminiProxyError::Config { message, .. } => {
                ProxyError::Config {
                    message: message.clone(),
                    details: None,
                }
            }
            GeminiProxyError::LoadBalancer { message, .. } => {
                ProxyError::ApiKey {
                    key_id: "unknown".to_string(),
                    message: message.clone(),
                }
            }
            GeminiProxyError::Authentication { message, .. } => {
                ProxyError::Authentication {
                    message: message.clone(),
                }
            }
            GeminiProxyError::Network { message, .. } => {
                ProxyError::Network {
                    endpoint: "unknown".to_string(),
                    message: message.clone(),
                }
            }
            GeminiProxyError::RateLimit { message: _message, .. } => {
                ProxyError::RateLimit {
                    client_id: "unknown".to_string(),
                    limit: 60, // 默认值
                }
            }
            GeminiProxyError::Tls { message, .. } => {
                ProxyError::Tls {
                    message: message.clone(),
                    cert_path: None,
                }
            }
            GeminiProxyError::Internal { message, .. } => {
                ProxyError::Internal {
                    message: message.clone(),
                    source: Some(error.get_context().component.clone()),
                }
            }
            _ => {
                ProxyError::Internal {
                    message: error.to_string(),
                    source: Some("migration".to_string()),
                }
            }
        }
    }

    /// 将新的错误严重程度转换为旧的严重程度
    fn to_legacy_severity(severity: &ErrorSeverity) -> OldErrorSeverity {
        match severity {
            ErrorSeverity::Debug | ErrorSeverity::Info => OldErrorSeverity::Low,
            ErrorSeverity::Warning => OldErrorSeverity::Low,
            ErrorSeverity::Error => OldErrorSeverity::Medium,
            ErrorSeverity::Critical => OldErrorSeverity::High,
            ErrorSeverity::Fatal => OldErrorSeverity::Critical,
        }
    }
}

/// 逐步迁移的错误处理器
/// 
/// 这个处理器可以同时处理新旧两种错误类型，
/// 帮助在迁移过程中保持系统的正常运行
pub struct HybridErrorHandler {
    /// 新的错误日志记录器
    new_logger: Option<crate::error::logging::ErrorLogger>,
    /// 旧的错误处理器
    legacy_handler: crate::utils::error::ErrorHandler,
}

impl HybridErrorHandler {
    /// 创建混合错误处理器
    pub fn new(max_legacy_log_size: usize) -> Self {
        Self {
            new_logger: None,
            legacy_handler: crate::utils::error::ErrorHandler::new(max_legacy_log_size),
        }
    }

    /// 启用新的错误日志记录器
    pub fn with_new_logger(mut self, logger: crate::error::logging::ErrorLogger) -> Self {
        self.new_logger = Some(logger);
        self
    }

    /// 处理新的错误类型
    pub async fn handle_gemini_error(&self, error: &GeminiProxyError) -> Result<(), String> {
        // 使用新的日志记录器
        if let Some(logger) = &self.new_logger {
            logger.log_error(error).await?;
        }

        // 为了向后兼容，也发送到旧的处理器
        let legacy_context = ErrorCompatibilityAdapter::to_legacy_context(error);
        self.legacy_handler.handle_error(legacy_context).await;

        Ok(())
    }

    /// 处理旧的错误类型
    pub async fn handle_legacy_error(&self, context: crate::utils::error::ErrorContext) {
        // 转换为新错误类型并处理
        let migrated_error = ErrorMigrationHelper::migrate_proxy_error(
            context.error.clone(),
            &context.component,
            "legacy_operation",
        );

        // 使用新的处理器
        if let Err(e) = self.handle_gemini_error(&migrated_error).await {
            tracing::error!("处理迁移错误失败: {}", e);
        }

        // 也使用旧的处理器
        self.legacy_handler.handle_error(context).await;
    }

    /// 获取统计信息（合并新旧两种统计）
    pub async fn get_statistics(&self) -> crate::utils::error::ErrorStatistics {
        // 目前返回旧的统计信息
        // 后续可以扩展为合并新旧统计信息
        self.legacy_handler.get_error_statistics().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::error::ProxyError;

    #[test]
    fn test_migrate_proxy_error() {
        let old_error = ProxyError::Config {
            message: "配置文件无效".to_string(),
            details: Some("缺少必要字段".to_string()),
        };

        let migrated = ErrorMigrationHelper::migrate_proxy_error(
            old_error,
            "config",
            "load_config",
        );

        match migrated {
            GeminiProxyError::Config { message, .. } => {
                assert!(message.contains("配置文件无效"));
                assert!(message.contains("缺少必要字段"));
            }
            _ => panic!("错误类型转换失败"),
        }
    }

    #[test]
    fn test_severity_migration() {
        assert_eq!(
            ErrorMigrationHelper::migrate_severity(&crate::utils::error::ErrorSeverity::Low),
            ErrorSeverity::Warning
        );
        assert_eq!(
            ErrorMigrationHelper::migrate_severity(&crate::utils::error::ErrorSeverity::Critical),
            ErrorSeverity::Fatal
        );
    }

    #[test]
    fn test_compatibility_adapter() {
        let new_error = GeminiProxyError::authentication("认证失败");
        let legacy_context = ErrorCompatibilityAdapter::to_legacy_context(&new_error);
        
        match legacy_context.error {
            ProxyError::Authentication { message } => {
                assert_eq!(message, "认证失败");
            }
            _ => panic!("兼容性转换失败"),
        }
    }
}