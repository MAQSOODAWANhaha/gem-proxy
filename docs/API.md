# API 文档

本文档描述了 Gemini API 代理服务的完整 API 接口。

## 目录

- [代理 API](#代理-api)
- [管理 API](#管理-api)
- [监控 API](#监控-api)
- [认证](#认证)
- [错误处理](#错误处理)

## 代理 API

### 基础端点

代理服务运行在配置的端口上（默认 8080），所有对 Gemini API 的请求都会通过此端点转发。

#### 请求格式

```http
POST /v1/models/{model}:generateContent HTTP/1.1
Host: localhost:8080
Authorization: Bearer <jwt-token>
Content-Type: application/json

{
  "contents": [
    {
      "parts": [
        {
          "text": "Hello, world!"
        }
      ]
    }
  ]
}
```

#### 响应格式

成功的响应会直接转发 Gemini API 的响应：

```json
{
  "candidates": [
    {
      "content": {
        "parts": [
          {
            "text": "Hello! How can I help you today?"
          }
        ]
      },
      "finishReason": "STOP"
    }
  ]
}
```

### 支持的模型

代理支持所有 Gemini API 模型：

- `gemini-1.5-pro`
- `gemini-1.5-flash`
- `gemini-1.0-pro`
- `gemini-1.0-pro-vision`

### 请求头

| 头部 | 必需 | 描述 |
|------|------|------|
| `Authorization` | 是 | Bearer JWT 令牌 |
| `Content-Type` | 是 | application/json |
| `X-Request-ID` | 否 | 用于追踪的请求 ID |

## 管理 API

管理 API 运行在单独的端口上（默认 9090），用于配置和监控代理服务。

### 配置管理

#### 获取配置

```http
GET /api/config HTTP/1.1
Host: localhost:9090
```

**响应：**

```json
{
  "success": true,
  "data": {
    "server": {
      "host": "0.0.0.0",
      "port": 8080,
      "workers": 4,
      "max_connections": 1000,
      "tls": {
        "enabled": false
      }
    },
    "gemini": {
      "api_keys": [
        {
          "id": "key1",
          "key": "***",
          "weight": 1,
          "max_requests_per_minute": 100
        }
      ],
      "base_url": "https://generativelanguage.googleapis.com",
      "timeout_seconds": 30
    },
    "auth": {
      "enabled": true,
      "rate_limit_per_minute": 60
    },
    "metrics": {
      "enabled": true,
      "prometheus_port": 9090
    }
  }
}
```

#### 更新配置

```http
PUT /api/config HTTP/1.1
Host: localhost:9090
Content-Type: application/json

{
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "workers": 4,
    "max_connections": 1000,
    "tls": {
      "enabled": false
    }
  },
  "gemini": {
    "api_keys": [
      {
        "id": "key1",
        "key": "your-api-key",
        "weight": 1,
        "max_requests_per_minute": 100
      }
    ],
    "base_url": "https://generativelanguage.googleapis.com",
    "timeout_seconds": 30
  },
  "auth": {
    "enabled": true,
    "jwt_secret": "your-jwt-secret",
    "rate_limit_per_minute": 60
  },
  "metrics": {
    "enabled": true,
    "prometheus_port": 9090
  }
}
```

**响应：**

```json
{
  "success": true,
  "message": "配置更新成功"
}
```

## 监控 API

### 健康检查

#### 服务健康状态

```http
GET /health HTTP/1.1
Host: localhost:9090
```

**响应：**

```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "uptime_seconds": 3600,
  "version": "1.0.0",
  "services": {
    "proxy": "healthy",
    "config": "healthy",
    "metrics": "healthy"
  },
  "api_keys": [
    {
      "id": "key1",
      "is_healthy": true,
      "failure_count": 0,
      "last_used": "2024-01-15T10:29:30Z"
    }
  ]
}
```

### 性能指标

#### 获取性能统计

```http
GET /performance HTTP/1.1
Host: localhost:9090
```

**响应：**

```json
{
  "qps": 125.5,
  "success_rate": 0.995,
  "avg_response_time_ms": 250,
  "p99_response_time_ms": 800,
  "active_connections": 45,
  "connection_usage": 0.045,
  "memory_usage_bytes": 104857600,
  "uptime_seconds": 3600
}
```

### 错误统计

#### 获取错误信息

```http
GET /errors HTTP/1.1
Host: localhost:9090
```

**响应：**

```json
{
  "total_errors": 25,
  "recent_errors": 5,
  "by_severity": {
    "Low": 15,
    "Medium": 8,
    "High": 2,
    "Critical": 0
  },
  "by_component": {
    "proxy": 12,
    "auth": 8,
    "config": 3,
    "network": 2
  }
}
```

### Prometheus 指标

#### 获取 Prometheus 格式指标

```http
GET /metrics HTTP/1.1
Host: localhost:9090
```

**响应：**

```
# HELP gemini_proxy_requests_total Total number of requests processed
# TYPE gemini_proxy_requests_total counter
gemini_proxy_requests_total{status="success"} 1234
gemini_proxy_requests_total{status="error"} 56

# HELP gemini_proxy_request_duration_seconds Request duration in seconds
# TYPE gemini_proxy_request_duration_seconds histogram
gemini_proxy_request_duration_seconds_bucket{le="0.1"} 100
gemini_proxy_request_duration_seconds_bucket{le="0.5"} 800
gemini_proxy_request_duration_seconds_bucket{le="1.0"} 1200
gemini_proxy_request_duration_seconds_bucket{le="+Inf"} 1290
gemini_proxy_request_duration_seconds_sum 500.5
gemini_proxy_request_duration_seconds_count 1290

# HELP gemini_proxy_api_key_requests_total Total requests per API key
# TYPE gemini_proxy_api_key_requests_total counter
gemini_proxy_api_key_requests_total{key_id="key1"} 800
gemini_proxy_api_key_requests_total{key_id="key2"} 490

# HELP gemini_proxy_api_key_failures_total Total failures per API key
# TYPE gemini_proxy_api_key_failures_total counter
gemini_proxy_api_key_failures_total{key_id="key1"} 5
gemini_proxy_api_key_failures_total{key_id="key2"} 3
```

## 认证

### JWT 令牌

所有对代理 API 的请求都需要有效的 JWT 令牌。令牌应在 `Authorization` 头中以 Bearer 格式提供。

#### 令牌格式

```
Authorization: Bearer <jwt-token>
```

#### JWT 负载示例

```json
{
  "sub": "user123",
  "iss": "gemini-proxy",
  "aud": "gemini-api",
  "exp": 1705312200,
  "iat": 1705308600,
  "client_id": "app-client-1",
  "permissions": ["api:read", "api:write"]
}
```

### 速率限制

代理实施基于客户端的速率限制：

- **默认限制**: 每分钟 60 个请求
- **限制头部**: 响应中包含速率限制信息

#### 速率限制头部

```http
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1705308660
```

## 错误处理

### 错误响应格式

所有 API 错误都遵循一致的格式：

```json
{
  "success": false,
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "速率限制超出",
    "details": {
      "limit": 60,
      "window": "1 minute",
      "retry_after": 30
    },
    "timestamp": "2024-01-15T10:30:00Z",
    "request_id": "req-123456"
  }
}
```

### 常见错误码

| 错误码 | HTTP 状态码 | 描述 |
|--------|-------------|------|
| `UNAUTHORIZED` | 401 | 无效或缺失的认证令牌 |
| `FORBIDDEN` | 403 | 权限不足 |
| `RATE_LIMIT_EXCEEDED` | 429 | 超出速率限制 |
| `INVALID_REQUEST` | 400 | 请求格式错误 |
| `API_KEY_ERROR` | 502 | API 密钥错误或不可用 |
| `UPSTREAM_ERROR` | 502 | 上游服务错误 |
| `TIMEOUT` | 504 | 请求超时 |
| `INTERNAL_ERROR` | 500 | 内部服务器错误 |

### 错误示例

#### 认证错误

```json
{
  "success": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "无效的 JWT 令牌",
    "timestamp": "2024-01-15T10:30:00Z",
    "request_id": "req-123456"
  }
}
```

#### 速率限制错误

```json
{
  "success": false,
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "超出每分钟 60 次请求限制",
    "details": {
      "limit": 60,
      "window": "1 minute",
      "retry_after": 30
    },
    "timestamp": "2024-01-15T10:30:00Z",
    "request_id": "req-123456"
  }
}
```

#### API 密钥错误

```json
{
  "success": false,
  "error": {
    "code": "API_KEY_ERROR",
    "message": "所有 API 密钥都不可用",
    "details": {
      "total_keys": 3,
      "healthy_keys": 0,
      "last_check": "2024-01-15T10:29:45Z"
    },
    "timestamp": "2024-01-15T10:30:00Z",
    "request_id": "req-123456"
  }
}
```

## SDK 和客户端库

### cURL 示例

```bash
# 发送请求到 Gemini API
curl -X POST http://localhost:8080/v1/models/gemini-1.5-pro:generateContent \
  -H "Authorization: Bearer your-jwt-token" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [
      {
        "parts": [
          {
            "text": "什么是人工智能？"
          }
        ]
      }
    ]
  }'

# 检查服务健康状态
curl http://localhost:9090/health

# 获取性能指标
curl http://localhost:9090/performance

# 获取 Prometheus 指标
curl http://localhost:9090/metrics
```

### Python 示例

```python
import requests
import json

# 配置
PROXY_URL = "http://localhost:8080"
ADMIN_URL = "http://localhost:9090"
JWT_TOKEN = "your-jwt-token"

# 发送请求到 Gemini API
def call_gemini_api(prompt):
    headers = {
        "Authorization": f"Bearer {JWT_TOKEN}",
        "Content-Type": "application/json"
    }
    
    data = {
        "contents": [
            {
                "parts": [
                    {
                        "text": prompt
                    }
                ]
            }
        ]
    }
    
    response = requests.post(
        f"{PROXY_URL}/v1/models/gemini-1.5-pro:generateContent",
        headers=headers,
        json=data
    )
    
    return response.json()

# 检查服务健康状态
def check_health():
    response = requests.get(f"{ADMIN_URL}/health")
    return response.json()

# 获取性能指标
def get_performance_metrics():
    response = requests.get(f"{ADMIN_URL}/performance")
    return response.json()

# 使用示例
if __name__ == "__main__":
    # 调用 API
    result = call_gemini_api("什么是人工智能？")
    print(json.dumps(result, indent=2, ensure_ascii=False))
    
    # 检查健康状态
    health = check_health()
    print(f"服务状态: {health['status']}")
    
    # 获取性能指标
    metrics = get_performance_metrics()
    print(f"QPS: {metrics['qps']}")
    print(f"成功率: {metrics['success_rate']:.1%}")
```

### JavaScript 示例

```javascript
// 配置
const PROXY_URL = "http://localhost:8080";
const ADMIN_URL = "http://localhost:9090";
const JWT_TOKEN = "your-jwt-token";

// 发送请求到 Gemini API
async function callGeminiAPI(prompt) {
  const response = await fetch(`${PROXY_URL}/v1/models/gemini-1.5-pro:generateContent`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${JWT_TOKEN}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      contents: [
        {
          parts: [
            {
              text: prompt
            }
          ]
        }
      ]
    })
  });

  return await response.json();
}

// 检查服务健康状态
async function checkHealth() {
  const response = await fetch(`${ADMIN_URL}/health`);
  return await response.json();
}

// 获取性能指标
async function getPerformanceMetrics() {
  const response = await fetch(`${ADMIN_URL}/performance`);
  return await response.json();
}

// 使用示例
async function example() {
  try {
    // 调用 API
    const result = await callGeminiAPI("什么是人工智能？");
    console.log('API 响应:', result);
    
    // 检查健康状态
    const health = await checkHealth();
    console.log('服务状态:', health.status);
    
    // 获取性能指标
    const metrics = await getPerformanceMetrics();
    console.log(`QPS: ${metrics.qps}`);
    console.log(`成功率: ${(metrics.success_rate * 100).toFixed(1)}%`);
  } catch (error) {
    console.error('错误:', error);
  }
}

example();
```

## 最佳实践

### 1. 错误处理

- 始终检查响应状态码
- 实施指数退避重试策略
- 处理速率限制和临时错误

### 2. 性能优化

- 使用连接池复用 HTTP 连接
- 实施客户端缓存
- 监控响应时间和错误率

### 3. 安全性

- 安全存储 JWT 令牌
- 定期轮换 API 密钥
- 使用 HTTPS 传输敏感数据

### 4. 监控

- 定期检查健康状态
- 监控性能指标
- 设置告警阈值

这份 API 文档提供了完整的接口说明，帮助开发者快速集成和使用 Gemini API 代理服务。