// src/proxy/service.rs
use async_trait::async_trait;
use pingora::proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use pingora_error::{Result, Error};
use pingora::http::ResponseHeader;
use pingora::protocols::l4::socket::SocketAddr;
use std::sync::Arc;
use chrono::Utc;
use crate::load_balancer::KeyManager;
use crate::auth::AuthHandler;
use crate::metrics::MetricsCollector;
use crate::config::GeminiConfig;

pub struct ProxyCtx {
    pub api_key_id: Option<String>,
    pub request_start_time: Option<chrono::DateTime<Utc>>,
}

pub struct GeminiProxyService {
    key_manager: Arc<KeyManager>,
    auth_handler: Arc<AuthHandler>,
    metrics: Arc<MetricsCollector>,
    gemini_config: Arc<GeminiConfig>,
}

impl GeminiProxyService {
    pub fn new(
        key_manager: Arc<KeyManager>,
        auth_handler: Arc<AuthHandler>,
        metrics: Arc<MetricsCollector>,
        gemini_config: Arc<GeminiConfig>,
    ) -> Self {
        Self {
            key_manager,
            auth_handler,
            metrics,
            gemini_config,
        }
    }
}

#[async_trait]
impl ProxyHttp for GeminiProxyService {
    type CTX = ProxyCtx;

    fn new_ctx(&self) -> Self::CTX {
        ProxyCtx {
            api_key_id: None,
            request_start_time: None,
        }
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        ctx.request_start_time = Some(Utc::now());

        if !self.auth_handler.validate_request(session).await? {
            session.respond_error(401).await?;
            return Ok(true);
        }

        if !self.auth_handler.check_rate_limit(session).await? {
            session.respond_error(429).await?;
            return Ok(true);
        }

        if let Some(api_key) = self.key_manager.get_next_key().await {
            session.req_header_mut().insert_header("x-goog-api-key", &api_key.key)?;
            ctx.api_key_id = Some(api_key.id.clone());
            self.metrics.increment_request_count(&api_key.id).await;
        } else {
            session.respond_error(503).await?;
            return Ok(true);
        }

        Ok(false)
    }

    async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> Result<Box<HttpPeer>> {
        let peer = Box::new(HttpPeer::new(
            self.gemini_config.base_url.clone(),
            true, // HTTPS
            self.gemini_config.base_url.split(':').next().unwrap_or("").to_string(),
        ));
        Ok(peer)
    }

    async fn response_filter(&self, _session: &mut Session, _response_header: &mut ResponseHeader, ctx: &mut Self::CTX) -> Result<()> {
        let status = _session.response_written().map(|r| r.status.as_u16()).unwrap_or(0);
        let response_time = ctx.request_start_time.map_or_else(
            || std::time::Duration::from_secs(0),
            |start| (Utc::now() - start).to_std().unwrap_or_default()
        );

        self.metrics.record_response(status, response_time).await;

        if let Some(key_id) = &ctx.api_key_id {
            if (200..300).contains(&status) {
                self.key_manager.mark_key_success(key_id).await;
            } else if status >= 400 {
                self.key_manager.mark_key_failed(key_id).await;
            }
        }
        Ok(())
    }

    async fn logging(&self, session: &mut Session, _e: Option<&Error>, ctx: &mut Self::CTX) {
        let response_time = ctx.request_start_time.map_or(0, |start| (Utc::now() - start).num_milliseconds());
        let client_ip = session.client_addr().map(|addr| {
            match addr {
                SocketAddr::Inet(inet_addr) => inet_addr.ip().to_string(),
                SocketAddr::Unix(_) => "unix_socket".to_string(),
            }
        }).unwrap_or_else(|| "unknown".to_string());

        tracing::info!(
            method = %session.req_header().method,
            uri = %session.req_header().uri,
            status = session.response_written().map(|r| r.status.as_u16()),
            client_ip = %client_ip,
            api_key_id = ctx.api_key_id.as_deref().unwrap_or("N/A"),
            processing_time_ms = response_time,
        );
    }
}