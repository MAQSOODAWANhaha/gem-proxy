// src/security/config_security.rs
//! 配置安全性验证
//! 
//! 检查配置中的安全漏洞和不安全设置

use crate::config::{ProxyConfig, AuthConfig};
use crate::error::{GeminiProxyError, ValidationError, ErrorSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 安全威胁级别
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreatLevel {
    /// 低威胁
    Low,
    /// 中等威胁
    Medium,
    /// 高威胁
    High,
    /// 严重威胁
    Critical,
}

/// 安全问题类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityIssueType {
    /// 弱密码
    WeakPassword,
    /// 默认凭据
    DefaultCredentials,
    /// 不安全的配置
    InsecureConfiguration,
    /// 信息泄露
    InformationDisclosure,
    /// 权限过度
    ExcessivePermissions,
    /// 加密问题
    CryptographicIssue,
    /// 网络安全
    NetworkSecurity,
}

/// 安全问题报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    /// 问题ID
    pub id: String,
    /// 问题类型
    pub issue_type: SecurityIssueType,
    /// 威胁级别
    pub threat_level: ThreatLevel,
    /// 问题描述
    pub description: String,
    /// 影响的配置字段
    pub affected_field: String,
    /// 建议修复方案
    pub remediation: String,
    /// CWE ID（Common Weakness Enumeration）
    pub cwe_id: Option<u32>,
    /// 影响评估
    pub impact: String,
}

/// 安全配置验证器
pub struct SecurityConfigValidator;

impl SecurityConfigValidator {
    /// 全面的安全配置检查
    pub fn validate_security(config: &ProxyConfig) -> Result<SecurityAuditReport, GeminiProxyError> {
        let mut issues = Vec::new();

        // 认证安全检查
        Self::check_authentication_security(&config.auth, &mut issues);
        
        // TLS 安全检查
        Self::check_tls_security(config, &mut issues);
        
        // API 密钥安全检查
        Self::check_api_key_security(config, &mut issues);
        
        // 网络配置安全检查
        Self::check_network_security(config, &mut issues);
        
        // 日志安全检查
        Self::check_logging_security(config, &mut issues);

        // 生成安全审计报告
        let report = SecurityAuditReport::new(issues);
        
        // 如果有严重安全问题，返回错误
        if report.has_critical_issues() {
            let critical_issues: Vec<String> = report.issues
                .iter()
                .filter(|i| i.threat_level == ThreatLevel::Critical)
                .map(|i| i.description.clone())
                .collect();

            return Err(GeminiProxyError::config_with_context(
                format!("发现 {} 个严重安全问题: {}", 
                    critical_issues.len(), 
                    critical_issues.join("; ")),
                "security",
                "validate_config"
            ).with_severity(ErrorSeverity::Critical));
        }

        Ok(report)
    }

    /// 检查认证配置安全性
    fn check_authentication_security(auth: &AuthConfig, issues: &mut Vec<SecurityIssue>) {
        if auth.enabled {
            // JWT 密钥强度检查
            Self::check_jwt_secret_strength(&auth.jwt_secret, issues);
            
            // 管理员密码强度检查
            Self::check_admin_password_strength(&auth.admin_password, issues);
            
            // Token 过期时间检查
            Self::check_token_expiry_settings(auth, issues);
            
            // 会话安全检查
            Self::check_session_security(auth, issues);
        } else {
            // 认证未启用的安全风险
            issues.push(SecurityIssue {
                id: "AUTH_001".to_string(),
                issue_type: SecurityIssueType::InsecureConfiguration,
                threat_level: ThreatLevel::High,
                description: "认证功能未启用，API端点无保护".to_string(),
                affected_field: "auth.enabled".to_string(),
                remediation: "启用认证功能以保护API端点".to_string(),
                cwe_id: Some(287), // CWE-287: Improper Authentication
                impact: "未经授权的用户可以访问所有API功能".to_string(),
            });
        }
    }

