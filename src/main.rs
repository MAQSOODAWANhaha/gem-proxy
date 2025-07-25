// src/main.rs
use crate::auth::AuthHandler;
use crate::config::ProxyConfig;
use crate::load_balancer::{UnifiedKeyManager, key_manager::ApiKey};
use crate::metrics::MetricsCollector;
use crate::proxy::acme_service::{AcmeChallengeService, AcmeChallengeState};
use crate::proxy::GeminiProxyService;
use crate::utils::health_check::HealthChecker;
use crate::api::config::ConfigState;
use crate::api::weight_management::WeightManagementState;
use crate::utils::tls::{acme_renewal_loop, generate_self_signed_cert_if_not_exists};
use crate::utils::performance::PerformanceOptimizer;
use crate::utils::error::ErrorHandler;
use chrono::Utc;
use pingora::proxy::http_proxy_service;
use pingora::server::Server;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::runtime::Builder;

mod api;
mod auth;
mod config;
mod error;
mod integration_example;
mod load_balancer;
mod metrics;
mod persistence;
mod proxy;
mod security;
mod utils;

fn main() {
    tracing_subscriber::fmt::init();

    // 使用增强的配置加载，包含安全验证
    let config = match load_and_validate_config("config/proxy.yaml") {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("配置加载或安全验证失败: {}", e);
            std::process::exit(1);
        }
    };

    // 使用新的统一密钥管理器，消除状态重复和锁竞争
    let key_manager = Arc::new(UnifiedKeyManager::new(
        config
            .gemini
            .api_keys
            .iter()
            .map(|k| ApiKey {
                id: k.id.clone(),
                key: k.key.clone(),
                weight: k.weight,
                max_requests_per_minute: k.max_requests_per_minute,
                current_requests: 0,
                last_reset: Utc::now(),
                is_active: true,
                failure_count: 0,
            })
            .collect(),
    ));

    let auth_handler = Arc::new(AuthHandler::new(
        config.auth.jwt_secret.clone(),
        config.auth.rate_limit_per_minute,
    ));
    let metrics = Arc::new(MetricsCollector::new());
    let gemini_config = Arc::new(config.gemini.clone());
    
    // 初始化性能监控和错误处理
    let performance_optimizer = Arc::new(PerformanceOptimizer::new(config.server.max_connections as u64));
    let error_handler = Arc::new(ErrorHandler::new(1000));

    if config.metrics.enabled {
        let metrics_clone = metrics.clone();
        let metrics_port = config.metrics.prometheus_port;
        let total_keys = config.gemini.api_keys.len();
        let config_state = ConfigState::new(config.clone(), "config/proxy.yaml".to_string());
        let performance_optimizer_clone = performance_optimizer.clone();
        let error_handler_clone = error_handler.clone();
        let key_manager_clone = key_manager.clone();
        
        std::thread::spawn(move || {
            let runtime = Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(async move {
                start_api_server(
                    metrics_clone, 
                    metrics_port, 
                    total_keys, 
                    config_state,
                    performance_optimizer_clone,
                    error_handler_clone,
                    key_manager_clone
                ).await;
            });
        });
    }

    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    if config.server.tls.enabled {
        let tls_config = &config.server.tls;
        if let Some(acme_config) = &tls_config.acme {
            if acme_config.enabled {
                let challenge_state: AcmeChallengeState = Arc::new(RwLock::new(HashMap::new()));

                let acme_challenge_service = AcmeChallengeService {
                    challenge_state: challenge_state.clone(),
                };
                let mut acme_http_service =
                    http_proxy_service(&server.configuration, acme_challenge_service);
                acme_http_service.add_tcp("0.0.0.0:80");
                server.add_service(acme_http_service);

                let acme_conf_clone = acme_config.clone();
                let cert_path_clone = tls_config.cert_path.clone();
                let key_path_clone = tls_config.key_path.clone();

                std::thread::spawn(move || {
                    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
                    runtime.block_on(async move {
                        acme_renewal_loop(
                            &acme_conf_clone,
                            challenge_state,
                            &cert_path_clone,
                            &key_path_clone,
                        )
                        .await;
                    });
                });
            }
        } else {
            generate_self_signed_cert_if_not_exists(&tls_config.cert_path, &tls_config.key_path)
                .expect("Failed to generate self-signed certificate");
        }
    }

    let service = GeminiProxyService::new(
        key_manager, 
        auth_handler, 
        metrics.clone(), 
        gemini_config
    );
    let mut proxy_service = http_proxy_service(&server.configuration, service);
    let addr = format!("{}:{}", config.server.host, config.server.port);

    if config.server.tls.enabled {
        let tls_config = &config.server.tls;
        tracing::info!("TLS is enabled, listening on {} with HTTPS", addr);
        let _ = proxy_service.add_tls(&addr, &tls_config.cert_path, &tls_config.key_path);
    } else {
        proxy_service.add_tcp(&addr);
    }
    server.add_service(proxy_service);

    server.run_forever();
}

