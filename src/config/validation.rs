// src/config/validation.rs
//! 配置验证模块
//! 
//! 使用新的统一错误系统进行配置验证

use crate::error::{GeminiProxyError, ValidationError, ErrorSeverity};
use super::{ProxyConfig, AuthConfig};

/// 配置验证器
pub struct ConfigValidator;

impl ConfigValidator {
    /// 验证完整的代理配置
    pub fn validate_proxy_config(config: &ProxyConfig) -> crate::error::Result<()> {
        let mut validation_errors = Vec::new();

        // 验证服务器配置
        Self::validate_server_config(config, &mut validation_errors);
        
        // 验证 Gemini 配置
        Self::validate_gemini_config(config, &mut validation_errors);
        
        // 验证认证配置
        Self::validate_auth_config(&config.auth, &mut validation_errors);
        
        // 验证监控配置
        Self::validate_metrics_config(config, &mut validation_errors);

        if !validation_errors.is_empty() {
            return Err(GeminiProxyError::validation(
                format!("配置验证失败，发现 {} 个错误", validation_errors.len()),
                validation_errors,
            ).with_severity(ErrorSeverity::Critical));
        }

        Ok(())
    }

    /// 验证服务器配置
    fn validate_server_config(config: &ProxyConfig, errors: &mut Vec<ValidationError>) {
        // 端口验证
        if config.server.port == 0 {
            errors.push(ValidationError {
                field: "server.port".to_string(),
                message: "服务器端口不能为0".to_string(),
                value: Some(config.server.port.to_string()),
            });
        }

        // 工作线程数验证
        if config.server.workers == 0 {
            errors.push(ValidationError {
                field: "server.workers".to_string(),
                message: "工作线程数不能为0".to_string(),
                value: Some(config.server.workers.to_string()),
            });
        }

        // 最大连接数验证
        if config.server.max_connections == 0 {
            errors.push(ValidationError {
                field: "server.max_connections".to_string(),
                message: "最大连接数不能为0".to_string(),
                value: Some(config.server.max_connections.to_string()),
            });
        }

        // TLS 配置验证
        if config.server.tls.enabled {
            if config.server.tls.cert_path.is_empty() {
                errors.push(ValidationError {
                    field: "server.tls.cert_path".to_string(),
                    message: "启用TLS时必须指定证书路径".to_string(),
                    value: Some(config.server.tls.cert_path.clone()),
                });
            }

            if config.server.tls.key_path.is_empty() {
                errors.push(ValidationError {
                    field: "server.tls.key_path".to_string(),
                    message: "启用TLS时必须指定私钥路径".to_string(),
                    value: Some(config.server.tls.key_path.clone()),
                });
            }

            // ACME 配置验证
            if let Some(acme) = &config.server.tls.acme {
                if acme.enabled {
                    if acme.domains.is_empty() {
                        errors.push(ValidationError {
                            field: "server.tls.acme.domains".to_string(),
                            message: "启用ACME时必须指定域名".to_string(),
                            value: None,
                        });
                    }

                    if acme.email.is_empty() {
                        errors.push(ValidationError {
                            field: "server.tls.acme.email".to_string(),
                            message: "启用ACME时必须指定邮箱".to_string(),
                            value: Some(acme.email.clone()),
                        });
                    } else if !acme.email.contains('@') {
                        errors.push(ValidationError {
                            field: "server.tls.acme.email".to_string(),
                            message: "ACME邮箱格式无效".to_string(),
                            value: Some(acme.email.clone()),
                        });
                    }

                    if acme.directory_url.is_empty() {
                        errors.push(ValidationError {
                            field: "server.tls.acme.directory_url".to_string(),
                            message: "启用ACME时必须指定目录URL".to_string(),
                            value: Some(acme.directory_url.clone()),
                        });
                    }

                    // 验证域名格式
                    for (i, domain) in acme.domains.iter().enumerate() {
                        if domain.is_empty() || domain.contains(' ') {
                            errors.push(ValidationError {
                                field: format!("server.tls.acme.domains[{}]", i),
                                message: "无效的域名".to_string(),
                                value: Some(domain.clone()),
                            });
                        }
                    }
                }
            }
        }
    }

