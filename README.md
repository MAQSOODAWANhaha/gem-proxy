# Gemini API Proxy

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Security](https://img.shields.io/badge/security-enterprise-green.svg)](#security)

企业级 Rust Gemini API 代理服务，基于 Pingora 框架构建，提供高性能负载均衡、安全认证、审计日志和 TLS/ACME 支持。

## 🚀 核心特性

### 基础功能
- **🔄 智能负载均衡**: 多 API 密钥加权轮询和性能优化
- **🔐 JWT 认证**: Bearer 令牌验证和权限控制
- **🛡️ 速率限制**: 基于 IP 和用户的请求频率控制
- **🔒 TLS/ACME**: 自动 Let's Encrypt 证书管理

### 企业级特性
- **📊 统一错误处理**: 结构化错误日志、自动恢复、兼容迁移
- **🔍 安全配置验证**: 启动时安全检查、CWE 威胁分析、评分系统
- **📋 审计合规**: 完整的 API 调用、配置变更、安全事件追踪
- **💾 数据持久化**: 配置历史、权重预设、会话状态管理
- **📈 性能监控**: Prometheus 指标、性能分析、异常检测

### 管理功能
- **🔄 配置热重载**: 动态配置更新，无需重启服务
- **⚙️ 权重预设**: 预定义权重配置的快速切换
- **🔑 密钥轮换**: 自动密钥轮换和强度检查
- **🚑 故障恢复**: 熔断器、重试策略、自愈机制

## 📦 快速开始

### 安装

```bash
# 克隆项目
git clone https://github.com/your-org/gemini-proxy.git
cd gemini-proxy

# 构建项目
cargo build --release
```

### 配置

```bash
# 复制配置模板
cp config/proxy.yaml.secure config/proxy.yaml

# 编辑配置文件
vim config/proxy.yaml
```

### 最小配置示例

```yaml
server:
  host: "127.0.0.1"
  port: 8080
  tls:
    enabled: false  # 开发环境

gemini:
  api_keys:
    - id: "primary"
      key: "your-gemini-api-key"
      weight: 100

auth:
  enabled: true
  jwt_secret: "your-secure-jwt-secret-at-least-32-chars"
  admin_password: "your-secure-admin-password"

metrics:
  enabled: true
  prometheus_port: 9090
```

### 启动服务

```bash
# 启动代理服务（包含安全验证）
./target/release/gemini-proxy

# 开发模式（详细日志）
RUST_LOG=debug cargo run
```

## 🔧 API 端点

### 监控端点（无需认证）
- `GET /metrics` - Prometheus 指标
- `GET /health` - 健康检查
- `GET /performance` - 性能统计
- `GET /errors` - 错误统计

### 认证端点
```bash
# 管理员登录
curl -X POST http://localhost:9090/auth/login \
  -H "Content-Type: application/json" \
  -d '{"password": "your-admin-password"}'
```

### 管理端点（需要 JWT 认证）
```bash
# 获取配置
curl -H "Authorization: Bearer <token>" \
  http://localhost:9090/api/config

# 应用权重预设
curl -X POST -H "Authorization: Bearer <token>" \
  http://localhost:9090/api/weights/apply \
  -d '{"preset_name": "high_performance"}'
```

## 🔒 安全配置

### 启动时安全检查

服务启动时会自动进行安全配置验证：

- ✅ **JWT 密钥**: 最少32字符，包含大小写字母、数字、特殊字符
- ✅ **管理员密码**: 最少12字符，避免常见弱密码
- ✅ **API 密钥**: 避免示例密钥，确保有效性
- ✅ **TLS 配置**: 生产环境必须启用 TLS
- ✅ **网络安全**: 检查绑定地址和端口配置

### 安全最佳实践

1. **使用环境变量存储敏感信息**：
```bash
export JWT_SECRET="$(openssl rand -base64 64)"
export ADMIN_PASSWORD="your-very-secure-password"
export GEMINI_API_KEY="your-real-api-key"
```

2. **生产环境配置**：
```yaml
server:
  host: "0.0.0.0"
  port: 443
  tls:
    enabled: true
    acme:
      enabled: true
      domains: ["api.yourdomain.com"]
```

3. **启用审计日志**：
```yaml
# 系统会自动记录到 logs/audit.log
```

## 📊 监控和运维

### Prometheus 指标

```bash
# 查看指标
curl http://localhost:9090/metrics
```

关键指标：
- `http_requests_total` - 总请求数
- `gemini_api_calls_total` - Gemini API 调用数
- `auth_failures_total` - 认证失败数
- `api_key_usage` - API 密钥使用统计

### 健康检查

```bash
# 检查服务健康状态
curl http://localhost:9090/health

# 查看性能统计
curl http://localhost:9090/performance
```

### 审计日志

审计日志自动记录到 `logs/audit.log`：
```json
{
  "timestamp": "2024-01-01T00:00:00Z",
  "event_type": "ApiCall",
  "source_ip": "127.0.0.1",
  "action": "GET /v1/models",
  "result": "Success",
  "duration_ms": 150
}
```

## 🏗️ 开发指南

### 项目结构

```
src/
├── proxy/          # Pingora 代理核心
├── load_balancer/  # 统一负载均衡器
├── auth/           # JWT 认证系统
├── config/         # 配置管理和验证
├── error/          # 统一错误处理
├── security/       # 安全模块
├── persistence/    # 数据持久化
├── metrics/        # 监控指标
└── api/           # 管理 API
```

### 开发命令

```bash
# 检查代码
cargo clippy

# 运行测试
cargo test

# 格式化代码
cargo fmt

# 运行安全测试
cargo test security
```

## 📚 文档

- **[开发文档](CLAUDE.md)** - 完整的开发指南和架构说明
- **[配置参考](config/proxy.yaml.example)** - 详细的配置说明
- **[安全指南](config/proxy.yaml.secure)** - 安全配置最佳实践

## 🤝 贡献

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

该项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🔗 相关链接

- [Pingora Framework](https://github.com/cloudflare/pingora)
- [Google Gemini API](https://ai.google.dev/)
- [Prometheus Monitoring](https://prometheus.io/)

---

**企业级 Rust 代理服务，为生产环境而生 🚀**