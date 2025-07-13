# Gemini API 代理 - 快速入门指南

本指南帮助您快速部署和使用企业级 Gemini API 代理服务。

## 🚀 快速部署

### 1. 准备环境

```bash
# 克隆项目
git clone <repository-url>
cd gem-proxy

# 检查 Rust 环境
rustc --version  # 需要 1.70+
```

### 2. 配置服务

```bash
# 复制配置模板
cp config/proxy.yaml.example config/proxy.yaml

# 编辑配置文件
vim config/proxy.yaml
```

**必须配置的关键参数：**

```yaml
gemini:
  api_keys:
    - id: "primary"
      key: "your-real-gemini-api-key"  # ⚠️ 替换为真实密钥
      weight: 100

auth:
  jwt_secret: "your-32-character-secret-key-here"  # ⚠️ 至少32字符
  admin_password: "your-admin-password"  # ⚠️ 至少12字符
```

### 3. 启动服务

```bash
# 开发环境
RUST_LOG=info cargo run

# 生产环境
cargo build --release
RUST_LOG=warn ./target/release/gemini-proxy
```

### 4. 验证部署

```bash
# 健康检查
curl http://localhost:9090/health

# 获取配置（需要管理员令牌）
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:9090/api/config
```

## 🔑 快速认证

### 获取管理员令牌

```bash
curl -X POST http://localhost:9090/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "your-admin-password"
  }'
```

### 使用令牌调用 API

```bash
curl -X POST http://localhost:8080/v1/models/gemini-1.5-pro:generateContent \
  -H "Authorization: Bearer <your-jwt-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [
      {
        "parts": [
          {
            "text": "你好，请介绍一下自己"
          }
        ]
      }
    ]
  }'
```

## 📊 常用管理操作

### 查看性能指标

```bash
# 实时性能数据
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:9090/api/metrics/performance

# Prometheus 指标
curl http://localhost:9090/metrics
```

### 权重预设管理

```bash
# 保存当前权重为预设
curl -X POST http://localhost:9090/api/persistence/weight-presets \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "生产环境配置",
    "description": "优化的生产权重分配",
    "weights": {
      "gemini-1.5-pro": 200,
      "gemini-1.5-flash": 300
    },
    "tags": ["production"]
  }'

# 应用权重预设
curl -X POST http://localhost:9090/api/persistence/weight-presets/{preset-id}/apply \
  -H "Authorization: Bearer <admin-token>"
```

### 审计日志查询

```bash
# 查看最近的 API 调用日志
curl -H "Authorization: Bearer <admin-token>" \
  "http://localhost:9090/api/audit/logs?event_type=api_call&limit=20"

# 查看配置变更历史
curl -H "Authorization: Bearer <admin-token>" \
  "http://localhost:9090/api/audit/config-changes"
```

## 🐍 Python 快速示例

```python
import requests

# 配置
PROXY_URL = "http://localhost:8080"
ADMIN_URL = "http://localhost:9090"

# 获取管理员令牌
auth_response = requests.post(f"{ADMIN_URL}/api/auth/login", json={
    "username": "admin",
    "password": "your-admin-password"
})
admin_token = auth_response.json()["tokens"]["access_token"]

# 调用 Gemini API
api_response = requests.post(
    f"{PROXY_URL}/v1/models/gemini-1.5-pro:generateContent",
    headers={
        "Authorization": f"Bearer {admin_token}",
        "Content-Type": "application/json"
    },
    json={
        "contents": [
            {
                "parts": [
                    {
                        "text": "用 Python 写一个 Hello World 程序"
                    }
                ]
            }
        ]
    }
)

print("AI 响应:", api_response.json()["candidates"][0]["content"]["parts"][0]["text"])

# 查看性能指标
metrics_response = requests.get(
    f"{ADMIN_URL}/api/metrics/performance",
    headers={"Authorization": f"Bearer {admin_token}"}
)
metrics = metrics_response.json()["metrics"]["current"]
print(f"当前 QPS: {metrics['qps']}")
print(f"成功率: {metrics['success_rate']:.1%}")
```

## 🔧 常见配置场景

### 高可用性配置

```yaml
gemini:
  api_keys:
    - id: "primary"
      key: "${GEMINI_KEY_1}"
      weight: 100
      max_requests_per_minute: 100
    - id: "secondary"
      key: "${GEMINI_KEY_2}"
      weight: 80
      max_requests_per_minute: 80
    - id: "backup"
      key: "${GEMINI_KEY_3}"
      weight: 50
      max_requests_per_minute: 50

auth:
  rate_limit_per_minute: 200  # 提高限制
  max_login_attempts: 5       # 适度的安全设置
```

### 开发环境配置

```yaml
server:
  host: "127.0.0.1"  # 仅本地访问
  tls:
    enabled: false   # 开发环境可以不用 TLS

auth:
  token_expiry_hours: 24      # 较长的令牌有效期
  session_timeout_minutes: 120 # 较长的会话时间
  rate_limit_per_minute: 1000  # 开发时不限制
```

### 生产环境安全配置

```yaml
server:
  host: "0.0.0.0"
  port: 443
  tls:
    enabled: true
    acme:
      enabled: true
      domains: ["your-domain.com"]

auth:
  jwt_secret: "${JWT_SECRET}"      # 从环境变量读取
  admin_password: "${ADMIN_PASSWORD}"
  token_expiry_hours: 8           # 较短的令牌有效期
  session_timeout_minutes: 30     # 较短的会话时间
  max_login_attempts: 3           # 严格的安全设置
  lockout_duration_minutes: 30
```

## 🛠️ 故障排除

### 常见启动问题

```bash
# 检查配置有效性
cargo run -- --check-config

# 查看详细启动日志
RUST_LOG=debug cargo run

# 检查端口占用
lsof -i :8080
lsof -i :9090
```

### 配置验证错误

```bash
# 生成安全的 JWT 密钥
openssl rand -base64 48

# 生成管理员密码
openssl rand -base64 16

# 验证 API 密钥
curl -H "Authorization: Bearer your-api-key" \
  "https://generativelanguage.googleapis.com/v1/models"
```

### 性能问题诊断

```bash
# 查看当前连接数
curl -H "Authorization: Bearer <token>" \
  http://localhost:9090/api/metrics/performance | jq '.metrics.current.active_connections'

# 查看错误统计
curl -H "Authorization: Bearer <token>" \
  http://localhost:9090/api/metrics/errors

# 查看 API 密钥健康状态
curl http://localhost:9090/health | jq '.api_keys'
```

## 📈 监控设置

### Prometheus 配置

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'gemini-proxy'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

### Grafana 仪表板

导入预定义的 Grafana 仪表板来监控：
- 请求量和响应时间
- API 密钥使用情况
- 错误率和成功率
- 系统资源使用

### 告警规则

```yaml
# prometheus-alerts.yml
groups:
  - name: gemini-proxy
    rules:
      - alert: HighErrorRate
        expr: rate(gemini_proxy_requests_total{status="error"}[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Gemini 代理错误率过高"
          
      - alert: AllAPIKeysDown
        expr: sum(gemini_proxy_api_key_health) == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "所有 API 密钥都不可用"
```

## 🔗 相关文档

- [完整 API 参考](./API_Reference.md) - 详细的 API 文档
- [CLAUDE.md](../CLAUDE.md) - 开发者指南
- [README.md](../README.md) - 项目概览
- [配置示例](../config/) - 配置文件模板

---

通过本快速入门指南，您应该能够在几分钟内部署并开始使用企业级 Gemini API 代理服务。如需更高级的配置和功能，请参考完整的 API 参考文档。