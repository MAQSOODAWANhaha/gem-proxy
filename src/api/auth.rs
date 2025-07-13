// src/api/auth.rs
use crate::config::ProxyConfig;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, decode, Header, Algorithm, EncodingKey, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Reply};
// 暂时注释掉未使用的导入
// use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
// use argon2::password_hash::{rand_core::OsRng, SaltString};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // 用户ID
    pub exp: usize,         // 过期时间
    pub iat: usize,         // 签发时间
    pub role: String,       // 用户角色
    pub session_id: String, // 会话ID
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub session_id: String,
}

#[derive(Debug)]
pub struct LoginAttempt {
    pub count: u32,
    pub last_attempt: chrono::DateTime<Utc>,
    pub locked_until: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug)]
pub struct Session {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub user_id: String,
    #[allow(dead_code)]
    pub created_at: chrono::DateTime<Utc>,
    pub last_activity: chrono::DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct AuthState {
    config: Arc<ProxyConfig>,
    login_attempts: Arc<RwLock<HashMap<String, LoginAttempt>>>,
    active_sessions: Arc<RwLock<HashMap<String, Session>>>,
    refresh_tokens: Arc<RwLock<HashMap<String, String>>>, // refresh_token -> session_id
}

impl AuthState {
    pub fn new(config: Arc<ProxyConfig>) -> Self {
        Self {
            config,
            login_attempts: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            refresh_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // 检查是否被锁定
    pub async fn is_locked(&self, client_ip: &str) -> bool {
        let attempts = self.login_attempts.read().await;
        if let Some(attempt) = attempts.get(client_ip) {
            if let Some(locked_until) = attempt.locked_until {
                return Utc::now() < locked_until;
            }
        }
        false
    }

    // 记录登录尝试
    pub async fn record_login_attempt(&self, client_ip: &str, success: bool) {
        let mut attempts = self.login_attempts.write().await;
        let now = Utc::now();
        
        let attempt = attempts.entry(client_ip.to_string()).or_insert(LoginAttempt {
            count: 0,
            last_attempt: now,
            locked_until: None,
        });

        if success {
            // 成功登录，重置计数
            attempt.count = 0;
            attempt.locked_until = None;
        } else {
            // 失败登录，增加计数
            attempt.count += 1;
            attempt.last_attempt = now;
            
            if attempt.count >= self.config.auth.max_login_attempts {
                // 锁定账户
                attempt.locked_until = Some(now + Duration::minutes(
                    self.config.auth.lockout_duration_minutes as i64
                ));
            }
        }
    }

    // 验证密码
    pub fn verify_password(&self, password: &str) -> bool {
        // 这里简化处理，实际应该使用哈希验证
        password == self.config.auth.admin_password
    }

    // 生成JWT token
    pub fn generate_token(&self, session_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let expiration = Utc::now()
            + Duration::hours(self.config.auth.token_expiry_hours as i64);
        
        let claims = Claims {
            sub: "admin".to_string(),
            exp: expiration.timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
            role: "admin".to_string(),
            session_id: session_id.to_string(),
        };

        let key = EncodingKey::from_secret(self.config.auth.jwt_secret.as_ref());
        encode(&Header::default(), &claims, &key)
    }

    // 生成刷新token
    pub fn generate_refresh_token(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..32)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect()
    }

    // 验证JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let key = DecodingKey::from_secret(self.config.auth.jwt_secret.as_ref());
        let validation = Validation::new(Algorithm::HS256);
        
        let token_data = decode::<Claims>(token, &key, &validation)?;
        Ok(token_data.claims)
    }

    // 创建会话
    pub async fn create_session(&self, user_id: &str) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let session = Session {
            id: session_id.clone(),
            user_id: user_id.to_string(),
            created_at: now,
            last_activity: now,
            is_active: true,
        };

        self.active_sessions.write().await.insert(session_id.clone(), session);
        session_id
    }

