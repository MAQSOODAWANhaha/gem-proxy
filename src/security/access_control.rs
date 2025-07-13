// src/security/access_control.rs
//! 访问控制安全管理
//! 
//! 提供基于角色的访问控制（RBAC）、IP白名单、API权限管理等功能

use crate::error::{GeminiProxyError, ErrorSeverity};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::time::{Duration, SystemTime};

/// 用户角色
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Role {
    /// 超级管理员
    SuperAdmin,
    /// 管理员
    Admin,
    /// 操作员
    Operator,
    /// 只读用户
    ReadOnly,
    /// API用户
    ApiUser,
}

/// 权限类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// 查看配置
    ConfigRead,
    /// 修改配置
    ConfigWrite,
    /// 查看监控指标
    MetricsRead,
    /// 管理API密钥
    ApiKeyManage,
    /// 查看API密钥
    ApiKeyRead,
    /// 管理用户
    UserManage,
    /// 查看审计日志
    AuditLogRead,
    /// 系统管理
    SystemAdmin,
    /// 代理访问
    ProxyAccess,
}

impl Role {
    /// 获取角色对应的权限集合
    pub fn get_permissions(&self) -> HashSet<Permission> {
        match self {
            Role::SuperAdmin => {
                // 超级管理员拥有所有权限
                [
                    Permission::ConfigRead,
                    Permission::ConfigWrite,
                    Permission::MetricsRead,
                    Permission::ApiKeyManage,
                    Permission::ApiKeyRead,
                    Permission::UserManage,
                    Permission::AuditLogRead,
                    Permission::SystemAdmin,
                    Permission::ProxyAccess,
                ].into_iter().collect()
            }
            Role::Admin => {
                // 管理员拥有大部分权限
                [
                    Permission::ConfigRead,
                    Permission::ConfigWrite,
                    Permission::MetricsRead,
                    Permission::ApiKeyManage,
                    Permission::ApiKeyRead,
                    Permission::AuditLogRead,
                    Permission::ProxyAccess,
                ].into_iter().collect()
            }
            Role::Operator => {
                // 操作员只能查看和基本操作
                [
                    Permission::ConfigRead,
                    Permission::MetricsRead,
                    Permission::ApiKeyRead,
                    Permission::ProxyAccess,
                ].into_iter().collect()
            }
            Role::ReadOnly => {
                // 只读用户只能查看
                [
                    Permission::ConfigRead,
                    Permission::MetricsRead,
                ].into_iter().collect()
            }
            Role::ApiUser => {
                // API用户只能访问代理
                [Permission::ProxyAccess].into_iter().collect()
            }
        }
    }
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub role: Role,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
    pub allowed_ips: Option<Vec<IpAddr>>,
    pub api_quota: Option<u64>,
    pub api_usage_count: u64,
    pub quota_reset_time: chrono::DateTime<chrono::Utc>,
}

impl User {
    /// 检查用户是否有指定权限
    pub fn has_permission(&self, permission: &Permission) -> bool {
        if !self.is_active {
            return false;
        }
        self.role.get_permissions().contains(permission)
    }

    /// 检查IP地址是否允许访问
    pub fn is_ip_allowed(&self, ip: &IpAddr) -> bool {
        match &self.allowed_ips {
            Some(allowed_ips) => allowed_ips.contains(ip),
            None => true, // 如果没有限制，则允许所有IP
        }
    }

    /// 检查API配额
    pub fn check_api_quota(&self) -> bool {
        match self.api_quota {
            Some(quota) => self.api_usage_count < quota,
            None => true, // 无配额限制
        }
    }

    /// 增加API使用计数
    pub fn increment_api_usage(&mut self) {
        self.api_usage_count += 1;
    }

    /// 重置API配额（通常在配额周期重置时调用）
    pub fn reset_api_quota(&mut self) {
        self.api_usage_count = 0;
        self.quota_reset_time = chrono::Utc::now() + chrono::Duration::days(1);
    }
}

