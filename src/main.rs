// src/main.rs
use pingora::server::Server;
use pingora::proxy::http_proxy_service;
use std::sync::Arc;
use crate::config::ProxyConfig;
use crate::proxy::GeminiProxyService;
use crate::load_balancer::KeyManager;
use crate::auth::AuthHandler;
use crate::metrics::MetricsCollector;
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
            let runtime = Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                start_metrics_server(metrics_clone, metrics_port).await;
            });
        });
    }

    let service = GeminiProxyService::new(
        key_manager,
        auth_handler,
        metrics.clone(),
        gemini_config,
    );

    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let mut proxy_service = http_proxy_service(&server.configuration, service);
    proxy_service.add_tcp(&format!("{}:{}", config.server.host, config.server.port));

    server.add_service(proxy_service);
    server.run_forever();
}

async fn start_metrics_server(metrics: Arc<MetricsCollector>, port: u16) {
    use warp::Filter;
    
    let metrics_route = warp::path("metrics")
        .map(move || metrics.get_metrics());

    tracing::info!("Metrics server running on 127.0.0.1:{}", port);
    warp::serve(metrics_route)
        .run(([127, 0, 0, 1], port))
        .await;
}
