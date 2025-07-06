// src/auth/handler.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use pingora::proxy::Session;
use pingora_error::Result;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use pingora::protocols::l4::socket::SocketAddr;

pub struct AuthHandler {
    jwt_secret: String,
    rate_limits: Arc<RwLock<HashMap<String, RateLimit>>>,
    rate_limit_per_minute: u32,
}

#[derive(Debug)]
struct RateLimit {
    count: u32,
    reset_time: std::time::Instant,
}

impl AuthHandler {
    pub fn new(jwt_secret: String, rate_limit_per_minute: u32) -> Self {
        Self {
            jwt_secret,
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_per_minute,
        }
    }

    pub async fn validate_request(&self, session: &mut Session) -> Result<bool> {
        let auth_header = session.req_header().headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));

        if let Some(token) = auth_header {
            let key = DecodingKey::from_secret(self.jwt_secret.as_ref());
            let validation = Validation::new(Algorithm::HS256);
            
            match decode::<serde_json::Value>(token, &key, &validation) {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    pub async fn check_rate_limit(&self, session: &mut Session) -> Result<bool> {
        let client_id = self.get_client_id(session).await;
        let mut limits = self.rate_limits.write().await;
        
        let limit = limits.entry(client_id).or_insert(RateLimit {
            count: 0,
            reset_time: std::time::Instant::now(),
        });

        if limit.reset_time.elapsed().as_secs() >= 60 {
            limit.count = 0;
            limit.reset_time = std::time::Instant::now();
        }

        if limit.count < self.rate_limit_per_minute {
            limit.count += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn get_client_id(&self, session: &mut Session) -> String {
        session.client_addr().map(|addr| {
            match addr {
                SocketAddr::Inet(inet_addr) => inet_addr.ip().to_string(),
                SocketAddr::Unix(_) => "unix_socket".to_string(),
            }
        }).unwrap_or_else(|| "unknown".to_string())
    }
}