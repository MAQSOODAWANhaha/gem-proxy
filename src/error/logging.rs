// src/error/logging.rs
//! 结构化错误日志记录
//! 
//! 提供统一的错误日志记录功能，支持多种日志格式和输出目标

use super::{GeminiProxyError, ErrorContext, ErrorSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 错误日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLoggingConfig {
    /// 启用错误日志
    pub enabled: bool,
    /// 日志级别阈值
    pub level_threshold: LogLevel,
    /// 日志格式
    pub format: LogFormat,
    /// 日志输出目标
    pub outputs: Vec<LogOutput>,
    /// 是否记录错误堆栈
    pub include_stack_trace: bool,
    /// 是否记录请求上下文
    pub include_request_context: bool,
    /// 错误聚合配置
    pub aggregation: ErrorAggregationConfig,
    /// 日志轮转配置
    pub rotation: LogRotationConfig,
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
    Fatal = 5,
}

/// 日志格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    /// JSON 格式
    Json,
    /// 结构化文本格式
    Structured,
    /// 紧凑格式
    Compact,
    /// 自定义格式
    Custom(String),
}

/// 日志输出目标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    /// 控制台输出
    Console,
    /// 文件输出
    File { path: PathBuf },
    /// 系统日志
    Syslog,
    /// 远程日志服务
    Remote { endpoint: String, auth_token: Option<String> },
    /// 数据库存储
    Database { connection_string: String },
}

/// 错误聚合配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAggregationConfig {
    /// 启用错误聚合
    pub enabled: bool,
    /// 聚合时间窗口（秒）
    pub window_seconds: u64,
    /// 相同错误的最大记录次数
    pub max_same_error_count: u32,
    /// 聚合键生成策略
    pub aggregation_strategy: AggregationStrategy,
}

/// 聚合策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationStrategy {
    /// 按错误类型聚合
    ByErrorType,
    /// 按组件聚合
    ByComponent,
    /// 按错误类型和组件聚合
    ByErrorTypeAndComponent,
    /// 按错误消息聚合
    ByMessage,
    /// 自定义聚合策略
    Custom(Vec<String>),
}

/// 日志轮转配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// 启用日志轮转
    pub enabled: bool,
    /// 最大文件大小（字节）
    pub max_file_size: u64,
    /// 保留的日志文件数量
    pub max_files: u32,
    /// 轮转时间间隔（小时）
    pub rotation_hours: Option<u64>,
}

/// 错误日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLogEntry {
    /// 日志ID
    pub log_id: String,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 错误信息
    pub error: LoggedError,
    /// 日志级别
    pub level: LogLevel,
    /// 主机信息
    pub host: String,
    /// 进程信息
    pub process: ProcessInfo,
    /// 环境信息
    pub environment: Option<String>,
}

/// 被记录的错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggedError {
    /// 错误ID
    pub error_id: String,
    /// 错误类型
    pub error_type: String,
    /// 错误消息
    pub message: String,
    /// 错误上下文
    pub context: ErrorContext,
    /// 错误链
    pub error_chain: Vec<String>,
    /// 堆栈跟踪
    pub stack_trace: Option<String>,
    /// 错误分类
    pub category: String,
    /// 是否为用户错误
    pub is_user_error: bool,
}

/// 进程信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub thread_id: String,
    pub version: String,
}

/// 错误聚合信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAggregation {
    /// 聚合键
    pub aggregation_key: String,
    /// 首次发生时间
    pub first_occurrence: chrono::DateTime<chrono::Utc>,
    /// 最后发生时间
    pub last_occurrence: chrono::DateTime<chrono::Utc>,
    /// 发生次数
    pub count: u64,
    /// 示例错误
    pub sample_error: LoggedError,
    /// 影响的组件
    pub affected_components: Vec<String>,
    /// 影响的用户数
    pub affected_users: u64,
}

/// 错误日志记录器
pub struct ErrorLogger {
    config: ErrorLoggingConfig,
    aggregations: Arc<RwLock<HashMap<String, ErrorAggregation>>>,
    stats: Arc<RwLock<ErrorLoggingStats>>,
}