    // 验证会话
    pub async fn validate_session(&self, session_id: &str) -> bool {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if session.is_active {
                let now = Utc::now();
                let timeout = Duration::minutes(self.config.auth.session_timeout_minutes as i64);
                
                if now - session.last_activity < timeout {
                    // 更新最后活动时间
                    session.last_activity = now;
                    return true;
                } else {
                    // 会话超时
                    session.is_active = false;
                }
            }
        }
        false
    }

    // 删除会话
    pub async fn remove_session(&self, session_id: &str) {
        self.active_sessions.write().await.remove(session_id);
        
        // 同时删除相关的刷新token
        let mut refresh_tokens = self.refresh_tokens.write().await;
        refresh_tokens.retain(|_, sid| sid != session_id);
    }

    // 清理过期会话
    #[allow(dead_code)]
    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.active_sessions.write().await;
        let now = Utc::now();
        let timeout = Duration::minutes(self.config.auth.session_timeout_minutes as i64);
        
        sessions.retain(|_, session| {
            if now - session.last_activity < timeout {
                true
            } else {
                false
            }
        });
    }
}

// 获取客户端IP
fn get_client_ip(headers: &warp::http::HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|hv| hv.to_str().ok())
        .and_then(|s| s.split(',').next())
        .unwrap_or("127.0.0.1")
        .trim()
        .to_string()
}

// 登录处理
async fn handle_login(
    login_req: LoginRequest,
    auth_state: AuthState,
    headers: warp::http::HeaderMap,
) -> Result<impl Reply, warp::Rejection> {
    let client_ip = get_client_ip(&headers);
    
    // 检查是否被锁定
    if auth_state.is_locked(&client_ip).await {
        auth_state.record_login_attempt(&client_ip, false).await;
        return Ok(warp::reply::with_status(
            warp::reply::json(&LoginResponse {
                success: false,
                token: None,
                refresh_token: None,
                expires_in: None,
                message: "账户已被锁定，请稍后再试".to_string(),
            }),
            warp::http::StatusCode::TOO_MANY_REQUESTS,
        ));
    }

    // 验证密码
    if !auth_state.verify_password(&login_req.password) {
        auth_state.record_login_attempt(&client_ip, false).await;
        return Ok(warp::reply::with_status(
            warp::reply::json(&LoginResponse {
                success: false,
                token: None,
                refresh_token: None,
                expires_in: None,
                message: "密码错误".to_string(),
            }),
            warp::http::StatusCode::UNAUTHORIZED,
        ));
    }

    // 登录成功
    auth_state.record_login_attempt(&client_ip, true).await;
    
    // 创建会话
    let session_id = auth_state.create_session("admin").await;
    
    // 生成tokens
    match auth_state.generate_token(&session_id) {
        Ok(token) => {
            let mut response = LoginResponse {
                success: true,
                token: Some(token),
                refresh_token: None,
                expires_in: Some(auth_state.config.auth.token_expiry_hours * 3600),
                message: "登录成功".to_string(),
            };

            // 如果启用了刷新token
            if auth_state.config.auth.refresh_token_enabled {
                let refresh_token = auth_state.generate_refresh_token();
                auth_state.refresh_tokens.write().await.insert(
                    refresh_token.clone(),
                    session_id.clone(),
                );
                response.refresh_token = Some(refresh_token);
            }

            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&LoginResponse {
                success: false,
                token: None,
                refresh_token: None,
                expires_in: None,
                message: "token生成失败".to_string(),
            }),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

// 刷新token处理
async fn handle_refresh(
    refresh_req: RefreshRequest,
    auth_state: AuthState,
) -> Result<impl Reply, warp::Rejection> {
    let refresh_tokens = auth_state.refresh_tokens.read().await;
    
    if let Some(session_id) = refresh_tokens.get(&refresh_req.refresh_token) {
        if auth_state.validate_session(session_id).await {
            match auth_state.generate_token(session_id) {
                Ok(new_token) => {
                    return Ok(warp::reply::with_status(
                        warp::reply::json(&LoginResponse {
                            success: true,
                            token: Some(new_token),
                            refresh_token: None,
                            expires_in: Some(auth_state.config.auth.token_expiry_hours * 3600),
                            message: "token刷新成功".to_string(),
                        }),
                        warp::http::StatusCode::OK,
                    ));
                }
                Err(_) => {}
            }
        }
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&LoginResponse {
            success: false,
            token: None,
            refresh_token: None,
            expires_in: None,
            message: "刷新token无效或已过期".to_string(),
        }),
        warp::http::StatusCode::UNAUTHORIZED,
    ))
}