    /// 验证 Gemini 配置
    fn validate_gemini_config(config: &ProxyConfig, errors: &mut Vec<ValidationError>) {
        // API 密钥数量验证
        if config.gemini.api_keys.is_empty() {
            errors.push(ValidationError {
                field: "gemini.api_keys".to_string(),
                message: "必须配置至少一个Gemini API密钥".to_string(),
                value: None,
            });
        }

        // 基础 URL 验证
        if config.gemini.base_url.is_empty() {
            errors.push(ValidationError {
                field: "gemini.base_url".to_string(),
                message: "Gemini基础URL不能为空".to_string(),
                value: Some(config.gemini.base_url.clone()),
            });
        }

        // 超时时间验证
        if config.gemini.timeout_seconds == 0 {
            errors.push(ValidationError {
                field: "gemini.timeout_seconds".to_string(),
                message: "Gemini超时时间不能为0".to_string(),
                value: Some(config.gemini.timeout_seconds.to_string()),
            });
        }

        // API 密钥配置验证
        for (i, api_key) in config.gemini.api_keys.iter().enumerate() {
            let prefix = format!("gemini.api_keys[{}]", i);

            if api_key.id.is_empty() {
                errors.push(ValidationError {
                    field: format!("{}.id", prefix),
                    message: "API密钥ID不能为空".to_string(),
                    value: Some(api_key.id.clone()),
                });
            }

            if api_key.key.is_empty() {
                errors.push(ValidationError {
                    field: format!("{}.key", prefix),
                    message: "API密钥不能为空".to_string(),
                    value: None, // 不记录实际密钥值
                });
            }

            // 检查是否使用示例值
            if api_key.key == "your-gemini-api-key-here" {
                errors.push(ValidationError {
                    field: format!("{}.key", prefix),
                    message: "检测到示例API密钥，请替换为真实密钥".to_string(),
                    value: None,
                });
            }

            if api_key.weight == 0 {
                errors.push(ValidationError {
                    field: format!("{}.weight", prefix),
                    message: "API密钥权重不能为0".to_string(),
                    value: Some(api_key.weight.to_string()),
                });
            }

            if api_key.max_requests_per_minute == 0 {
                errors.push(ValidationError {
                    field: format!("{}.max_requests_per_minute", prefix),
                    message: "API密钥最大请求数不能为0".to_string(),
                    value: Some(api_key.max_requests_per_minute.to_string()),
                });
            }
        }

        // 检查重复的 API 密钥 ID
        let mut seen_ids = std::collections::HashSet::new();
        for (i, api_key) in config.gemini.api_keys.iter().enumerate() {
            if !seen_ids.insert(&api_key.id) {
                errors.push(ValidationError {
                    field: format!("gemini.api_keys[{}].id", i),
                    message: "API密钥ID重复".to_string(),
                    value: Some(api_key.id.clone()),
                });
            }
        }
    }

    /// 验证认证配置
    fn validate_auth_config(config: &AuthConfig, errors: &mut Vec<ValidationError>) {
        if config.enabled {
            // JWT 密钥验证
            if config.jwt_secret.is_empty() {
                errors.push(ValidationError {
                    field: "auth.jwt_secret".to_string(),
                    message: "启用认证时JWT密钥不能为空".to_string(),
                    value: None,
                });
            } else if config.jwt_secret.len() < 32 {
                errors.push(ValidationError {
                    field: "auth.jwt_secret".to_string(),
                    message: "JWT密钥长度至少需要32个字符".to_string(),
                    value: None,
                });
            }

            // 检查是否使用不安全的默认值
            if config.jwt_secret == "your-super-secret-key-that-is-long-and-secure" {
                errors.push(ValidationError {
                    field: "auth.jwt_secret".to_string(),
                    message: "检测到不安全的默认JWT密钥，请更换为安全的密钥".to_string(),
                    value: None,
                });
            }

            // 管理员密码验证
            if config.admin_password.is_empty() {
                errors.push(ValidationError {
                    field: "auth.admin_password".to_string(),
                    message: "启用认证时管理员密码不能为空".to_string(),
                    value: None,
                });
            } else if config.admin_password.len() < 8 {
                errors.push(ValidationError {
                    field: "auth.admin_password".to_string(),
                    message: "管理员密码长度至少需要8个字符".to_string(),
                    value: None,
                });
            }

            // 检查是否使用不安全的默认密码
            if config.admin_password == "admin123" {
                errors.push(ValidationError {
                    field: "auth.admin_password".to_string(),
                    message: "检测到不安全的默认密码，请更换为强密码".to_string(),
                    value: None,
                });
            }

            // 速率限制验证
            if config.rate_limit_per_minute == 0 {
                errors.push(ValidationError {
                    field: "auth.rate_limit_per_minute".to_string(),
                    message: "速率限制不能为0".to_string(),
                    value: Some(config.rate_limit_per_minute.to_string()),
                });
            }

            // Token 过期时间验证
            if config.token_expiry_hours == 0 {
                errors.push(ValidationError {
                    field: "auth.token_expiry_hours".to_string(),
                    message: "Token过期时间不能为0".to_string(),
                    value: Some(config.token_expiry_hours.to_string()),
                });
            } else if config.token_expiry_hours > 168 { // 7天
                errors.push(ValidationError {
                    field: "auth.token_expiry_hours".to_string(),
                    message: "Token过期时间不建议超过168小时(7天)".to_string(),
                    value: Some(config.token_expiry_hours.to_string()),
                });
            }
        }
    }