/// 错误日志统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorLoggingStats {
    pub total_errors_logged: u64,
    pub errors_by_level: HashMap<String, u64>,
    pub errors_by_component: HashMap<String, u64>,
    pub aggregated_errors: u64,
    pub suppressed_errors: u64,
    pub logging_errors: u64,
}

impl Default for ErrorLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level_threshold: LogLevel::Warning,
            format: LogFormat::Json,
            outputs: vec![LogOutput::Console],
            include_stack_trace: false,
            include_request_context: true,
            aggregation: ErrorAggregationConfig {
                enabled: true,
                window_seconds: 300, // 5分钟
                max_same_error_count: 10,
                aggregation_strategy: AggregationStrategy::ByErrorTypeAndComponent,
            },
            rotation: LogRotationConfig {
                enabled: false,
                max_file_size: 100 * 1024 * 1024, // 100MB
                max_files: 5,
                rotation_hours: Some(24),
            },
        }
    }
}

impl ErrorLogger {
    /// 创建新的错误日志记录器
    pub fn new(config: ErrorLoggingConfig) -> Self {
        Self {
            config,
            aggregations: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ErrorLoggingStats::default())),
        }
    }

    /// 记录错误
    pub async fn log_error(&self, error: &GeminiProxyError) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        // 检查日志级别阈值
        let error_level = self.severity_to_log_level(&error.get_context().severity);
        if error_level < self.config.level_threshold {
            return Ok(());
        }

        // 检查是否需要聚合
        if self.config.aggregation.enabled {
            if let Some(agg_key) = self.generate_aggregation_key(error).await {
                if self.should_suppress_error(&agg_key).await {
                    self.update_aggregation(&agg_key, error).await?;
                    self.increment_stat("suppressed_errors").await;
                    return Ok(());
                }
            }
        }

        // 创建日志条目
        let log_entry = self.create_log_entry(error).await?;

        // 输出到各个目标
        for output in &self.config.outputs {
            if let Err(e) = self.write_to_output(&log_entry, output).await {
                self.increment_stat("logging_errors").await;
                eprintln!("日志输出失败: {}", e);
            }
        }

        // 更新统计信息
        self.update_stats(error).await;

        Ok(())
    }

    /// 记录聚合的错误摘要
    pub async fn log_aggregated_errors(&self) -> Result<(), String> {
        let aggregations = self.aggregations.read().await;
        
        for (agg_key, aggregation) in aggregations.iter() {
            if aggregation.count > 1 {
                let summary = self.create_aggregation_summary(agg_key, aggregation).await?;
                
                for output in &self.config.outputs {
                    if let Err(e) = self.write_aggregation_to_output(&summary, output).await {
                        eprintln!("聚合日志输出失败: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取错误日志统计
    pub async fn get_statistics(&self) -> ErrorLoggingStats {
        self.stats.read().await.clone()
    }

    /// 清理过期的聚合数据
    pub async fn cleanup_expired_aggregations(&self) -> Result<(), String> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::seconds(self.config.aggregation.window_seconds as i64);
        
        let mut aggregations = self.aggregations.write().await;
        aggregations.retain(|_, agg| agg.last_occurrence > cutoff_time);
        
        Ok(())
    }

    // 私有辅助方法

    async fn generate_aggregation_key(&self, error: &GeminiProxyError) -> Option<String> {
        match &self.config.aggregation.aggregation_strategy {
            AggregationStrategy::ByErrorType => {
                Some(error.error_type_name().to_string())
            }
            AggregationStrategy::ByComponent => {
                Some(error.get_context().component.clone())
            }
            AggregationStrategy::ByErrorTypeAndComponent => {
                Some(format!("{}:{}", error.error_type_name(), error.get_context().component))
            }
            AggregationStrategy::ByMessage => {
                Some(error.to_string())
            }
            AggregationStrategy::Custom(fields) => {
                let mut key_parts = Vec::new();
                for field in fields {
                    match field.as_str() {
                        "error_type" => key_parts.push(error.error_type_name().to_string()),
                        "component" => key_parts.push(error.get_context().component.clone()),
                        "operation" => key_parts.push(error.get_context().operation.clone()),
                        "severity" => key_parts.push(format!("{:?}", error.get_context().severity)),
                        _ => {}
                    }
                }
                if key_parts.is_empty() {
                    None
                } else {
                    Some(key_parts.join(":"))
                }
            }
        }
    }

    async fn should_suppress_error(&self, agg_key: &str) -> bool {
        let aggregations = self.aggregations.read().await;
        if let Some(agg) = aggregations.get(agg_key) {
            agg.count >= self.config.aggregation.max_same_error_count as u64
        } else {
            false
        }
    }

    async fn update_aggregation(&self, agg_key: &str, error: &GeminiProxyError) -> Result<(), String> {
        let mut aggregations = self.aggregations.write().await;
        let now = chrono::Utc::now();
        
        if let Some(agg) = aggregations.get_mut(agg_key) {
            agg.count += 1;
            agg.last_occurrence = now;
            
            // 更新影响的组件
            if !agg.affected_components.contains(&error.get_context().component) {
                agg.affected_components.push(error.get_context().component.clone());
            }
            
            // 更新影响的用户数（如果有用户ID）
            if error.get_context().user_id.is_some() {
                agg.affected_users += 1;
            }
        } else {
            // 创建新的聚合条目
            let logged_error = self.convert_to_logged_error(error).await?;
            aggregations.insert(agg_key.to_string(), ErrorAggregation {
                aggregation_key: agg_key.to_string(),
                first_occurrence: now,
                last_occurrence: now,
                count: 1,
                sample_error: logged_error,
                affected_components: vec![error.get_context().component.clone()],
                affected_users: if error.get_context().user_id.is_some() { 1 } else { 0 },
            });
        }
        
        Ok(())
    }

    async fn create_log_entry(&self, error: &GeminiProxyError) -> Result<ErrorLogEntry, String> {
        let logged_error = self.convert_to_logged_error(error).await?;
        
        Ok(ErrorLogEntry {
            log_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            error: logged_error,
            level: self.severity_to_log_level(&error.get_context().severity),
            host: hostname::get()
                .map_err(|e| format!("获取主机名失败: {}", e))?
                .to_string_lossy()
                .to_string(),
            process: ProcessInfo {
                pid: std::process::id(),
                thread_id: format!("{:?}", std::thread::current().id()),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            environment: std::env::var("RUST_ENV").ok(),
        })
    }

    async fn convert_to_logged_error(&self, error: &GeminiProxyError) -> Result<LoggedError, String> {
        Ok(LoggedError {
            error_id: error.get_context().error_id.clone(),
            error_type: error.error_type_name().to_string(),
            message: error.to_string(),
            context: error.get_context().clone(),
            error_chain: error.error_chain().iter().map(|e| e.to_string()).collect(),
            stack_trace: if self.config.include_stack_trace {
                Some(std::backtrace::Backtrace::capture().to_string())
            } else {
                None
            },
            category: format!("{:?}", error.category()),
            is_user_error: error.is_user_error(),
        })
    }

    fn severity_to_log_level(&self, severity: &ErrorSeverity) -> LogLevel {
        match severity {
            ErrorSeverity::Debug => LogLevel::Debug,
            ErrorSeverity::Info => LogLevel::Info,
            ErrorSeverity::Warning => LogLevel::Warning,
            ErrorSeverity::Error => LogLevel::Error,
            ErrorSeverity::Critical => LogLevel::Critical,
            ErrorSeverity::Fatal => LogLevel::Fatal,
        }
    }

    async fn write_to_output(&self, log_entry: &ErrorLogEntry, output: &LogOutput) -> Result<(), String> {
        let formatted = self.format_log_entry(log_entry).await?;
        
        match output {
            LogOutput::Console => {
                match log_entry.level {
                    LogLevel::Debug => debug!("{}", formatted),
                    LogLevel::Info => info!("{}", formatted),
                    LogLevel::Warning => warn!("{}", formatted),
                    LogLevel::Error | LogLevel::Critical | LogLevel::Fatal => error!("{}", formatted),
                }
            }
            LogOutput::File { path } => {
                tokio::fs::write(path, formatted.as_bytes()).await
                    .map_err(|e| format!("写入文件失败: {}", e))?;
            }
            LogOutput::Syslog => {
                // 这里可以集成 syslog 库
                return Err("Syslog 输出尚未实现".to_string());
            }
            LogOutput::Remote { .. } => {
                // 这里可以集成远程日志服务
                return Err("远程日志输出尚未实现".to_string());
            }
            LogOutput::Database { .. } => {
                // 这里可以集成数据库存储
                return Err("数据库日志输出尚未实现".to_string());
            }
        }
        
        Ok(())
    }

    async fn format_log_entry(&self, log_entry: &ErrorLogEntry) -> Result<String, String> {
        match &self.config.format {
            LogFormat::Json => {
                serde_json::to_string_pretty(log_entry)
                    .map_err(|e| format!("JSON序列化失败: {}", e))
            }
            LogFormat::Structured => {
                Ok(format!(
                    "[{}] [{}] [{}:{}] {} - {} (ID: {})",
                    log_entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
                    format!("{:?}", log_entry.level).to_uppercase(),
                    log_entry.error.context.component,
                    log_entry.error.context.operation,
                    log_entry.error.error_type,
                    log_entry.error.message,
                    log_entry.error.error_id
                ))
            }
            LogFormat::Compact => {
                Ok(format!(
                    "{} [{}] {} - {}",
                    log_entry.timestamp.format("%H:%M:%S"),
                    format!("{:?}", log_entry.level),
                    log_entry.error.error_type,
                    log_entry.error.message
                ))
            }
            LogFormat::Custom(template) => {
                // 这里可以实现自定义模板格式化
                Ok(template.clone())
            }
        }
    }

    async fn create_aggregation_summary(&self, agg_key: &str, aggregation: &ErrorAggregation) -> Result<String, String> {
        let summary = serde_json::json!({
            "type": "error_aggregation_summary",
            "aggregation_key": agg_key,
            "count": aggregation.count,
            "first_occurrence": aggregation.first_occurrence,
            "last_occurrence": aggregation.last_occurrence,
            "sample_error": aggregation.sample_error,
            "affected_components": aggregation.affected_components,
            "affected_users": aggregation.affected_users
        });
        
        serde_json::to_string_pretty(&summary)
            .map_err(|e| format!("聚合摘要序列化失败: {}", e))
    }

    async fn write_aggregation_to_output(&self, summary: &str, output: &LogOutput) -> Result<(), String> {
        match output {
            LogOutput::Console => {
                warn!("ERROR_AGGREGATION: {}", summary);
            }
            LogOutput::File { path } => {
                let mut file_path = path.clone();
                if let Some(file_stem) = file_path.file_stem() {
                    let new_name = format!("{}_aggregation.log", file_stem.to_string_lossy());
                    file_path.set_file_name(new_name);
                }
                
                tokio::fs::write(file_path, summary.as_bytes()).await
                    .map_err(|e| format!("写入聚合文件失败: {}", e))?;
            }
            _ => {
                // 其他输出类型使用相同的逻辑
                return self.write_to_output(&ErrorLogEntry {
                    log_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    error: LoggedError {
                        error_id: "aggregation".to_string(),
                        error_type: "ErrorAggregation".to_string(),
                        message: summary.to_string(),
                        context: super::ErrorContext::new("error_logger", "aggregation"),
                        error_chain: vec![],
                        stack_trace: None,
                        category: "SystemError".to_string(),
                        is_user_error: false,
                    },
                    level: LogLevel::Info,
                    host: "localhost".to_string(),
                    process: ProcessInfo {
                        pid: std::process::id(),
                        thread_id: "main".to_string(),
                        version: "1.0.0".to_string(),
                    },
                    environment: None,
                }, output).await;
            }
        }
        
        Ok(())
    }

    async fn update_stats(&self, error: &GeminiProxyError) {
        let mut stats = self.stats.write().await;
        stats.total_errors_logged += 1;
        
        let level_key = format!("{:?}", self.severity_to_log_level(&error.get_context().severity));
        *stats.errors_by_level.entry(level_key).or_insert(0) += 1;
        
        let component_key = error.get_context().component.clone();
        *stats.errors_by_component.entry(component_key).or_insert(0) += 1;
    }

    async fn increment_stat(&self, stat_name: &str) {
        let mut stats = self.stats.write().await;
        match stat_name {
            "aggregated_errors" => stats.aggregated_errors += 1,
            "suppressed_errors" => stats.suppressed_errors += 1,
            "logging_errors" => stats.logging_errors += 1,
            _ => {}
        }
    }
}

/// 错误日志查询接口
pub struct ErrorLogQuery {
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub level: Option<LogLevel>,
    pub component: Option<String>,
    pub error_type: Option<String>,
    pub user_id: Option<String>,
    pub limit: Option<usize>,
}

/// 创建默认的错误日志记录器
pub fn create_default_logger() -> ErrorLogger {
    ErrorLogger::new(ErrorLoggingConfig::default())
}

/// 创建用于生产环境的错误日志记录器
pub fn create_production_logger(log_dir: PathBuf) -> ErrorLogger {
    let config = ErrorLoggingConfig {
        enabled: true,
        level_threshold: LogLevel::Warning,
        format: LogFormat::Json,
        outputs: vec![
            LogOutput::Console,
            LogOutput::File { path: log_dir.join("errors.log") },
        ],
        include_stack_trace: true,
        include_request_context: true,
        aggregation: ErrorAggregationConfig {
            enabled: true,
            window_seconds: 300,
            max_same_error_count: 5,
            aggregation_strategy: AggregationStrategy::ByErrorTypeAndComponent,
        },
        rotation: LogRotationConfig {
            enabled: true,
            max_file_size: 50 * 1024 * 1024, // 50MB
            max_files: 10,
            rotation_hours: Some(24),
        },
    };
    
    ErrorLogger::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GeminiProxyError;

    #[tokio::test]
    async fn test_error_logger_basic() {
        let logger = create_default_logger();
        
        let error = GeminiProxyError::config("测试错误")
            .with_severity(super::ErrorSeverity::Error);
        
        assert!(logger.log_error(&error).await.is_ok());
        
        let stats = logger.get_statistics().await;
        assert_eq!(stats.total_errors_logged, 1);
    }

    #[tokio::test]
    async fn test_error_aggregation() {
        let mut config = ErrorLoggingConfig::default();
        config.aggregation.max_same_error_count = 2;
        
        let logger = ErrorLogger::new(config);
        
        // 记录相同的错误多次
        for _ in 0..5 {
            let error = GeminiProxyError::config("重复错误")
                .with_severity(super::ErrorSeverity::Error);
            logger.log_error(&error).await.unwrap();
        }
        
        let stats = logger.get_statistics().await;
        assert!(stats.suppressed_errors > 0);
    }

    #[tokio::test]
    async fn test_log_level_filtering() {
        let mut config = ErrorLoggingConfig::default();
        config.level_threshold = LogLevel::Error;
        
        let logger = ErrorLogger::new(config);
        
        // 这个错误应该被过滤掉
        let warning_error = GeminiProxyError::config("警告错误")
            .with_severity(super::ErrorSeverity::Warning);
        logger.log_error(&warning_error).await.unwrap();
        
        // 这个错误应该被记录
        let error = GeminiProxyError::config("错误")
            .with_severity(super::ErrorSeverity::Error);
        logger.log_error(&error).await.unwrap();
        
        let stats = logger.get_statistics().await;
        assert_eq!(stats.total_errors_logged, 1);
    }
}