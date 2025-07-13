use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub server: ServerConfig,
    pub gemini: GeminiConfig,
    pub auth: AuthConfig,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub max_connections: usize,
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcmeConfig {
    pub enabled: bool,
    pub domains: Vec<String>,
    pub email: String,
    pub directory_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: String,
    pub key_path: String,
    pub acme: Option<AcmeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_keys: Vec<ApiKeyConfig>,
    pub base_url: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub id: String,
    pub key: String,
    pub weight: u32,
    pub max_requests_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt_secret: String,
    pub rate_limit_per_minute: u32,
    pub admin_password: String,
    pub token_expiry_hours: u64,
    pub refresh_token_enabled: bool,
    pub session_timeout_minutes: u64,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub prometheus_port: u16,
    pub tls: Option<TlsConfig>,  // API 服务器的 TLS 配置
}

impl ProxyConfig {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path)?;
        let config: ProxyConfig = serde_yaml::from_str(&content)?;
        
        // 配置验证
        config.validate()?;
        
        Ok(config)
    }

    /// 使用新错误系统从文件加载配置
    pub fn from_file_enhanced(path: &str) -> crate::error::Result<Self> {
        use crate::config::validation::ConfigValidator;
        
        let content = fs::read_to_string(path)
            .map_err(|e| crate::error::GeminiProxyError::config_with_context(
                format!("无法读取配置文件: {}", e),
                "config",
                "load_file"
            ).with_metadata("file_path", path))?;

        let config: ProxyConfig = serde_yaml::from_str(&content)
            .map_err(|e| crate::error::GeminiProxyError::config_with_context(
                format!("配置文件格式错误: {}", e),
                "config", 
                "parse_yaml"
            ).with_metadata("file_path", path))?;

        // 使用新的验证器
        ConfigValidator::validate_proxy_config(&config)?;
        
        Ok(config)
    }
    
    /// 配置验证
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 服务器配置验证
        if self.server.port == 0 {
            return Err("服务器端口不能为0".into());
        }
        
        if self.server.workers == 0 {
            return Err("工作线程数不能为0".into());
        }
        
        if self.server.max_connections == 0 {
            return Err("最大连接数不能为0".into());
        }
        
        // TLS配置验证
        if self.server.tls.enabled {
            if self.server.tls.cert_path.is_empty() {
                return Err("启用TLS时必须指定证书路径".into());
            }
            if self.server.tls.key_path.is_empty() {
                return Err("启用TLS时必须指定私钥路径".into());
            }
            
            // ACME配置验证
            if let Some(acme) = &self.server.tls.acme {
                if acme.enabled {
                    if acme.domains.is_empty() {
                        return Err("启用ACME时必须指定域名".into());
                    }
                    if acme.email.is_empty() {
                        return Err("启用ACME时必须指定邮箱".into());
                    }
                    if acme.directory_url.is_empty() {
                        return Err("启用ACME时必须指定目录URL".into());
                    }
                    
                    // 验证邮箱格式
                    if !acme.email.contains('@') {
                        return Err("ACME邮箱格式无效".into());
                    }
                    
                    // 验证域名格式
                    for domain in &acme.domains {
                        if domain.is_empty() || domain.contains(' ') {
                            return Err(format!("无效的域名: {domain}").into());
                        }
                    }
                }
            }
        }
        
        // Gemini配置验证
        if self.gemini.api_keys.is_empty() {
            return Err("必须配置至少一个Gemini API密钥".into());
        }
        
        if self.gemini.base_url.is_empty() {
            return Err("Gemini基础URL不能为空".into());
        }
        
        if self.gemini.timeout_seconds == 0 {
            return Err("Gemini超时时间不能为0".into());
        }
        
        // API密钥配置验证
        for (i, api_key) in self.gemini.api_keys.iter().enumerate() {
            if api_key.id.is_empty() {
                return Err(format!("第{}个API密钥的ID不能为空", i + 1).into());
            }
            if api_key.key.is_empty() {
                return Err(format!("第{}个API密钥的key不能为空", i + 1).into());
            }
            if api_key.weight == 0 {
                return Err(format!("第{}个API密钥的权重不能为0", i + 1).into());
            }
            if api_key.max_requests_per_minute == 0 {
                return Err(format!("第{}个API密钥的每分钟最大请求数不能为0", i + 1).into());
            }
        }
        
        // 检查API密钥ID唯一性
        let mut seen_ids = std::collections::HashSet::new();
        for api_key in &self.gemini.api_keys {
            if !seen_ids.insert(&api_key.id) {
                return Err(format!("API密钥ID重复: {}", api_key.id).into());
            }
        }
        
        // 认证配置验证
        if self.auth.enabled {
            if self.auth.jwt_secret.is_empty() {
                return Err("启用认证时JWT密钥不能为空".into());
            }
            if self.auth.jwt_secret.len() < 32 {
                return Err("JWT密钥长度至少需要32个字符".into());
            }
            if self.auth.rate_limit_per_minute == 0 {
                return Err("速率限制值不能为0".into());
            }
            if self.auth.admin_password.is_empty() {
                return Err("启用认证时管理员密码不能为空".into());
            }
            if self.auth.admin_password.len() < 8 {
                return Err("管理员密码长度至少需要8个字符".into());
            }
            
            // 检查不安全的默认值
            if self.auth.admin_password == "admin123456" || 
               self.auth.admin_password == "admin" ||
               self.auth.admin_password == "password" ||
               self.auth.admin_password == "123456" {
                return Err("⚠️  安全警告：检测到不安全的默认密码，请更改为强密码".into());
            }
            
            if self.auth.jwt_secret == "your-super-secret-key-that-is-long-and-secure" ||
               self.auth.jwt_secret.len() < 32 {
                return Err("⚠️  安全警告：JWT密钥不安全，请使用至少32字符的随机密钥".into());
            }
            
            // 检查密码强度
            if !self.is_password_strong(&self.auth.admin_password) {
                tracing::warn!("⚠️  建议使用更强的管理员密码（包含大小写字母、数字和特殊字符）");
            }
            if self.auth.token_expiry_hours == 0 {
                return Err("Token过期时间不能为0".into());
            }
            if self.auth.session_timeout_minutes == 0 {
                return Err("会话超时时间不能为0".into());
            }
            if self.auth.max_login_attempts == 0 {
                return Err("最大登录尝试次数不能为0".into());
            }
            if self.auth.lockout_duration_minutes == 0 {
                return Err("锁定持续时间不能为0".into());
            }
        }
        
        // 监控配置验证
        if self.metrics.enabled {
            if self.metrics.prometheus_port == 0 {
                return Err("Prometheus端口不能为0".into());
            }
            if self.metrics.prometheus_port == self.server.port {
                return Err("Prometheus端口不能与服务器端口相同".into());
            }
            
            // API 服务器 TLS 配置验证
            if let Some(api_tls) = &self.metrics.tls {
                if api_tls.enabled {
                    if api_tls.cert_path.is_empty() {
                        return Err("API服务器启用TLS时必须指定证书路径".into());
                    }
                    if api_tls.key_path.is_empty() {
                        return Err("API服务器启用TLS时必须指定私钥路径".into());
                    }
                }
            }
        }
        
        // Gemini API 密钥安全检查
        for (i, api_key) in self.gemini.api_keys.iter().enumerate() {
            if api_key.key.starts_with("AIzaSyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX") ||
               api_key.key.starts_with("AIzaSyYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY") ||
               api_key.key.starts_with("AIzaSyZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ") ||
               api_key.key == "YOUR_REAL_GEMINI_API_KEY_HERE" {
                return Err(format!("⚠️  安全警告：API密钥 {} 使用的是示例值，请配置真实的 Gemini API 密钥", i + 1).into());
            }
            
            if api_key.key.len() < 30 {
                return Err(format!("⚠️  API密钥 {} 长度异常，请检查是否为有效的 Gemini API 密钥", i + 1).into());
            }
        }
        
        Ok(())
    }
    
    /// 检查密码强度
    fn is_password_strong(&self, password: &str) -> bool {
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));
        
        password.len() >= 12 && has_lower && has_upper && has_digit && has_special
    }
}
