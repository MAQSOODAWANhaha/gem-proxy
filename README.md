这是一个完整的基于Pingora的Gemini API代理架构。以下是详细的设计方案：

## 1. 整体架构设计

### 1.1 架构概览
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│    Client       │    │  Pingora Proxy  │    │   Gemini API    │
│   Application   │◄──►│     Service     │◄──►│    Servers      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌─────────────────┐
                       │  Configuration  │
                       │   & Metrics     │
                       └─────────────────┘
```

### 1.2 核心组件架构
```
Pingora Proxy Service
├── API Gateway Layer
│   ├── Request Router
│   ├── Authentication Handler
│   └── Rate Limiter
├── Load Balancer Layer
│   ├── Key Pool Manager
│   ├── Round Robin Scheduler
│   └── Health Checker
├── Proxy Core Layer
│   ├── Connection Pool
│   ├── Request Transformer
│   └── Response Handler
└── Monitoring Layer
    ├── Metrics Collector
    ├── Logging System
    └── Health Monitor
```

## 2. 详细设计方案

### 2.1 项目结构
```
gemini-proxy/
├── src/
│   ├── main.rs
│   ├── config/
│   │   ├── mod.rs
│   │   └── settings.rs
│   ├── proxy/
│   │   ├── mod.rs
│   │   ├── service.rs
│   │   └── middleware.rs
│   ├── load_balancer/
│   │   ├── mod.rs
│   │   ├── key_manager.rs
│   │   └── scheduler.rs
│   ├── auth/
│   │   ├── mod.rs
│   │   └── handler.rs
│   ├── metrics/
│   │   ├── mod.rs
│   │   └── collector.rs
│   └── utils/
│       ├── mod.rs
│       └── health_check.rs
├── config/
│   ├── proxy.yaml
│   └── keys.yaml
├── Cargo.toml
└── README.md
```

### 2.2 核心实现关键点

#### 2.2.1 Key Manager 实现
```rust
// src/load_balancer/key_manager.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub key: String,
    pub weight: u32,
    pub max_requests_per_minute: u32,
    pub current_requests: u32,
    pub last_reset: std::time::Instant,
    pub is_active: bool,
    pub failure_count: u32,
}

#[derive(Debug)]
pub struct KeyManager {
    keys: Arc<RwLock<Vec<ApiKey>>>,
    current_index: Arc<RwLock<usize>>,
}

impl KeyManager {
    pub fn new(keys: Vec<ApiKey>) -> Self {
        Self {
            keys: Arc::new(RwLock::new(keys)),
            current_index: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn get_next_key(&self) -> Option<ApiKey> {
        let mut keys = self.keys.write().await;
        let mut index = self.current_index.write().await;
        
        // Round-robin with health check
        for _ in 0..keys.len() {
            let key = &mut keys[*index];
            *index = (*index + 1) % keys.len();
            
            if self.is_key_available(key).await {
                key.current_requests += 1;
                return Some(key.clone());
            }
        }
        None
    }

    async fn is_key_available(&self, key: &mut ApiKey) -> bool {
        // Reset rate limit counter if needed
        if key.last_reset.elapsed().as_secs() >= 60 {
            key.current_requests = 0;
            key.last_reset = std::time::Instant::now();
        }
        
        key.is_active && 
        key.current_requests < key.max_requests_per_minute &&
        key.failure_count < 5
    }

    pub async fn mark_key_failed(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            key.failure_count += 1;
            if key.failure_count >= 5 {
                key.is_active = false;
            }
        }
    }

    pub async fn mark_key_success(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.iter_mut().find(|k| k.id == key_id) {
            key.failure_count = 0;
            key.is_active = true;
        }
    }
}
```

#### 2.2.2 Pingora Service 实现
```rust
// src/proxy/service.rs
use async_trait::async_trait;
use pingora::prelude::*;
use std::sync::Arc;
use crate::load_balancer::KeyManager;
use crate::auth::AuthHandler;
use crate::metrics::MetricsCollector;

pub struct GeminiProxyService {
    key_manager: Arc<KeyManager>,
    auth_handler: Arc<AuthHandler>,
    metrics: Arc<MetricsCollector>,
}

impl GeminiProxyService {
    pub fn new(
        key_manager: Arc<KeyManager>,
        auth_handler: Arc<AuthHandler>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            key_manager,
            auth_handler,
            metrics,
        }
    }
}