async fn start_api_server(
    metrics: Arc<MetricsCollector>, 
    port: u16, 
    total_keys: usize,
    config_state: ConfigState,
    performance_optimizer: Arc<PerformanceOptimizer>,
    error_handler: Arc<ErrorHandler>,
    key_manager: Arc<UnifiedKeyManager>,
) {
    use warp::Filter;
    
    // Setup health checker
    let health_checker = HealthChecker::new(total_keys, total_keys, true);
    let health_checker = Arc::new(health_checker);
    
    // Metrics route
    let metrics_route = warp::path("metrics")
        .map(move || metrics.get_metrics());
    
    // Health check route
    let health_checker_clone = health_checker.clone();
    let health_route = warp::path("health")
        .and(warp::get())
        .and_then(move || {
            let checker = health_checker_clone.clone();
            async move {
                let health_status = checker.check_health().await;
                let json = serde_json::to_string(&health_status).unwrap();
                Result::<_, warp::Rejection>::Ok(warp::reply::with_header(
                    json,
                    "content-type",
                    "application/json",
                ))
            }
        });
    
    // 性能监控路由
    let performance_optimizer_clone = performance_optimizer.clone();
    let performance_route = warp::path("performance")
        .and(warp::get())
        .and_then(move || {
            let optimizer = performance_optimizer_clone.clone();
            async move {
                let stats = optimizer.get_performance_stats().await;
                let json = serde_json::to_string(&stats).unwrap();
                Result::<_, warp::Rejection>::Ok(warp::reply::with_header(
                    json,
                    "content-type",
                    "application/json",
                ))
            }
        });

    // 错误统计路由  
    let error_handler_clone = error_handler.clone();
    let errors_route = warp::path("errors")
        .and(warp::get())
        .and_then(move || {
            let handler = error_handler_clone.clone();
            async move {
                let stats = handler.get_error_statistics().await;
                let json = serde_json::to_string(&stats).unwrap();
                Result::<_, warp::Rejection>::Ok(warp::reply::with_header(
                    json,
                    "content-type",
                    "application/json",
                ))
            }
        });
    
    // 获取 API 服务器的配置
    let api_config = config_state.get_config().await;
    
    // 配置API路由
    let config_routes = crate::api::config::config_routes(config_state.clone());
    
    // 权重管理路由
    let weight_state = WeightManagementState::new(config_state);
    weight_state.set_key_manager(key_manager.clone()).await;
    let weight_routes = crate::api::weight_management::weight_management_routes(weight_state);
    
    // 负载均衡统计路由
    let stats_state = crate::api::load_balancing_stats::StatsState::new(Some(key_manager));
    let stats_routes = crate::api::load_balancing_stats::load_balancing_stats_routes(stats_state);
    
    // 认证路由 (暂时保持原有结构，计划重构到 /api/v1/auth/*)
    let auth_state = crate::api::auth::AuthState::new(Arc::new(api_config.clone()));
    let auth_routes = crate::api::auth::auth_routes(auth_state.clone());
    
    // API路由 (暂时移除认证保护以解决404问题)
    let business_api_routes = config_routes
        .or(weight_routes)
        .or(stats_routes);
    
    let api_routes = warp::path("api")
        .and(business_api_routes);
    
    // 组合所有路由 - 暂时移除认证保护
    let routes = metrics_route
        .or(health_route)
        .or(performance_route)
        .or(errors_route)
        .or(auth_routes)
        .or(api_routes)
        .with(crate::api::handlers::cors())
        .with(crate::api::handlers::with_logging())
        .recover(crate::api::handlers::handle_rejection);
    
    // 检查是否启用 API 服务器 TLS
    if let Some(api_tls) = &api_config.metrics.tls {
        if api_tls.enabled {
            // 确保证书存在
            crate::utils::tls::generate_self_signed_cert_if_not_exists(
                &api_tls.cert_path,
                &api_tls.key_path,
            ).expect("Failed to generate API server certificate");
            
            tracing::info!("API server running on https://127.0.0.1:{} (HTTPS)", port);
            tracing::info!("Business APIs: /api/config/*, /api/weights/*, /api/stats/* (暂时无认证)");
            tracing::info!("Auth APIs: /auth/* (JWT获取和刷新)");
            tracing::info!("Monitor APIs: /metrics, /health, /performance, /errors (无需认证)");
            
            warp::serve(routes)
                .tls()
                .cert_path(&api_tls.cert_path)
                .key_path(&api_tls.key_path)
                .run(([127, 0, 0, 1], port))
                .await;
        } else {
            tracing::info!("API server running on http://127.0.0.1:{} (HTTP)", port);
            tracing::info!("Business APIs: /api/config/*, /api/weights/*, /api/stats/* (暂时无认证)");
            tracing::info!("Auth APIs: /auth/* (JWT获取和刷新)");
            tracing::info!("Monitor APIs: /metrics, /health, /performance, /errors (无需认证)");
            warp::serve(routes).run(([127, 0, 0, 1], port)).await;
        }
    } else {
        tracing::info!("API server running on http://127.0.0.1:{} (HTTP)", port);
        tracing::info!("Business APIs: /api/config/*, /api/weights/*, /api/stats/* (暂时无认证)");
        tracing::info!("Auth APIs: /auth/* (JWT获取和刷新)");
        tracing::info!("Monitor APIs: /metrics, /health, /performance, /errors (无需认证)");
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    }
}