    /// 检查JWT密钥强度
    fn check_jwt_secret_strength(jwt_secret: &str, issues: &mut Vec<SecurityIssue>) {
        // 检查是否使用默认值
        let dangerous_defaults = [
            "your-super-secret-key-that-is-long-and-secure",
            "secret",
            "jwt-secret",
            "my-secret-key",
            "changeme",
        ];

        if dangerous_defaults.contains(&jwt_secret) {
            issues.push(SecurityIssue {
                id: "AUTH_002".to_string(),
                issue_type: SecurityIssueType::DefaultCredentials,
                threat_level: ThreatLevel::Critical,
                description: "检测到默认的JWT密钥".to_string(),
                affected_field: "auth.jwt_secret".to_string(),
                remediation: "使用加密学安全的随机字符串作为JWT密钥".to_string(),
                cwe_id: Some(798), // CWE-798: Use of Hard-coded Credentials
                impact: "攻击者可以伪造有效的JWT令牌".to_string(),
            });
        }

        // 检查密钥长度
        if jwt_secret.len() < 32 {
            issues.push(SecurityIssue {
                id: "AUTH_003".to_string(),
                issue_type: SecurityIssueType::WeakPassword,
                threat_level: ThreatLevel::High,
                description: format!("JWT密钥长度过短: {} 字符", jwt_secret.len()),
                affected_field: "auth.jwt_secret".to_string(),
                remediation: "使用至少32个字符的强密钥".to_string(),
                cwe_id: Some(521), // CWE-521: Weak Password Requirements
                impact: "弱密钥容易被暴力破解".to_string(),
            });
        }

        // 检查密钥复杂度
        let has_lowercase = jwt_secret.chars().any(|c| c.is_ascii_lowercase());
        let has_uppercase = jwt_secret.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = jwt_secret.chars().any(|c| c.is_ascii_digit());
        let has_special = jwt_secret.chars().any(|c| !c.is_ascii_alphanumeric());

        let complexity_score = [has_lowercase, has_uppercase, has_digit, has_special]
            .iter()
            .filter(|&&x| x)
            .count();

        if complexity_score < 3 {
            issues.push(SecurityIssue {
                id: "AUTH_004".to_string(),
                issue_type: SecurityIssueType::WeakPassword,
                threat_level: ThreatLevel::Medium,
                description: "JWT密钥复杂度不足".to_string(),
                affected_field: "auth.jwt_secret".to_string(),
                remediation: "使用包含大小写字母、数字和特殊字符的复杂密钥".to_string(),
                cwe_id: Some(521), // CWE-521: Weak Password Requirements
                impact: "简单密钥更容易被字典攻击破解".to_string(),
            });
        }
    }

    /// 检查管理员密码强度
    fn check_admin_password_strength(password: &str, issues: &mut Vec<SecurityIssue>) {
        // 检查危险的默认密码
        let dangerous_passwords = [
            "admin",
            "password",
            "123456",
            "admin123",
            "changeme",
            "root",
            "administrator",
        ];

        if dangerous_passwords.contains(&password) {
            issues.push(SecurityIssue {
                id: "AUTH_005".to_string(),
                issue_type: SecurityIssueType::DefaultCredentials,
                threat_level: ThreatLevel::Critical,
                description: "检测到弱的默认管理员密码".to_string(),
                affected_field: "auth.admin_password".to_string(),
                remediation: "设置强密码，至少12个字符，包含大小写字母、数字和特殊字符".to_string(),
                cwe_id: Some(798), // CWE-798: Use of Hard-coded Credentials
                impact: "攻击者可以轻易获得管理员权限".to_string(),
            });
        }

        // 密码长度检查
        if password.len() < 8 {
            issues.push(SecurityIssue {
                id: "AUTH_006".to_string(),
                issue_type: SecurityIssueType::WeakPassword,
                threat_level: ThreatLevel::High,
                description: format!("管理员密码过短: {} 字符", password.len()),
                affected_field: "auth.admin_password".to_string(),
                remediation: "使用至少12个字符的强密码".to_string(),
                cwe_id: Some(521), // CWE-521: Weak Password Requirements
                impact: "短密码容易被暴力破解".to_string(),
            });
        }
    }

