// src/error/context.rs
//! 错误上下文管理
//! 
//! 提供错误上下文的传播、收集和管理功能

use super::{ErrorContext, GeminiProxyError, ErrorSeverity};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, warn, info, debug};

/// 错误上下文管理器
#[derive(Clone)]
pub struct ErrorContextManager {
    /// 当前请求的上下文
    current_context: Arc<RwLock<RequestContext>>,
    /// 全局错误统计
    error_stats: Arc<RwLock<ErrorStatistics>>,
}

/// 请求上下文
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// 请求ID
    pub request_id: Option<String>,
    /// 用户ID
    pub user_id: Option<String>,
    /// 会话ID
    pub session_id: Option<String>,
    /// 操作链路
    pub operation_stack: Vec<String>,
    /// 组件路径
    pub component_path: Vec<String>,
    /// 请求元数据
    pub metadata: HashMap<String, String>,
    /// 开始时间
    pub start_time: u64,
}

/// 错误统计信息
#[derive(Debug, Clone, Default)]
pub struct ErrorStatistics {
    /// 按错误类型统计
    pub by_type: HashMap<String, ErrorTypeStats>,
    /// 按组件统计
    pub by_component: HashMap<String, ComponentErrorStats>,
    /// 按严重程度统计
    pub by_severity: HashMap<String, u64>,
    /// 总错误数
    pub total_errors: u64,
    /// 用户错误数
    pub user_errors: u64,
    /// 系统错误数
    pub system_errors: u64,
}

/// 错误类型统计
#[derive(Debug, Clone, Default)]
pub struct ErrorTypeStats {
    pub count: u64,
    pub last_occurrence: u64,
    pub first_occurrence: u64,
    pub avg_resolution_time: Option<f64>,
}

/// 组件错误统计
#[derive(Debug, Clone, Default)]
pub struct ComponentErrorStats {
    pub error_count: u64,
    pub last_error_time: u64,
    pub error_rate: f64, // 错误/总请求比例
    pub most_common_errors: Vec<(String, u64)>,
}

/// 操作上下文包装器
pub struct OperationContext {
    manager: ErrorContextManager,
    operation: String,
    component: String,
}

impl ErrorContextManager {
    /// 创建新的错误上下文管理器
    pub fn new() -> Self {
        Self {
            current_context: Arc::new(RwLock::new(RequestContext::default())),
            error_stats: Arc::new(RwLock::new(ErrorStatistics::default())),
        }
    }

    /// 设置请求上下文
    pub async fn set_request_context(&self, request_id: String, user_id: Option<String>) {
        let mut context = self.current_context.write().await;
        context.request_id = Some(request_id);
        context.user_id = user_id;
        context.start_time = chrono::Utc::now().timestamp() as u64;
        context.operation_stack.clear();
        context.component_path.clear();
        context.metadata.clear();
    }

    /// 开始新操作
    pub async fn begin_operation(&self, component: &str, operation: &str) -> OperationContext {
        let mut context = self.current_context.write().await;
        context.operation_stack.push(operation.to_string());
        context.component_path.push(component.to_string());
        
        debug!(
            component = component,
            operation = operation,
            request_id = ?context.request_id,
            "开始操作"
        );

        OperationContext {
            manager: self.clone(),
            operation: operation.to_string(),
            component: component.to_string(),
        }
    }

    /// 结束操作
    pub async fn end_operation(&self, component: &str, operation: &str) {
        let mut context = self.current_context.write().await;
        
        // 移除操作栈顶部的操作
        if let Some(last_op) = context.operation_stack.last() {
            if last_op == operation {
                context.operation_stack.pop();
            }
        }
        
        // 移除组件路径顶部的组件
        if let Some(last_comp) = context.component_path.last() {
            if last_comp == component {
                context.component_path.pop();
            }
        }

        debug!(
            component = component,
            operation = operation,
            request_id = ?context.request_id,
            "结束操作"
        );
    }

    /// 添加上下文元数据
    pub async fn add_metadata<K: Into<String>, V: Into<String>>(&self, key: K, value: V) {
        let mut context = self.current_context.write().await;
        context.metadata.insert(key.into(), value.into());
    }