/// 访问控制策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    /// 策略名称
    pub name: String,
    /// 允许的角色
    pub allowed_roles: Vec<Role>,
    /// 允许的IP地址段（CIDR格式）
    pub allowed_ip_ranges: Vec<String>,
    /// 拒绝的IP地址段（CIDR格式）
    pub denied_ip_ranges: Vec<String>,
    /// 访问时间限制（小时）
    pub allowed_hours: Option<(u8, u8)>, // (开始小时, 结束小时)
    /// 最大并发连接数
    pub max_concurrent_connections: Option<u32>,
    /// 是否启用
    pub enabled: bool,
}

/// 访问控制管理器
pub struct AccessControlManager {
    /// 用户存储
    users: HashMap<String, User>,
    /// 访问策略
    policies: HashMap<String, AccessPolicy>,
    /// 当前活跃会话
    active_sessions: HashMap<String, SessionInfo>,
    /// IP地址访问计数
    ip_access_counts: HashMap<IpAddr, AccessCounter>,
    /// 访问日志
    access_logs: Vec<AccessLogEntry>,
}

/// 会话信息
#[derive(Debug, Clone)]
struct SessionInfo {
    user_id: String,
    ip_address: IpAddr,
    created_at: SystemTime,
    last_activity: SystemTime,
    permissions: HashSet<Permission>,
}

/// 访问计数器
#[derive(Debug, Clone)]
struct AccessCounter {
    count: u32,
    last_reset: SystemTime,
    window_start: SystemTime,
}

/// 访问日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<String>,
    pub ip_address: IpAddr,
    pub action: String,
    pub resource: String,
    pub result: AccessResult,
    pub reason: Option<String>,
}

/// 访问结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessResult {
    Allow,
    Deny,
    RateLimit,
}