    /// 验证监控配置
    fn validate_metrics_config(config: &ProxyConfig, errors: &mut Vec<ValidationError>) {
        if config.metrics.enabled {
            // Prometheus 端口验证
            if config.metrics.prometheus_port == 0 {
                errors.push(ValidationError {
                    field: "metrics.prometheus_port".to_string(),
                    message: "启用监控时Prometheus端口不能为0".to_string(),
                    value: Some(config.metrics.prometheus_port.to_string()),
                });
            }

            // 避免端口冲突
            if config.metrics.prometheus_port == config.server.port {
                errors.push(ValidationError {
                    field: "metrics.prometheus_port".to_string(),
                    message: "Prometheus端口不能与服务器端口相同".to_string(),
                    value: Some(config.metrics.prometheus_port.to_string()),
                });
            }
        }
    }

    /// 验证单个字段
    pub fn validate_field<T>(
        field_name: &str,
        value: &T,
        validator: impl Fn(&T) -> bool,
        error_message: &str,
    ) -> Result<(), ValidationError> {
        if validator(value) {
            Ok(())
        } else {
            Err(ValidationError {
                field: field_name.to_string(),
                message: error_message.to_string(),
                value: None,
            })
        }
    }

    /// 创建配置错误
    pub fn create_config_error(message: &str, field: Option<&str>) -> GeminiProxyError {
        let mut error = GeminiProxyError::config_with_context(message, "config", "validation")
            .with_severity(ErrorSeverity::Critical);

        if let Some(field) = field {
            error = error.with_metadata("field", field);
        }

        error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProxyConfig, ServerConfig, GeminiConfig, AuthConfig, MetricsConfig, TlsConfig, ApiKeyConfig};

    fn create_valid_config() -> ProxyConfig {
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
                    id: "test-key".to_string(),
                    key: "valid-test-key-12345678901234567890".to_string(),
                    weight: 100,
                    max_requests_per_minute: 60,
                }],
                base_url: "https://generativelanguage.googleapis.com".to_string(),
                timeout_seconds: 30,
            },
            auth: AuthConfig {
                enabled: true,
                jwt_secret: "very-secure-jwt-secret-key-12345678901234567890".to_string(),
                rate_limit_per_minute: 60,
                admin_password: "secure-admin-password".to_string(),
                token_expiry_hours: 24,
                refresh_token_enabled: true,
                session_timeout_minutes: 30,
                max_login_attempts: 5,
                lockout_duration_minutes: 15,
            },
            metrics: MetricsConfig {
                enabled: true,
                prometheus_port: 9090,
                tls: None,
            },
        }
    }

    #[test]
    fn test_valid_config() {
        let config = create_valid_config();
        assert!(ConfigValidator::validate_proxy_config(&config).is_ok());
    }

    #[test]
    fn test_invalid_server_port() {
        let mut config = create_valid_config();
        config.server.port = 0;

        let result = ConfigValidator::validate_proxy_config(&config);
        assert!(result.is_err());

        if let Err(GeminiProxyError::Validation { fields, .. }) = result {
            assert!(!fields.is_empty());
            assert!(fields.iter().any(|e| e.field == "server.port"));
        } else {
            panic!("期望验证错误");
        }
    }

    #[test]
    fn test_unsafe_jwt_secret() {
        let mut config = create_valid_config();
        config.auth.jwt_secret = "your-super-secret-key-that-is-long-and-secure".to_string();

        let result = ConfigValidator::validate_proxy_config(&config);
        assert!(result.is_err());

        if let Err(GeminiProxyError::Validation { fields, .. }) = result {
            assert!(fields.iter().any(|e| e.field == "auth.jwt_secret" && e.message.contains("不安全的默认")));
        } else {
            panic!("期望验证错误");
        }
    }

    #[test]
    fn test_duplicate_api_key_ids() {
        let mut config = create_valid_config();
        config.gemini.api_keys.push(ApiKeyConfig {
            id: "test-key".to_string(), // 重复的ID
            key: "another-valid-key-12345678901234567890".to_string(),
            weight: 50,
            max_requests_per_minute: 30,
        });

        let result = ConfigValidator::validate_proxy_config(&config);
        assert!(result.is_err());

        if let Err(GeminiProxyError::Validation { fields, .. }) = result {
            assert!(fields.iter().any(|e| e.message.contains("重复")));
        } else {
            panic!("期望验证错误");
        }
    }
}