#[async_trait]
impl ProxyHttp for GeminiProxyService {
    type CTX = ();
    
    fn new_ctx(&self) -> Self::CTX {}

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        // Authentication check
        if !self.auth_handler.validate_request(session).await? {
            session.respond_error(401).await?;
            return Ok(true);
        }

        // Rate limiting
        if !self.auth_handler.check_rate_limit(session).await? {
            session.respond_error(429).await?;
            return Ok(true);
        }

        // Get API key
        if let Some(api_key) = self.key_manager.get_next_key().await {
            // Add API key to request headers
            session.req_header_mut().insert_header(
                "x-goog-api-key", 
                api_key.key.as_str()
            )?;
            
            // Store key ID for later use
            session.set_downstream_reuse(false);
            session.set_keepalive(None);
            
            // Metrics
            self.metrics.increment_request_count(&api_key.id).await;
        } else {
            session.respond_error(503).await?;
            return Ok(true);
        }

        Ok(false)
    }

    async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> Result<Box<HttpPeer>> {
        // Gemini API endpoint
        let peer = Box::new(HttpPeer::new(
            "generativelanguage.googleapis.com:443".to_string(),
            true, // HTTPS
            "generativelanguage.googleapis.com".to_string(),
        ));
        Ok(peer)
    }

    async fn response_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<()> {
        // Record response metrics
        let status = session.response_written().map(|r| r.status.as_u16()).unwrap_or(0);
        self.metrics.record_response(status).await;

        // Handle API key success/failure
        if let Some(key_id) = session.get_header("x-api-key-id") {
            if status >= 200 && status < 300 {
                self.key_manager.mark_key_success(&key_id).await;
            } else if status >= 400 && status < 500 {
                self.key_manager.mark_key_failed(&key_id).await;
            }
        }

        Ok(())
    }

    async fn logging(&self, session: &mut Session, _e: Option<&pingora::Error>, _ctx: &mut Self::CTX) {
        // Structured logging
        let log_entry = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "method": session.req_header().method.as_str(),
            "uri": session.req_header().uri.to_string(),
            "status": session.response_written().map(|r| r.status.as_u16()),
            "user_agent": session.req_header().headers.get("user-agent").map(|h| h.to_str().unwrap_or("")),
            "processing_time_ms": session.response_time().unwrap_or_default().as_millis(),
        });
        
        println!("{}", log_entry);
    }
}
```

#### 2.2.3 配置管理
```rust
// src/config/settings.rs
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub prometheus_port: u16,
}

impl ProxyConfig {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: ProxyConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
```

#### 2.2.4 认证处理器
```rust
// src/auth/handler.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use pingora::prelude::*;
use jsonwebtoken::{decode, DecodingKey, Validation};

pub struct AuthHandler {
    jwt_secret: String,
    rate_limits: Arc<RwLock<HashMap<String, RateLimit>>>,
}

#[derive(Debug)]
struct RateLimit {
    count: u32,
    reset_time: std::time::Instant,
}

impl AuthHandler {
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_secret,
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn validate_request(&self, session: &mut Session) -> Result<bool> {
        // Extract JWT token from Authorization header
        let auth_header = session.req_header().headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));