    /// 检查Token过期设置
    fn check_token_expiry_settings(auth: &AuthConfig, issues: &mut Vec<SecurityIssue>) {
        // Token过期时间过长
        if auth.token_expiry_hours > 24 {
            issues.push(SecurityIssue {
                id: "AUTH_007".to_string(),
                issue_type: SecurityIssueType::InsecureConfiguration,
                threat_level: ThreatLevel::Medium,
                description: format!("Token过期时间过长: {} 小时", auth.token_expiry_hours),
                affected_field: "auth.token_expiry_hours".to_string(),
                remediation: "建议将Token过期时间设置为24小时以内".to_string(),
                cwe_id: Some(613), // CWE-613: Insufficient Session Expiration
                impact: "长期有效的Token增加了被滥用的风险".to_string(),
            });
        }

        // 会话超时时间过长
        if auth.session_timeout_minutes > 120 {
            issues.push(SecurityIssue {
                id: "AUTH_008".to_string(),
                issue_type: SecurityIssueType::InsecureConfiguration,
                threat_level: ThreatLevel::Low,
                description: format!("会话超时时间过长: {} 分钟", auth.session_timeout_minutes),
                affected_field: "auth.session_timeout_minutes".to_string(),
                remediation: "建议将会话超时设置为60分钟以内".to_string(),
                cwe_id: Some(613), // CWE-613: Insufficient Session Expiration
                impact: "长时间的会话增加了会话劫持的风险".to_string(),
            });
        }
    }

    /// 检查会话安全配置
    fn check_session_security(auth: &AuthConfig, issues: &mut Vec<SecurityIssue>) {
        // 登录尝试次数限制
        if auth.max_login_attempts > 10 {
            issues.push(SecurityIssue {
                id: "AUTH_009".to_string(),
                issue_type: SecurityIssueType::InsecureConfiguration,
                threat_level: ThreatLevel::Medium,
                description: format!("最大登录尝试次数过高: {}", auth.max_login_attempts),
                affected_field: "auth.max_login_attempts".to_string(),
                remediation: "建议将最大登录尝试次数设置为5次以内".to_string(),
                cwe_id: Some(307), // CWE-307: Improper Restriction of Excessive Authentication Attempts
                impact: "过高的尝试次数限制使暴力破解攻击更容易成功".to_string(),
            });
        }

        // 锁定时间过短
        if auth.lockout_duration_minutes < 5 {
            issues.push(SecurityIssue {
                id: "AUTH_010".to_string(),
                issue_type: SecurityIssueType::InsecureConfiguration,
                threat_level: ThreatLevel::Low,
                description: format!("账户锁定时间过短: {} 分钟", auth.lockout_duration_minutes),
                affected_field: "auth.lockout_duration_minutes".to_string(),
                remediation: "建议将锁定时间设置为至少15分钟".to_string(),
                cwe_id: Some(307), // CWE-307: Improper Restriction of Excessive Authentication Attempts
                impact: "短锁定时间不能有效阻止暴力破解攻击".to_string(),
            });
        }
    }

