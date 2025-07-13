// src/security/audit_logging.rs
//! 安全审计日志模块
//! 
//! 提供API调用审计、配置变更追踪、安全事件监控等功能

use crate::error::GeminiProxyError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// 审计事件类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditEventType {
    /// API调用
    ApiCall,
    /// 配置变更
    ConfigChange,
    /// 认证事件
    Authentication,
    /// 安全事件
    SecurityEvent,
    /// 系统操作
    SystemOperation,
    /// 错误事件
    ErrorEvent,
}

/// 审计事件严重性
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditSeverity {
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 严重
    Critical,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// 事件ID
    pub id: String,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 事件类型
    pub event_type: AuditEventType,
    /// 严重性
    pub severity: AuditSeverity,
    /// 源IP地址
    pub source_ip: Option<IpAddr>,
    /// 用户标识（JWT sub或API key ID）
    pub user_identifier: Option<String>,
    /// 操作描述
    pub action: String,
    /// 资源路径
    pub resource: String,
    /// 请求方法（HTTP方法）
    pub method: Option<String>,
    /// 响应状态码
    pub status_code: Option<u16>,
    /// 处理时间（毫秒）
    pub duration_ms: Option<u64>,
    /// 附加数据
    pub metadata: HashMap<String, String>,
    /// 结果
    pub result: AuditResult,
    /// 详细信息
    pub details: Option<String>,
}

/// 审计结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditResult {
    /// 成功
    Success,
    /// 失败
    Failure,
    /// 被拒绝
    Denied,
    /// 限流
    RateLimited,
    /// 超时
    Timeout,
}

/// 安全事件详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// 事件类型
    pub event_type: String,
    /// 威胁级别
    pub threat_level: String,
    /// 检测规则
    pub detection_rule: String,
    /// 影响描述
    pub impact: String,
    /// 建议操作
    pub recommended_action: String,
}

/// 审计日志管理器
pub struct AuditLogManager {
    /// 内存日志缓冲区
    log_buffer: VecDeque<AuditLogEntry>,
    /// 日志输出配置
    config: AuditConfig,
    /// 统计信息
    statistics: AuditStatistics,
    /// IP访问统计
    ip_statistics: HashMap<IpAddr, IpAccessStats>,
}

/// 审计配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// 是否启用文件输出
    pub file_output_enabled: bool,
    /// 日志文件路径
    pub log_file_path: String,
    /// 最大文件大小（MB）
    pub max_file_size_mb: u64,
    /// 日志保留天数
    pub retention_days: u32,
    /// 内存缓冲区大小
    pub buffer_size: usize,
    /// 是否启用实时监控
    pub realtime_monitoring: bool,
    /// 安全事件阈值
    pub security_thresholds: SecurityThresholds,
}

/// 安全阈值配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityThresholds {
    /// 每分钟最大失败登录次数
    pub max_auth_failures_per_minute: u32,
    /// 每小时最大API调用次数（单IP）
    pub max_api_calls_per_hour: u32,
    /// 配置变更间隔最小时间（秒）
    pub min_config_change_interval_seconds: u64,
    /// 异常响应时间阈值（毫秒）
    pub anomaly_response_time_ms: u64,
}

/// 审计统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_events: u64,
    pub api_calls: u64,
    pub config_changes: u64,
    pub auth_events: u64,
    pub security_events: u64,
    pub error_events: u64,
    pub failed_requests: u64,
    pub blocked_requests: u64,
    pub unique_ips: usize,
    pub last_reset: chrono::DateTime<chrono::Utc>,
}

/// IP访问统计
#[derive(Debug, Clone)]
struct IpAccessStats {
    total_requests: u64,
    failed_requests: u64,
    last_request: SystemTime,
    first_seen: SystemTime,
    blocked: bool,
}

impl Default for IpAccessStats {
    fn default() -> Self {
        let now = SystemTime::now();
        Self {
            total_requests: 0,
            failed_requests: 0,
            last_request: now,
            first_seen: now,
            blocked: false,
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            file_output_enabled: true,
            log_file_path: "logs/audit.log".to_string(),
            max_file_size_mb: 100,
            retention_days: 30,
            buffer_size: 10000,
            realtime_monitoring: true,
            security_thresholds: SecurityThresholds::default(),
        }
    }
}

impl Default for SecurityThresholds {
    fn default() -> Self {
        Self {
            max_auth_failures_per_minute: 10,
            max_api_calls_per_hour: 1000,
            min_config_change_interval_seconds: 60,
            anomaly_response_time_ms: 5000,
        }
    }
}

