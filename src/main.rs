// src/main.rs
use pingora::server::Server;
use pingora::proxy::http_proxy_service;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use crate::config::ProxyConfig;
use crate::proxy::GeminiProxyService;
use crate::proxy::acme_service::{AcmeChallengeService, AcmeChallengeState};
use crate::load_balancer::KeyManager;
use crate::auth::AuthHandler;
use crate::metrics::MetricsCollector;
use crate::utils::tls::{generate_self_signed_cert_if_not_exists, acme_renewal_loop};
use chrono::Utc;
use tokio::runtime::Builder;

mod config;
mod proxy;
mod load_balancer;
mod auth;
mod metrics;
mod utils;

fn main() {
    tracing_subscriber::fmt::init();

    let config = ProxyConfig::from_file("config/proxy.yaml")
        .expect("Failed to load configuration");

    let key_manager = Arc::new(KeyManager::new(
        config.gemini.api_keys.iter().map(|k| {
            load_balancer::ApiKey {
                id: k.id.clone(),
                key: k.key.clone(),
                weight: k.weight,
                max_requests_per_minute: k.max_requests_per_minute,
                current_requests: 0,
                last_reset: Utc::now(),
                is_active: true,
                failure_count: 0,
            }
        }).collect()
    ));

    let auth_handler = Arc::new(AuthHandler::new(
        config.auth.jwt_secret.clone(),
        config.auth.rate_limit_per_minute,
    ));
    let metrics = Arc::new(MetricsCollector::new());
    let gemini_config = Arc::new(config.gemini.clone());

    if config.metrics.enabled {
        let metrics_clone = metrics.clone();
        let metrics_port = config.metrics.prometheus_port;
        std::thread::spawn(move || {
            let runtime = Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(async move {
                start_metrics_server(metrics_clone, metrics_port).await;
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
                
                let acme_challenge_service = AcmeChallengeService { challenge_state: challenge_state.clone() };
                let mut acme_http_service = http_proxy_service(&server.configuration, acme_challenge_service);
                acme_http_service.add_tcp("0.0.0.0:80");
                server.add_service(acme_http_service);
                
                let acme_conf_clone = acme_config.clone();
                let cert_path_clone = tls_config.cert_path.clone();
                let key_path_clone = tls_config.key_path.clone();

                std::thread::spawn(move || {
                    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
                    runtime.block_on(async move {
                        acme_renewal_loop(&acme_conf_clone, challenge_state, &cert_path_clone, &key_path_clone).await;
                    });
                });
            }
        } else {
            generate_self_signed_cert_if_not_exists(&tls_config.cert_path, &tls_config.key_path)
                .expect("Failed to generate self-signed certificate");
        }
    }

    let service = GeminiProxyService::new(key_manager, auth_handler, metrics.clone(), gemini_config);
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

async fn start_metrics_server(metrics: Arc<MetricsCollector>, port: u16) {
    use warp::Filter;
    let metrics_route = warp::path("metrics").map(move || metrics.get_metrics());
    tracing::info!("Metrics server running on 127.0.0.1:{}", port);
    warp::serve(metrics_route).run(([127, 0, 0, 1], port)).await;
}