    /// 创建增强的错误上下文
    pub async fn create_error_context(&self, component: &str, operation: &str) -> ErrorContext {
        let context = self.current_context.read().await;
        
        let mut error_context = ErrorContext::new(component, operation);
        
        // 设置请求相关信息
        error_context.request_id = context.request_id.clone();
        error_context.user_id = context.user_id.clone();
        error_context.session_id = context.session_id.clone();
        
        // 添加操作栈信息
        if !context.operation_stack.is_empty() {
            error_context.metadata.insert(
                "operation_stack".to_string(),
                context.operation_stack.join(" -> ")
            );
        }
        
        // 添加组件路径信息
        if !context.component_path.is_empty() {
            error_context.metadata.insert(
                "component_path".to_string(),
                context.component_path.join(" -> ")
            );
        }
        
        // 添加请求元数据
        for (key, value) in &context.metadata {
            error_context.metadata.insert(format!("req_{}", key), value.clone());
        }
        
        error_context
    }

    /// 记录错误并更新统计
    pub async fn record_error(&self, error: &GeminiProxyError) {
        let mut stats = self.error_stats.write().await;
        let now = chrono::Utc::now().timestamp() as u64;
        
        // 更新总计数
        stats.total_errors += 1;
        
        // 更新用户/系统错误计数
        if error.is_user_error() {
            stats.user_errors += 1;
        } else {
            stats.system_errors += 1;
        }
        
        // 更新按类型统计
        let error_type = error.error_type_name();
        let type_stats = stats.by_type.entry(error_type.to_string()).or_default();
        type_stats.count += 1;
        type_stats.last_occurrence = now;
        if type_stats.first_occurrence == 0 {
            type_stats.first_occurrence = now;
        }
        
        // 更新按组件统计
        let component = &error.get_context().component;
        let component_stats = stats.by_component.entry(component.clone()).or_default();
        component_stats.error_count += 1;
        component_stats.last_error_time = now;
        
        // 更新按严重程度统计
        let severity = format!("{:?}", error.get_context().severity);
        *stats.by_severity.entry(severity).or_insert(0) += 1;
        
        // 记录结构化日志
        self.log_error(error).await;
    }

    /// 记录结构化错误日志
    async fn log_error(&self, error: &GeminiProxyError) {
        let context = self.current_context.read().await;
        
        match error.get_context().severity {
            ErrorSeverity::Debug => {
                debug!(
                    error_id = error.get_context().error_id,
                    error_type = error.error_type_name(),
                    message = %error,
                    component = error.get_context().component,
                    operation = error.get_context().operation,
                    request_id = ?context.request_id,
                    user_id = ?context.user_id,
                    "调试错误"
                );
            }
            ErrorSeverity::Info => {
                info!(
                    error_id = error.get_context().error_id,
                    error_type = error.error_type_name(),
                    message = %error,
                    component = error.get_context().component,
                    operation = error.get_context().operation,
                    request_id = ?context.request_id,
                    user_id = ?context.user_id,
                    "信息错误"
                );
            }
            ErrorSeverity::Warning => {
                warn!(
                    error_id = error.get_context().error_id,
                    error_type = error.error_type_name(),
                    message = %error,
                    component = error.get_context().component,
                    operation = error.get_context().operation,
                    request_id = ?context.request_id,
                    user_id = ?context.user_id,
                    retryable = error.get_context().retryable,
                    "警告错误"
                );
            }
            ErrorSeverity::Error | ErrorSeverity::Critical | ErrorSeverity::Fatal => {
                error!(
                    error_id = error.get_context().error_id,
                    error_type = error.error_type_name(),
                    message = %error,
                    component = error.get_context().component,
                    operation = error.get_context().operation,
                    request_id = ?context.request_id,
                    user_id = ?context.user_id,
                    session_id = ?context.session_id,
                    retryable = error.get_context().retryable,
                    recovery_hint = ?error.get_context().recovery_hint,
                    metadata = ?error.get_context().metadata,
                    severity = ?error.get_context().severity,
                    "严重错误"
                );
            }
        }
    }

    /// 获取错误统计信息
    pub async fn get_error_statistics(&self) -> ErrorStatistics {
        self.error_stats.read().await.clone()
    }

    /// 清理错误统计（保留最近的数据）
    pub async fn cleanup_old_statistics(&self, retention_hours: u64) {
        let mut stats = self.error_stats.write().await;
        let cutoff_time = chrono::Utc::now().timestamp() as u64 - (retention_hours * 3600);
        
        // 清理组件统计中的过期数据
        stats.by_component.retain(|_, component_stats| {
            component_stats.last_error_time > cutoff_time
        });
        
        // 清理错误类型统计中的过期数据
        stats.by_type.retain(|_, type_stats| {
            type_stats.last_occurrence > cutoff_time
        });
    }