impl AuditLogManager {
    /// 创建新的审计日志管理器
    pub fn new(config: AuditConfig) -> Self {
        Self {
            log_buffer: VecDeque::with_capacity(config.buffer_size),
            config,
            statistics: AuditStatistics::default(),
            ip_statistics: HashMap::new(),
        }
    }

    /// 记录API调用
    pub async fn log_api_call(
        &mut self,
        source_ip: IpAddr,
        user_id: Option<String>,
        method: &str,
        resource: &str,
        status_code: u16,
        duration_ms: u64,
        result: AuditResult,
    ) -> Result<(), GeminiProxyError> {
        let mut metadata = HashMap::new();
        metadata.insert("user_agent".to_string(), "gemini-proxy".to_string());
        
        let entry = AuditLogEntry {
            id: self.generate_event_id(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::ApiCall,
            severity: if status_code >= 400 { AuditSeverity::Warning } else { AuditSeverity::Info },
            source_ip: Some(source_ip),
            user_identifier: user_id,
            action: format!("{} {}", method, resource),
            resource: resource.to_string(),
            method: Some(method.to_string()),
            status_code: Some(status_code),
            duration_ms: Some(duration_ms),
            metadata,
            result,
            details: None,
        };

        self.add_log_entry(entry).await?;
        self.update_ip_statistics(source_ip, status_code >= 400);
        
        // 检查异常响应时间
        if duration_ms > self.config.security_thresholds.anomaly_response_time_ms {
            self.log_security_event(
                source_ip,
                "异常响应时间检测",
                &format!("请求响应时间 {}ms 超过阈值", duration_ms),
                "Warning",
            ).await?;
        }

        Ok(())
    }

    /// 记录配置变更
    pub async fn log_config_change(
        &mut self,
        source_ip: Option<IpAddr>,
        user_id: Option<String>,
        config_section: &str,
        old_value: &str,
        new_value: &str,
        change_type: &str,
    ) -> Result<(), GeminiProxyError> {
        let mut metadata = HashMap::new();
        metadata.insert("config_section".to_string(), config_section.to_string());
        metadata.insert("old_value".to_string(), old_value.to_string());
        metadata.insert("new_value".to_string(), new_value.to_string());
        metadata.insert("change_type".to_string(), change_type.to_string());

        let entry = AuditLogEntry {
            id: self.generate_event_id(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::ConfigChange,
            severity: AuditSeverity::Warning,
            source_ip,
            user_identifier: user_id,
            action: format!("配置变更: {}", change_type),
            resource: config_section.to_string(),
            method: Some("CONFIG_UPDATE".to_string()),
            status_code: None,
            duration_ms: None,
            metadata,
            result: AuditResult::Success,
            details: Some(format!("从 '{}' 更改为 '{}'", old_value, new_value)),
        };

        self.add_log_entry(entry).await
    }

    /// 记录认证事件
    pub async fn log_auth_event(
        &mut self,
        source_ip: IpAddr,
        user_id: Option<String>,
        auth_type: &str,
        result: AuditResult,
        details: Option<String>,
    ) -> Result<(), GeminiProxyError> {
        let mut metadata = HashMap::new();
        metadata.insert("auth_type".to_string(), auth_type.to_string());

        let severity = match result {
            AuditResult::Success => AuditSeverity::Info,
            AuditResult::Failure | AuditResult::Denied => AuditSeverity::Warning,
            _ => AuditSeverity::Error,
        };

        let entry = AuditLogEntry {
            id: self.generate_event_id(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::Authentication,
            severity,
            source_ip: Some(source_ip),
            user_identifier: user_id.clone(),
            action: format!("认证尝试: {}", auth_type),
            resource: "/auth".to_string(),
            method: Some("AUTH".to_string()),
            status_code: None,
            duration_ms: None,
            metadata,
            result: result.clone(),
            details,
        };

        self.add_log_entry(entry).await?;

        // 检查认证失败频率
        if result == AuditResult::Failure {
            self.check_auth_failure_threshold(source_ip).await?;
        }

        Ok(())
    }

    /// 记录安全事件
    pub async fn log_security_event(
        &mut self,
        source_ip: IpAddr,
        event_description: &str,
        details: &str,
        threat_level: &str,
    ) -> Result<(), GeminiProxyError> {
        let mut metadata = HashMap::new();
        metadata.insert("threat_level".to_string(), threat_level.to_string());
        metadata.insert("detection_time".to_string(), chrono::Utc::now().to_rfc3339());

        let severity = match threat_level {
            "Critical" => AuditSeverity::Critical,
            "Warning" => AuditSeverity::Warning,
            _ => AuditSeverity::Info,
        };

        let entry = AuditLogEntry {
            id: self.generate_event_id(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SecurityEvent,
            severity,
            source_ip: Some(source_ip),
            user_identifier: None,
            action: format!("安全事件: {}", event_description),
            resource: "/security".to_string(),
            method: Some("SECURITY_DETECTION".to_string()),
            status_code: None,
            duration_ms: None,
            metadata,
            result: AuditResult::Failure,
            details: Some(details.to_string()),
        };

        self.add_log_entry(entry).await
    }

    /// 记录系统操作
    pub async fn log_system_operation(
        &mut self,
        operation: &str,
        component: &str,
        result: AuditResult,
        details: Option<String>,
    ) -> Result<(), GeminiProxyError> {
        let mut metadata = HashMap::new();
        metadata.insert("component".to_string(), component.to_string());

        let entry = AuditLogEntry {
            id: self.generate_event_id(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SystemOperation,
            severity: match result {
                AuditResult::Success => AuditSeverity::Info,
                _ => AuditSeverity::Warning,
            },
            source_ip: None,
            user_identifier: Some("system".to_string()),
            action: format!("系统操作: {}", operation),
            resource: component.to_string(),
            method: Some("SYSTEM".to_string()),
            status_code: None,
            duration_ms: None,
            metadata,
            result,
            details,
        };

        self.add_log_entry(entry).await
    }

    /// 添加日志条目
    async fn add_log_entry(&mut self, entry: AuditLogEntry) -> Result<(), GeminiProxyError> {
        // 更新统计信息
        self.update_statistics(&entry);

        // 添加到缓冲区
        if self.log_buffer.len() >= self.config.buffer_size {
            self.log_buffer.pop_front();
        }
        self.log_buffer.push_back(entry.clone());

        // 写入文件（如果启用）
        if self.config.file_output_enabled {
            self.write_to_file(&entry).await?;
        }

        Ok(())
    }

    /// 写入文件
    async fn write_to_file(&self, entry: &AuditLogEntry) -> Result<(), GeminiProxyError> {
        use tokio::io::AsyncWriteExt;

        let log_line = format!("{}\n", serde_json::to_string(entry)
            .map_err(|e| GeminiProxyError::storage(format!("序列化日志失败: {}", e)))?);

        // 确保日志目录存在
        if let Some(parent) = Path::new(&self.config.log_file_path).parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| GeminiProxyError::storage(format!("创建日志目录失败: {}", e)))?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file_path)
            .await
            .map_err(|e| GeminiProxyError::storage(format!("打开日志文件失败: {}", e)))?;

        file.write_all(log_line.as_bytes()).await
            .map_err(|e| GeminiProxyError::storage(format!("写入日志文件失败: {}", e)))?;

        Ok(())
    }

    /// 更新统计信息
    fn update_statistics(&mut self, entry: &AuditLogEntry) {
        self.statistics.total_events += 1;

        match entry.event_type {
            AuditEventType::ApiCall => self.statistics.api_calls += 1,
            AuditEventType::ConfigChange => self.statistics.config_changes += 1,
            AuditEventType::Authentication => self.statistics.auth_events += 1,
            AuditEventType::SecurityEvent => self.statistics.security_events += 1,
            AuditEventType::ErrorEvent => self.statistics.error_events += 1,
            _ => {}
        }

        if entry.result == AuditResult::Failure || entry.result == AuditResult::Denied {
            self.statistics.failed_requests += 1;
        }

        if let Some(ip) = entry.source_ip {
            if !self.ip_statistics.contains_key(&ip) {
                self.statistics.unique_ips += 1;
            }
        }
    }

    /// 更新IP统计
    fn update_ip_statistics(&mut self, ip: IpAddr, failed: bool) {
        let stats = self.ip_statistics.entry(ip).or_insert_with(|| IpAccessStats {
            total_requests: 0,
            failed_requests: 0,
            last_request: SystemTime::now(),
            first_seen: SystemTime::now(),
            blocked: false,
        });

        stats.total_requests += 1;
        if failed {
            stats.failed_requests += 1;
        }
        stats.last_request = SystemTime::now();
    }

    /// 检查认证失败阈值
    async fn check_auth_failure_threshold(&mut self, source_ip: IpAddr) -> Result<(), GeminiProxyError> {
        let now = SystemTime::now();
        let threshold_window = Duration::from_secs(60); // 1分钟窗口
        
        // 计算过去一分钟内的认证失败次数
        let recent_failures = self.log_buffer
            .iter()
            .filter(|entry| {
                entry.event_type == AuditEventType::Authentication
                    && entry.result == AuditResult::Failure
                    && entry.source_ip == Some(source_ip)
                    && now.duration_since(
                        SystemTime::UNIX_EPOCH + Duration::from_secs(entry.timestamp.timestamp() as u64)
                    ).unwrap_or(Duration::MAX) <= threshold_window
            })
            .count();

        if recent_failures >= self.config.security_thresholds.max_auth_failures_per_minute as usize {
            self.log_security_event(
                source_ip,
                "认证失败次数超过阈值",
                &format!("IP {} 在1分钟内认证失败 {} 次", source_ip, recent_failures),
                "Critical",
            ).await?;
        }

        Ok(())
    }

    /// 生成事件ID
    fn generate_event_id(&self) -> String {
        format!("audit_{}", uuid::Uuid::new_v4().to_string())
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> &AuditStatistics {
        &self.statistics
    }

    /// 获取最近的日志条目
    pub fn get_recent_logs(&self, limit: usize) -> Vec<&AuditLogEntry> {
        self.log_buffer
            .iter()
            .rev()
            .take(limit)
            .collect()
    }

    /// 按事件类型筛选日志
    pub fn get_logs_by_type(&self, event_type: AuditEventType, limit: usize) -> Vec<&AuditLogEntry> {
        self.log_buffer
            .iter()
            .rev()
            .filter(|entry| entry.event_type == event_type)
            .take(limit)
            .collect()
    }

    /// 获取安全事件汇总
    pub fn get_security_summary(&self) -> SecuritySummary {
        let recent_time = chrono::Utc::now() - chrono::Duration::hours(24);
        
        let recent_security_events = self.log_buffer
            .iter()
            .filter(|entry| {
                entry.event_type == AuditEventType::SecurityEvent
                    && entry.timestamp >= recent_time
            })
            .count();

        let recent_auth_failures = self.log_buffer
            .iter()
            .filter(|entry| {
                entry.event_type == AuditEventType::Authentication
                    && entry.result == AuditResult::Failure
                    && entry.timestamp >= recent_time
            })
            .count();

        SecuritySummary {
            recent_security_events,
            recent_auth_failures,
            total_unique_ips: self.statistics.unique_ips,
            blocked_ips: self.ip_statistics.values().filter(|stats| stats.blocked).count(),
        }
    }

    /// 清理过期日志
    pub async fn cleanup_expired_logs(&mut self) -> Result<(), GeminiProxyError> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(self.config.retention_days as i64);
        
        // 清理内存缓冲区中的过期日志
        self.log_buffer.retain(|entry| entry.timestamp >= cutoff_date);
        
        // 清理过期的IP统计
        let cutoff_time = SystemTime::now() - Duration::from_secs(86400 * self.config.retention_days as u64);
        self.ip_statistics.retain(|_, stats| stats.last_request >= cutoff_time);

        Ok(())
    }
}

/// 安全汇总信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySummary {
    pub recent_security_events: usize,
    pub recent_auth_failures: usize,
    pub total_unique_ips: usize,
    pub blocked_ips: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_audit_logging() {
        let config = AuditConfig::default();
        let mut manager = AuditLogManager::new(config);
        
        let test_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        
        // 测试API调用日志
        manager.log_api_call(
            test_ip,
            Some("test_user".to_string()),
            "GET",
            "/api/test",
            200,
            150,
            AuditResult::Success,
        ).await.unwrap();
        
        // 测试认证事件日志
        manager.log_auth_event(
            test_ip,
            Some("test_user".to_string()),
            "JWT",
            AuditResult::Success,
            None,
        ).await.unwrap();
        
        // 验证统计信息
        let stats = manager.get_statistics();
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.api_calls, 1);
        assert_eq!(stats.auth_events, 1);
    }

    #[tokio::test]
    async fn test_security_event_logging() {
        let config = AuditConfig::default();
        let mut manager = AuditLogManager::new(config);
        
        let test_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        
        manager.log_security_event(
            test_ip,
            "可疑访问模式",
            "检测到异常高频访问",
            "Warning",
        ).await.unwrap();
        
        let security_logs = manager.get_logs_by_type(AuditEventType::SecurityEvent, 10);
        assert_eq!(security_logs.len(), 1);
    }

    #[tokio::test]
    async fn test_config_change_logging() {
        let config = AuditConfig::default();
        let mut manager = AuditLogManager::new(config);
        
        manager.log_config_change(
            Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            Some("admin".to_string()),
            "server.port",
            "8080",
            "8443",
            "update",
        ).await.unwrap();
        
        let config_logs = manager.get_logs_by_type(AuditEventType::ConfigChange, 10);
        assert_eq!(config_logs.len(), 1);
        assert_eq!(config_logs[0].action, "配置变更: update");
    }
}