impl AccessControlManager {
    /// 创建新的访问控制管理器
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            policies: HashMap::new(),
            active_sessions: HashMap::new(),
            ip_access_counts: HashMap::new(),
            access_logs: Vec::new(),
        }
    }

    /// 添加用户
    pub fn add_user(&mut self, user: User) -> Result<(), GeminiProxyError> {
        if self.users.contains_key(&user.id) {
            return Err(GeminiProxyError::config_with_context(
                "用户ID已存在",
                "access_control",
                "add_user"
            ));
        }

        self.users.insert(user.id.clone(), user);
        Ok(())
    }

    /// 验证用户访问权限
    pub fn check_access(
        &mut self,
        user_id: &str,
        ip: &IpAddr,
        action: &str,
        resource: &str,
        required_permission: &Permission,
    ) -> Result<bool, GeminiProxyError> {
        let current_time = SystemTime::now();
        
        // 检查用户是否存在
        let user = self.users.get_mut(user_id)
            .ok_or_else(|| GeminiProxyError::auth_with_context(
                "用户不存在",
                "access_control",
                "check_access"
            ))?;

        // 检查用户是否活跃
        if !user.is_active {
            self.log_access(user_id, ip, action, resource, AccessResult::Deny, Some("用户已被禁用"));
            return Ok(false);
        }

        // 检查IP地址是否允许
        if !user.is_ip_allowed(ip) {
            self.log_access(user_id, ip, action, resource, AccessResult::Deny, Some("IP地址不在允许列表中"));
            return Ok(false);
        }

        // 检查用户权限
        if !user.has_permission(required_permission) {
            self.log_access(user_id, ip, action, resource, AccessResult::Deny, Some("权限不足"));
            return Ok(false);
        }

        // 检查API配额
        if *required_permission == Permission::ProxyAccess && !user.check_api_quota() {
            self.log_access(user_id, ip, action, resource, AccessResult::RateLimit, Some("API配额已用完"));
            return Ok(false);
        }

        // 检查访问策略
        if !self.check_policies(user, ip, current_time) {
            self.log_access(user_id, ip, action, resource, AccessResult::Deny, Some("访问策略限制"));
            return Ok(false);
        }

        // 检查速率限制
        if !self.check_rate_limit(ip) {
            self.log_access(user_id, ip, action, resource, AccessResult::RateLimit, Some("访问频率过高"));
            return Ok(false);
        }

        // 更新用户状态
        if *required_permission == Permission::ProxyAccess {
            user.increment_api_usage();
        }
        user.last_login = Some(chrono::Utc::now());

        // 记录成功访问
        self.log_access(user_id, ip, action, resource, AccessResult::Allow, None);
        
        Ok(true)
    }

    /// 检查访问策略
    fn check_policies(&self, user: &User, ip: &IpAddr, _current_time: SystemTime) -> bool {
        for policy in self.policies.values() {
            if !policy.enabled {
                continue;
            }

            // 检查角色限制
            if !policy.allowed_roles.is_empty() && !policy.allowed_roles.contains(&user.role) {
                return false;
            }

            // 检查IP范围限制
            if !policy.denied_ip_ranges.is_empty() {
                for denied_range in &policy.denied_ip_ranges {
                    if self.ip_in_range(ip, denied_range) {
                        return false;
                    }
                }
            }

            if !policy.allowed_ip_ranges.is_empty() {
                let mut ip_allowed = false;
                for allowed_range in &policy.allowed_ip_ranges {
                    if self.ip_in_range(ip, allowed_range) {
                        ip_allowed = true;
                        break;
                    }
                }
                if !ip_allowed {
                    return false;
                }
            }

            // 检查时间限制
            if let Some((start_hour, end_hour)) = policy.allowed_hours {
                let current_hour = chrono::Utc::now().hour() as u8;
                if current_hour < start_hour || current_hour > end_hour {
                    return false;
                }
            }
        }

        true
    }

    /// 检查IP是否在指定范围内（简化实现）
    fn ip_in_range(&self, ip: &IpAddr, range: &str) -> bool {
        // 简化实现：只支持精确匹配和基本的子网
        if range.contains('/') {
            // CIDR格式处理（简化）
            let parts: Vec<&str> = range.split('/').collect();
            if parts.len() == 2 {
                if let Ok(network_ip) = parts[0].parse::<IpAddr>() {
                    // 简单的子网检查（实际应用中需要更复杂的逻辑）
                    match (ip, network_ip) {
                        (IpAddr::V4(ip4), IpAddr::V4(net4)) => {
                            let prefix: u8 = parts[1].parse().unwrap_or(32);
                            let mask = !((1u32 << (32 - prefix)) - 1);
                            (u32::from(*ip4) & mask) == (u32::from(net4) & mask)
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            // 精确匹配
            if let Ok(range_ip) = range.parse::<IpAddr>() {
                *ip == range_ip
            } else {
                false
            }
        }
    }

    /// 检查速率限制
    fn check_rate_limit(&mut self, ip: &IpAddr) -> bool {
        let current_time = SystemTime::now();
        let window_duration = Duration::from_secs(60); // 1分钟窗口
        let max_requests = 100; // 每分钟最大100个请求

        let counter = self.ip_access_counts.entry(*ip).or_insert(AccessCounter {
            count: 0,
            last_reset: current_time,
            window_start: current_time,
        });

        // 检查是否需要重置计数器
        if current_time.duration_since(counter.window_start).unwrap_or(Duration::ZERO) >= window_duration {
            counter.count = 0;
            counter.window_start = current_time;
        }

        counter.count += 1;
        counter.last_reset = current_time;

        counter.count <= max_requests
    }

    /// 记录访问日志
    fn log_access(
        &mut self,
        user_id: &str,
        ip: &IpAddr,
        action: &str,
        resource: &str,
        result: AccessResult,
        reason: Option<&str>,
    ) {
        let log_entry = AccessLogEntry {
            timestamp: chrono::Utc::now(),
            user_id: Some(user_id.to_string()),
            ip_address: *ip,
            action: action.to_string(),
            resource: resource.to_string(),
            result,
            reason: reason.map(|s| s.to_string()),
        };

        self.access_logs.push(log_entry);

        // 保持日志大小在合理范围内
        if self.access_logs.len() > 10000 {
            self.access_logs.drain(0..1000);
        }
    }

    /// 添加访问策略
    pub fn add_policy(&mut self, policy: AccessPolicy) -> Result<(), GeminiProxyError> {
        if self.policies.contains_key(&policy.name) {
            return Err(GeminiProxyError::config_with_context(
                "策略名称已存在",
                "access_control",
                "add_policy"
            ));
        }

        self.policies.insert(policy.name.clone(), policy);
        Ok(())
    }

    /// 获取用户统计信息
    pub fn get_user_statistics(&self) -> UserStatistics {
        let mut stats = UserStatistics::default();
        
        for user in self.users.values() {
            stats.total_users += 1;
            
            if user.is_active {
                stats.active_users += 1;
            }

            match user.role {
                Role::SuperAdmin => stats.super_admin_users += 1,
                Role::Admin => stats.admin_users += 1,
                Role::Operator => stats.operator_users += 1,
                Role::ReadOnly => stats.readonly_users += 1,
                Role::ApiUser => stats.api_users += 1,
            }
        }

        stats.active_sessions = self.active_sessions.len();
        stats.total_policies = self.policies.len();
        stats.recent_access_logs = self.access_logs.len().min(100);

        stats
    }

    /// 获取访问日志
    pub fn get_access_logs(&self, limit: Option<usize>) -> Vec<AccessLogEntry> {
        let limit = limit.unwrap_or(100);
        self.access_logs
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// 清理过期的访问计数和日志
    pub fn cleanup_expired_data(&mut self) {
        let current_time = SystemTime::now();
        let cleanup_duration = Duration::from_secs(3600); // 1小时

        // 清理过期的IP访问计数
        self.ip_access_counts.retain(|_, counter| {
            current_time.duration_since(counter.last_reset).unwrap_or(Duration::ZERO) < cleanup_duration
        });

        // 清理过期的会话
        self.active_sessions.retain(|_, session| {
            current_time.duration_since(session.last_activity).unwrap_or(Duration::ZERO) < cleanup_duration
        });

        // 重置API配额（如果需要）
        for user in self.users.values_mut() {
            if chrono::Utc::now() >= user.quota_reset_time {
                user.reset_api_quota();
            }
        }
    }
}

/// 用户统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserStatistics {
    pub total_users: usize,
    pub active_users: usize,
    pub super_admin_users: usize,
    pub admin_users: usize,
    pub operator_users: usize,
    pub readonly_users: usize,
    pub api_users: usize,
    pub active_sessions: usize,
    pub total_policies: usize,
    pub recent_access_logs: usize,
}

/// 默认访问控制策略
impl Default for AccessPolicy {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            allowed_roles: vec![Role::SuperAdmin, Role::Admin],
            allowed_ip_ranges: vec!["127.0.0.1/32".to_string()],
            denied_ip_ranges: vec![],
            allowed_hours: None,
            max_concurrent_connections: Some(100),
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn create_test_user() -> User {
        User {
            id: "test_user".to_string(),
            username: "testuser".to_string(),
            role: Role::Admin,
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
            allowed_ips: Some(vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))]),
            api_quota: Some(1000),
            api_usage_count: 0,
            quota_reset_time: chrono::Utc::now() + chrono::Duration::days(1),
        }
    }

    #[test]
    fn test_user_permissions() {
        let user = create_test_user();
        
        assert!(user.has_permission(&Permission::ConfigRead));
        assert!(user.has_permission(&Permission::ConfigWrite));
        assert!(!user.has_permission(&Permission::UserManage)); // Admin没有用户管理权限
    }

    #[test]
    fn test_ip_access_control() {
        let user = create_test_user();
        let allowed_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let denied_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        assert!(user.is_ip_allowed(&allowed_ip));
        assert!(!user.is_ip_allowed(&denied_ip));
    }

    #[test]
    fn test_api_quota() {
        let mut user = create_test_user();
        
        assert!(user.check_api_quota());
        
        // 模拟达到配额
        user.api_usage_count = 1000;
        assert!(!user.check_api_quota());
        
        // 重置配额
        user.reset_api_quota();
        assert!(user.check_api_quota());
        assert_eq!(user.api_usage_count, 0);
    }

    #[test]
    fn test_access_control_manager() {
        let mut manager = AccessControlManager::new();
        let user = create_test_user();
        
        manager.add_user(user).unwrap();
        
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let result = manager.check_access(
            "test_user",
            &ip,
            "read",
            "/config",
            &Permission::ConfigRead,
        ).unwrap();
        
        assert!(result);
    }

    #[test]
    fn test_role_permissions() {
        assert_eq!(Role::SuperAdmin.get_permissions().len(), 9); // 所有权限
        assert_eq!(Role::ReadOnly.get_permissions().len(), 2); // 只有读权限
        assert_eq!(Role::ApiUser.get_permissions().len(), 1); // 只有代理访问权限
    }
}