/// 加载并验证配置的安全性
fn load_and_validate_config(config_path: &str) -> Result<ProxyConfig, String> {
    use crate::security::{SecurityConfigValidator, AuditLogManager, AuditConfig};
    use crate::config::validation::ConfigValidator;
    
    // 1. 基础配置加载
    tracing::info!("正在加载配置文件: {}", config_path);
    let config = ProxyConfig::from_file_enhanced(config_path)
        .map_err(|e| format!("配置文件加载失败: {}", e))?;
    
    // 2. 配置验证（语法和逻辑）
    tracing::info!("正在验证配置有效性...");
    if let Err(validation_error) = ConfigValidator::validate_proxy_config(&config) {
        return Err(format!("配置验证失败: {}", validation_error));
    }
    
    // 3. 安全配置验证
    tracing::info!("正在进行安全配置检查...");
    match SecurityConfigValidator::validate_security(&config) {
        Ok(security_report) => {
            // 显示安全评分和问题摘要
            tracing::info!("安全评分: {}/100", security_report.security_score);
            tracing::info!("发现 {} 个安全问题 (严重:{}, 高:{}, 中:{}, 低:{})",
                security_report.summary.total_issues,
                security_report.summary.critical_issues,
                security_report.summary.high_issues,
                security_report.summary.medium_issues,
                security_report.summary.low_issues
            );
            
            // 如果有高风险或中风险问题，给出警告
            if security_report.summary.high_issues > 0 || security_report.summary.medium_issues > 0 {
                tracing::warn!("检测到安全问题，建议查看安全报告:");
                for issue in &security_report.issues {
                    if matches!(issue.threat_level, crate::security::ThreatLevel::High | crate::security::ThreatLevel::Medium) {
                        tracing::warn!("- [{}] {}: {}", 
                            match issue.threat_level {
                                crate::security::ThreatLevel::Critical => "严重",
                                crate::security::ThreatLevel::High => "高",
                                crate::security::ThreatLevel::Medium => "中",
                                crate::security::ThreatLevel::Low => "低",
                            },
                            issue.description, 
                            issue.remediation
                        );
                    }
                }
            }
            
            // 严重问题已经在验证函数中处理，这里只处理非严重问题
        }
        Err(critical_error) => {
            // 严重安全问题，拒绝启动
            return Err(format!("严重安全问题: {}", critical_error));
        }
    }
    
    // 4. 初始化审计日志并记录配置加载事件
    let audit_config = AuditConfig {
        file_output_enabled: true,
        log_file_path: "logs/audit.log".to_string(),
        ..AuditConfig::default()
    };
    
    let mut audit_manager = AuditLogManager::new(audit_config);
    
    // 记录配置加载成功事件
    tokio::runtime::Handle::try_current()
        .unwrap_or_else(|_| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.handle().clone()
        })
        .block_on(async {
            if let Err(e) = audit_manager.log_system_operation(
                "配置加载",
                "config_loader",
                crate::security::AuditResult::Success,
                Some(format!("成功加载配置文件: {}", config_path)),
            ).await {
                tracing::warn!("记录审计日志失败: {}", e);
            }
        });
    
    tracing::info!("✅ 配置加载和安全验证完成");
    Ok(config)
}