    /// 检查TLS配置安全性
    fn check_tls_security(config: &ProxyConfig, issues: &mut Vec<SecurityIssue>) {
        if !config.server.tls.enabled {
            issues.push(SecurityIssue {
                id: "TLS_001".to_string(),
                issue_type: SecurityIssueType::NetworkSecurity,
                threat_level: ThreatLevel::High,
                description: "TLS未启用，通信未加密".to_string(),
                affected_field: "server.tls.enabled".to_string(),
                remediation: "启用TLS加密保护通信安全".to_string(),
                cwe_id: Some(319), // CWE-319: Cleartext Transmission of Sensitive Information
                impact: "敏感数据在网络传输中可能被窃听".to_string(),
            });
        } else {
            // 检查证书路径
            if config.server.tls.cert_path.is_empty() || config.server.tls.key_path.is_empty() {
                issues.push(SecurityIssue {
                    id: "TLS_002".to_string(),
                    issue_type: SecurityIssueType::InsecureConfiguration,
                    threat_level: ThreatLevel::Critical,
                    description: "TLS证书或私钥路径未配置".to_string(),
                    affected_field: "server.tls".to_string(),
                    remediation: "配置有效的TLS证书和私钥路径".to_string(),
                    cwe_id: Some(295), // CWE-295: Improper Certificate Validation
                    impact: "无效的TLS配置导致服务无法启动或不安全".to_string(),
                });
            }
        }
    }

    /// 检查API密钥安全性
    fn check_api_key_security(config: &ProxyConfig, issues: &mut Vec<SecurityIssue>) {
        for (i, api_key) in config.gemini.api_keys.iter().enumerate() {
            // 检查是否使用示例密钥
            if api_key.key.contains("example") || 
               api_key.key == "your-gemini-api-key-here" ||
               api_key.key.starts_with("AIza") && api_key.key.len() < 20 {
                issues.push(SecurityIssue {
                    id: format!("API_001_{}", i),
                    issue_type: SecurityIssueType::DefaultCredentials,
                    threat_level: ThreatLevel::Critical,
                    description: format!("API密钥 {} 似乎是示例或无效密钥", api_key.id),
                    affected_field: format!("gemini.api_keys[{}].key", i),
                    remediation: "使用有效的Gemini API密钥".to_string(),
                    cwe_id: Some(798), // CWE-798: Use of Hard-coded Credentials
                    impact: "无效的API密钥导致服务无法正常工作".to_string(),
                });
            }

            // 检查密钥ID是否包含敏感信息
            if api_key.id.to_lowercase().contains("prod") || 
               api_key.id.to_lowercase().contains("production") {
                issues.push(SecurityIssue {
                    id: format!("API_002_{}", i),
                    issue_type: SecurityIssueType::InformationDisclosure,
                    threat_level: ThreatLevel::Low,
                    description: format!("API密钥ID '{}' 可能泄露环境信息", api_key.id),
                    affected_field: format!("gemini.api_keys[{}].id", i),
                    remediation: "使用不包含环境信息的通用密钥ID".to_string(),
                    cwe_id: Some(200), // CWE-200: Information Exposure
                    impact: "攻击者可能从密钥ID推断系统信息".to_string(),
                });
            }
        }
    }

    /// 检查网络配置安全性
    fn check_network_security(config: &ProxyConfig, issues: &mut Vec<SecurityIssue>) {
        // 检查是否绑定到所有接口
        if config.server.host == "0.0.0.0" {
            issues.push(SecurityIssue {
                id: "NET_001".to_string(),
                issue_type: SecurityIssueType::NetworkSecurity,
                threat_level: ThreatLevel::Medium,
                description: "服务绑定到所有网络接口 (0.0.0.0)".to_string(),
                affected_field: "server.host".to_string(),
                remediation: "考虑绑定到特定的网络接口以限制访问".to_string(),
                cwe_id: Some(668), // CWE-668: Exposure of Resource to Wrong Sphere
                impact: "服务可能从不期望的网络接口被访问".to_string(),
            });
        }

        // 检查端口使用
        if config.server.port < 1024 && config.server.port != 80 && config.server.port != 443 {
            issues.push(SecurityIssue {
                id: "NET_002".to_string(),
                issue_type: SecurityIssueType::ExcessivePermissions,
                threat_level: ThreatLevel::Low,
                description: format!("使用特权端口: {}", config.server.port),
                affected_field: "server.port".to_string(),
                remediation: "考虑使用非特权端口 (>1024) 以降低权限要求".to_string(),
                cwe_id: Some(250), // CWE-250: Execution with Unnecessary Privileges
                impact: "需要提升权限运行服务".to_string(),
            });
        }
    }