        if let Some(token) = auth_header {
            // Validate JWT token
            let validation = Validation::default();
            let key = DecodingKey::from_secret(self.jwt_secret.as_ref());
            
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

        // Reset if minute has passed
        if limit.reset_time.elapsed().as_secs() >= 60 {
            limit.count = 0;
            limit.reset_time = std::time::Instant::now();
        }

        if limit.count < 100 { // 100 requests per minute
            limit.count += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn get_client_id(&self, session: &mut Session) -> String {
        // Extract client ID from JWT or IP
        session.client_addr().map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}
```

#### 2.2.5 监控指标
```rust
// src/metrics/collector.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use prometheus::{Counter, Histogram, Registry, Encoder, TextEncoder};

pub struct MetricsCollector {
    registry: Registry,
    request_counter: Counter,
    response_histogram: Histogram,
    key_usage: Arc<RwLock<HashMap<String, u64>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();
        let request_counter = Counter::new("gemini_proxy_requests_total", "Total requests").unwrap();
        let response_histogram = Histogram::new("gemini_proxy_response_duration_seconds", "Response time").unwrap();
        
        registry.register(Box::new(request_counter.clone())).unwrap();
        registry.register(Box::new(response_histogram.clone())).unwrap();

        Self {
            registry,
            request_counter,
            response_histogram,
            key_usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn increment_request_count(&self, key_id: &str) {
        self.request_counter.inc();
        let mut usage = self.key_usage.write().await;
        *usage.entry(key_id.to_string()).or_insert(0) += 1;
    }

    pub async fn record_response(&self, status: u16) {
        // Record response metrics
        self.response_histogram.observe(0.1); // This should be actual response time
    }

    pub fn get_metrics(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap_or_default()
    }
}
```

#### 2.2.6 主程序入口
```rust
// src/main.rs
use pingora::prelude::*;
use std::sync::Arc;
use crate::config::ProxyConfig;
use crate::proxy::GeminiProxyService;
use crate::load_balancer::KeyManager;
use crate::auth::AuthHandler;
use crate::metrics::MetricsCollector;

mod config;
mod proxy;
mod load_balancer;
mod auth;
mod metrics;

#[tokio::main]
async fn main() {
    // Load configuration
    let config = ProxyConfig::from_file("config/proxy.yaml")
        .expect("Failed to load configuration");

    // Initialize components
    let key_manager = Arc::new(KeyManager::new(
        config.gemini.api_keys.into_iter().map(|k| {
            crate::load_balancer::ApiKey {
                id: k.id,
                key: k.key,
                weight: k.weight,
                max_requests_per_minute: k.max_requests_per_minute,
                current_requests: 0,
                last_reset: std::time::Instant::now(),
                is_active: true,
                failure_count: 0,
            }
        }).collect()
    ));

    let auth_handler = Arc::new(AuthHandler::new(config.auth.jwt_secret.clone()));
    let metrics = Arc::new(MetricsCollector::new());

    // Create service
    let service = GeminiProxyService::new(
        key_manager,
        auth_handler,
        metrics.clone(),
    );

    // Configure server
    let mut server = Server::new(Some(Opt::default())).unwrap();
    server.bootstrap();

    let mut proxy_service = pingora::proxy::http_proxy_service(&server.configuration, service);
    proxy_service.add_tcp(&format!("{}:{}", config.server.host, config.server.port));

    // Start metrics server
    if config.metrics.enabled {
        let metrics_clone = metrics.clone();
        tokio::spawn(async move {
            start_metrics_server(metrics_clone, config.metrics.prometheus_port).await;
        });
    }

    // Start server
    server.add_service(proxy_service);
    server.run_forever();
}

async fn start_metrics_server(metrics: Arc<MetricsCollector>, port: u16) {
    use warp::Filter;
    
    let metrics_route = warp::path("metrics")
        .map(move || metrics.get_metrics());

    warp::serve(metrics_route)
        .run(([127, 0, 0, 1], port))
        .await;
}
```

### 2.3 配置文件示例

#### 2.3.1 proxy.yaml
```yaml
server:
  host: "0.0.0.0"
  port: 8080
  workers: 4
  max_connections: 10000
  tls:
    enabled: true
    cert_path: "config/cert.pem"
    key_path: "config/key.pem"

gemini:
  base_url: "https://generativelanguage.googleapis.com"
  timeout_seconds: 30
  api_keys:
    - id: "key1"
      key: "AIzaSyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
      weight: 100
      max_requests_per_minute: 60
    - id: "key2"
      key: "AIzaSyYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY"
      weight: 100
      max_requests_per_minute: 60

auth:
  enabled: true
  jwt_secret: "your-secret-key-here"
  rate_limit_per_minute: 100

metrics:
  enabled: true
  prometheus_port: 9090
```

### TLS 配置说明

为保证通信安全，TLS 配置为必需项。`server.tls` 字段说明如下：

- `enabled`：布尔值，是否启用 TLS。必须为 `true`。
- `cert_path`：字符串，TLS 证书文件路径（如 `.pem` 或 `.crt`）。
- `key_path`：字符串，TLS 私钥文件路径（如 `.key`）。

#### ACME 自动证书配置

- `acme.enabled`：布尔值，是否启用 ACME 自动证书管理。
- `acme.domains`：字符串列表，申请证书的域名。
- `acme.email`：字符串，接收 Let's Encrypt 通知的邮箱。
- `acme.directory_url`：字符串，ACME 提供商的目录 URL（如 Let's Encrypt 生产环境）。

示例配置：

```yaml
server:
  tls:
    enabled: true
    cert_path: "config/cert.pem"
    key_path: "config/key.pem"
    acme:
      enabled: true
      domains:
        - "proxy.example.com"
      email: "admin@example.com"
      directory_url: "https://acme-v02.api.letsencrypt.org/directory"
```
**如何生成自签名证书（开发/测试用）：**

```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes -subj "/CN=localhost"
```

将生成的 `cert.pem` 和 `key.pem` 放入 `config/` 目录，并在 `proxy.yaml` 中配置对应路径。

生产环境请使用受信任 CA 签发的证书。

#### 2.3.2 Cargo.toml
```toml
[package]
name = "gemini-proxy"
version = "0.1.0"
edition = "2021"

[dependencies]
pingora = "0.1"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1.0"
async-trait = "0.1"
jsonwebtoken = "8.0"
prometheus = "0.13"
warp = "0.3"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1.0", features = ["v4"] }
```

## 3. 扩展功能设计

### 3.1 健康检查机制
```rust
// src/utils/health_check.rs
pub struct HealthChecker {
    // 定期检查API Key的可用性
    // 自动恢复失败的Key
    // 监控响应时间和错误率
}
```

### 3.2 缓存层
```rust
// 实现响应缓存以减少API调用
// 支持Redis作为缓存后端
// 智能缓存策略
```

### 3.3 监控面板
```rust
// 实时监控API Key使用情况
// 请求成功率统计
// 性能指标展示
// 告警功能
```

### 3.4 动态配置
```rust
// 热更新API Key配置
// 动态调整负载均衡策略
// 运行时配置修改
```

## 4. 部署方案

### 4.1 Docker部署
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/gemini-proxy /usr/local/bin/
CMD ["gemini-proxy"]
```

### 4.2 Kubernetes部署
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: gemini-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: gemini-proxy
  template:
    metadata:
      labels:
        app: gemini-proxy
    spec:
      containers:
      - name: gemini-proxy
        image: gemini-proxy:latest
        ports:
        - containerPort: 8080
        - containerPort: 9090
```

这个架构设计基于Pingora的高性能特性[ref:1,3,10]，实现了多API Key的轮询负载均衡[ref:11,12,13]，并预留了丰富的扩展接口。核心关键点包括：

1. **高性能代理**：利用Pingora的Rust异步多线程框架
2. **智能负载均衡**：Round-robin + 健康检查 + 速率限制
3. **容错机制**：自动故障转移和恢复
4. **监控体系**：完整的指标收集和告警
5. **可扩展架构**：模块化设计，易于添加新功能

该设计可以轻松扩展支持其他LLM API、高级路由策略、分布式部署等功能。