    /// 获取当前请求上下文
    pub async fn get_current_context(&self) -> RequestContext {
        self.current_context.read().await.clone()
    }

    /// 创建带错误上下文的错误
    pub async fn create_error<F>(&self, component: &str, operation: &str, error_fn: F) -> GeminiProxyError
    where
        F: FnOnce(ErrorContext) -> GeminiProxyError,
    {
        let context = self.create_error_context(component, operation).await;
        let error = error_fn(context);
        self.record_error(&error).await;
        error
    }
}

impl OperationContext {
    /// 在操作完成时自动清理
    pub async fn finish(self) {
        self.manager.end_operation(&self.component, &self.operation).await;
    }

    /// 在操作中创建错误
    pub async fn create_error<F>(&self, error_fn: F) -> GeminiProxyError
    where
        F: FnOnce(ErrorContext) -> GeminiProxyError,
    {
        self.manager.create_error(&self.component, &self.operation, error_fn).await
    }

    /// 添加操作元数据
    pub async fn add_metadata<K: Into<String>, V: Into<String>>(&self, key: K, value: V) {
        self.manager.add_metadata(key, value).await;
    }
}

impl Drop for OperationContext {
    fn drop(&mut self) {
        // 注意：在 Drop 中无法使用 async，这里只是一个安全网
        // 正常情况下应该显式调用 finish()
        debug!(
            component = self.component,
            operation = self.operation,
            "操作上下文被销毁（可能未正确清理）"
        );
    }
}

/// 错误上下文管理的便捷宏
#[macro_export]
macro_rules! with_error_context {
    ($manager:expr, $component:expr, $operation:expr, $block:block) => {{
        let _ctx = $manager.begin_operation($component, $operation).await;
        let result = { $block };
        _ctx.finish().await;
        result
    }};
}

/// 创建带上下文的错误的便捷宏
#[macro_export]
macro_rules! error_with_context {
    ($manager:expr, $component:expr, $operation:expr, $error_constructor:expr) => {{
        $manager.create_error($component, $operation, $error_constructor).await
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GeminiProxyError;

    #[tokio::test]
    async fn test_error_context_manager() {
        let manager = ErrorContextManager::new();
        
        // 设置请求上下文
        manager.set_request_context("req-123".to_string(), Some("user-456".to_string())).await;
        
        // 开始操作
        let ctx = manager.begin_operation("auth", "login").await;
        
        // 添加元数据
        ctx.add_metadata("ip_address", "192.168.1.1").await;
        
        // 创建错误
        let error = ctx.create_error(|context| {
            GeminiProxyError::authentication("登录失败")
                .with_context(context)
        }).await;
        
        // 验证错误上下文
        assert_eq!(error.get_context().component, "auth");
        assert_eq!(error.get_context().operation, "login");
        assert_eq!(error.get_context().request_id, Some("req-123".to_string()));
        assert_eq!(error.get_context().user_id, Some("user-456".to_string()));
        
        // 结束操作
        ctx.finish().await;
        
        // 检查统计信息
        let stats = manager.get_error_statistics().await;
        assert_eq!(stats.total_errors, 1);
        assert_eq!(stats.user_errors, 1);
    }

    #[tokio::test]
    async fn test_operation_stack() {
        let manager = ErrorContextManager::new();
        
        manager.set_request_context("req-789".to_string(), None).await;
        
        let ctx1 = manager.begin_operation("proxy", "handle_request").await;
        let ctx2 = manager.begin_operation("load_balancer", "select_key").await;
        
        let context = manager.get_current_context().await;
        assert_eq!(context.operation_stack, vec!["handle_request", "select_key"]);
        assert_eq!(context.component_path, vec!["proxy", "load_balancer"]);
        
        ctx2.finish().await;
        ctx1.finish().await;
    }

    #[tokio::test]
    async fn test_error_statistics() {
        let manager = ErrorContextManager::new();
        
        // 记录不同类型的错误
        let auth_error = GeminiProxyError::authentication("认证失败");
        let config_error = GeminiProxyError::config("配置错误");
        let storage_error = GeminiProxyError::storage("存储错误");
        
        manager.record_error(&auth_error).await;
        manager.record_error(&config_error).await;
        manager.record_error(&storage_error).await;
        
        let stats = manager.get_error_statistics().await;
        assert_eq!(stats.total_errors, 3);
        assert!(stats.by_type.contains_key("Authentication"));
        assert!(stats.by_type.contains_key("Config"));
        assert!(stats.by_type.contains_key("Storage"));
    }
}