    /// 检查日志配置安全性
    fn check_logging_security(config: &ProxyConfig, issues: &mut Vec<SecurityIssue>) {
        // 如果监控未启用，可能影响安全审计
        if !config.metrics.enabled {
            issues.push(SecurityIssue {
                id: "LOG_001".to_string(),
                issue_type: SecurityIssueType::InsecureConfiguration,
                threat_level: ThreatLevel::Medium,
                description: "监控功能未启用，缺乏安全审计能力".to_string(),
                affected_field: "metrics.enabled".to_string(),
                remediation: "启用监控功能以支持安全审计和异常检测".to_string(),
                cwe_id: Some(778), // CWE-778: Insufficient Logging
                impact: "无法有效监控和检测安全事件".to_string(),
            });
        }
    }
}

/// 安全审计报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditReport {
    /// 安全问题列表
    pub issues: Vec<SecurityIssue>,
    /// 审计时间
    pub audit_timestamp: chrono::DateTime<chrono::Utc>,
    /// 总体安全评分 (0-100)
    pub security_score: u8,
    /// 摘要统计
    pub summary: SecuritySummary,
}

/// 安全摘要统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySummary {
    pub total_issues: usize,
    pub critical_issues: usize,
    pub high_issues: usize,
    pub medium_issues: usize,
    pub low_issues: usize,
    pub issues_by_type: HashMap<String, usize>,
}

impl SecurityAuditReport {
    /// 创建新的安全审计报告
    pub fn new(issues: Vec<SecurityIssue>) -> Self {
        let summary = SecuritySummary::from_issues(&issues);
        let security_score = Self::calculate_security_score(&summary);

        Self {
            issues,
            audit_timestamp: chrono::Utc::now(),
            security_score,
            summary,
        }
    }

    /// 检查是否有严重安全问题
    pub fn has_critical_issues(&self) -> bool {
        self.summary.critical_issues > 0
    }

    /// 计算安全评分
    fn calculate_security_score(summary: &SecuritySummary) -> u8 {
        let base_score = 100;
        let critical_penalty = summary.critical_issues * 25;
        let high_penalty = summary.high_issues * 15;
        let medium_penalty = summary.medium_issues * 8;
        let low_penalty = summary.low_issues * 3;

        let total_penalty = critical_penalty + high_penalty + medium_penalty + low_penalty;
        (base_score.saturating_sub(total_penalty)).max(0) as u8
    }