// 登出处理
async fn handle_logout(
    logout_req: LogoutRequest,
    auth_state: AuthState,
) -> Result<impl Reply, warp::Rejection> {
    auth_state.remove_session(&logout_req.session_id).await;
    
    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": true,
            "message": "登出成功"
        })),
        warp::http::StatusCode::OK,
    ))
}

// 验证token处理
async fn handle_verify(
    token: String,
    auth_state: AuthState,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    match auth_state.verify_token(&token) {
        Ok(claims) => {
            if auth_state.validate_session(&claims.session_id).await {
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "valid": true,
                        "claims": claims
                    })),
                    warp::http::StatusCode::OK,
                ))
            } else {
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "valid": false,
                        "message": "会话已过期"
                    })),
                    warp::http::StatusCode::UNAUTHORIZED,
                ))
            }
        }
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "valid": false,
                "message": "token无效"
            })),
            warp::http::StatusCode::UNAUTHORIZED,
        )),
    }
}

// 认证中间件
pub fn auth_middleware(
    auth_state: AuthState,
) -> impl Filter<Extract = (Claims,), Error = warp::Rejection> + Clone {
    warp::header::<String>("authorization")
        .and_then(move |auth_header: String| {
            let auth_state = auth_state.clone();
            async move {
                if let Some(token) = auth_header.strip_prefix("Bearer ") {
                    match auth_state.verify_token(token) {
                        Ok(claims) => {
                            if auth_state.validate_session(&claims.session_id).await {
                                Ok(claims)
                            } else {
                                Err(warp::reject::custom(AuthError::SessionExpired))
                            }
                        }
                        Err(_) => Err(warp::reject::custom(AuthError::InvalidToken)),
                    }
                } else {
                    Err(warp::reject::custom(AuthError::MissingToken))
                }
            }
        })
}

// 认证错误类型
#[derive(Debug)]
pub enum AuthError {
    InvalidToken,
    MissingToken,
    SessionExpired,
}

impl warp::reject::Reject for AuthError {}

// 认证路由
pub fn auth_routes(
    auth_state: AuthState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let auth_state_filter = warp::any().map(move || auth_state.clone());

    let login = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and(auth_state_filter.clone())
        .and(warp::header::headers_cloned())
        .and_then(handle_login);

    let refresh = warp::path("refresh")
        .and(warp::post())
        .and(warp::body::json())
        .and(auth_state_filter.clone())
        .and_then(handle_refresh);

    let logout = warp::path("logout")
        .and(warp::post())
        .and(warp::body::json())
        .and(auth_state_filter.clone())
        .and_then(handle_logout);

    let verify = warp::path("verify")
        .and(warp::post())
        .and(warp::body::json())
        .and(auth_state_filter.clone())
        .and_then(|token_req: serde_json::Value, auth_state: AuthState| async move {
            if let Some(token) = token_req.get("token").and_then(|t| t.as_str()) {
                handle_verify(token.to_string(), auth_state).await
            } else {
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "valid": false,
                        "message": "缺少token参数"
                    })),
                    warp::http::StatusCode::BAD_REQUEST,
                ))
            }
        });

    warp::path("auth")
        .and(login.or(refresh).or(logout).or(verify))
}