    /// 生成修复建议
    pub fn generate_remediation_plan(&self) -> Vec<String> {
        let mut plan = Vec::new();

        // 按优先级排序问题
        let mut sorted_issues = self.issues.clone();
        sorted_issues.sort_by(|a, b| {
            match (&a.threat_level, &b.threat_level) {
                (ThreatLevel::Critical, ThreatLevel::Critical) => std::cmp::Ordering::Equal,
                (ThreatLevel::Critical, _) => std::cmp::Ordering::Less,
                (_, ThreatLevel::Critical) => std::cmp::Ordering::Greater,
                (ThreatLevel::High, ThreatLevel::High) => std::cmp::Ordering::Equal,
                (ThreatLevel::High, _) => std::cmp::Ordering::Less,
                (_, ThreatLevel::High) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });

        for (i, issue) in sorted_issues.iter().enumerate() {
            plan.push(format!(
                "{}. [{}] {} - {} (字段: {})",
                i + 1,
                match issue.threat_level {
                    ThreatLevel::Critical => "严重",
                    ThreatLevel::High => "高",
                    ThreatLevel::Medium => "中",
                    ThreatLevel::Low => "低",
                },
                issue.description,
                issue.remediation,
                issue.affected_field
            ));
        }

        plan
    }
}

impl SecuritySummary {
    /// 从问题列表创建摘要
    fn from_issues(issues: &[SecurityIssue]) -> Self {
        let mut summary = Self {
            total_issues: issues.len(),
            critical_issues: 0,
            high_issues: 0,
            medium_issues: 0,
            low_issues: 0,
            issues_by_type: HashMap::new(),
        };

        for issue in issues {
            // 按威胁级别统计
            match issue.threat_level {
                ThreatLevel::Critical => summary.critical_issues += 1,
                ThreatLevel::High => summary.high_issues += 1,
                ThreatLevel::Medium => summary.medium_issues += 1,
                ThreatLevel::Low => summary.low_issues += 1,
            }

            // 按问题类型统计
            let type_key = format!("{:?}", issue.issue_type);
            *summary.issues_by_type.entry(type_key).or_insert(0) += 1;
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProxyConfig, ServerConfig, GeminiConfig, AuthConfig, MetricsConfig, TlsConfig, ApiKeyConfig};

    fn create_insecure_config() -> ProxyConfig {
        ProxyConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 4,
                max_connections: 1000,
                tls: TlsConfig {
                    enabled: false,
                    cert_path: "".to_string(),
                    key_path: "".to_string(),
                    acme: None,
                },
            },
            gemini: GeminiConfig {
                api_keys: vec![ApiKeyConfig {
                    id: "example-key".to_string(),
                    key: "your-gemini-api-key-here".to_string(),
                    weight: 100,
                    max_requests_per_minute: 60,
                }],
                base_url: "https://generativelanguage.googleapis.com".to_string(),
                timeout_seconds: 30,
            },
            auth: AuthConfig {
                enabled: true,
                jwt_secret: "secret".to_string(), // 弱密钥
                rate_limit_per_minute: 60,
                admin_password: "admin123".to_string(), // 弱密码
                token_expiry_hours: 48, // 过长
                refresh_token_enabled: true,
                session_timeout_minutes: 180, // 过长
                max_login_attempts: 20, // 过高
                lockout_duration_minutes: 1, // 过短
            },
            metrics: MetricsConfig {
                enabled: false, // 未启用监控
                prometheus_port: 9090,
                tls: None,
            },
        }
    }

    #[test]
    fn test_security_validation() {
        let config = create_insecure_config();
        let report = SecurityConfigValidator::validate_security(&config).unwrap();

        // 应该检测到多个安全问题
        assert!(report.issues.len() > 5);
        assert!(report.has_critical_issues());
        assert!(report.security_score < 50);
    }

    #[test]
    fn test_jwt_secret_validation() {
        let mut issues = Vec::new();
        
        // 测试弱密钥
        SecurityConfigValidator::check_jwt_secret_strength("secret", &mut issues);
        assert!(!issues.is_empty());
        
        issues.clear();
        
        // 测试强密钥
        SecurityConfigValidator::check_jwt_secret_strength(
            "very-strong-jwt-secret-with-special-chars-123!@#", 
            &mut issues
        );
        // 强密钥应该没有问题
        assert!(issues.is_empty());
    }

    #[test]
    fn test_security_score_calculation() {
        let issues = vec![
            SecurityIssue {
                id: "TEST_001".to_string(),
                issue_type: SecurityIssueType::DefaultCredentials,
                threat_level: ThreatLevel::Critical,
                description: "测试严重问题".to_string(),
                affected_field: "test.field".to_string(),
                remediation: "修复建议".to_string(),
                cwe_id: None,
                impact: "影响描述".to_string(),
            }
        ];

        let report = SecurityAuditReport::new(issues);
        assert!(report.security_score <= 75); // 严重问题应该大幅降低评分
    }
}