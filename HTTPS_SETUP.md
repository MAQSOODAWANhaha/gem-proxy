# HTTPS 配置说明

## 概述

Gemini Proxy 现在默认启用 HTTPS，为主服务和管理接口都提供安全连接。

## 端口配置

| 服务           | 端口  | 协议  | 用途               |
|----------------|-------|-------|--------------------|
| 主代理服务     | 8443  | HTTPS | Gemini API 代理    |
| 管理界面       | 9443  | HTTPS | 监控和管理接口     |
| Prometheus     | 9091  | HTTP  | 指标收集           |
| Grafana        | 3000  | HTTP  | 可视化仪表板       |

## 证书管理

### 自签证书（默认）

系统会自动生成以下证书：

```
certs/
├── cert.pem        # 主服务证书
├── key.pem         # 主服务私钥
├── api-cert.pem    # API服务证书
└── api-key.pem     # API服务私钥
```

### 支持的域名

自签证书包含以下 SAN（Subject Alternative Names）：

- `localhost`
- `gemini-proxy`
- `gemini-proxy.local`
- `proxy.local`
- `127.0.0.1`
- `::1`

## 快速启动

### 1. 启动服务

```bash
./scripts/quickstart.sh
```

### 2. 验证配置

```bash
./scripts/verify-https.sh
```

### 3. 测试连接

```bash
# 主服务健康检查
curl -k https://localhost:8443/health

# 管理界面健康检查
curl -k https://localhost:9443/health

# 查看监控指标
curl -k https://localhost:9443/metrics
```

## 配置文件

### config/proxy.yaml

```yaml
server:
  host: "0.0.0.0"
  port: 8443              # HTTPS 端口
  tls:
    enabled: true         # 启用 TLS
    cert_path: "certs/cert.pem"
    key_path: "certs/key.pem"

metrics:
  enabled: true
  prometheus_port: 9443   # HTTPS API 端口
  tls:
    enabled: true         # API 服务也启用 TLS
    cert_path: "certs/api-cert.pem"
    key_path: "certs/api-key.pem"
```

## 浏览器访问

由于使用自签证书，浏览器会显示安全警告：

1. **Chrome/Edge**: 点击"Advanced" → "Proceed to localhost (unsafe)"
2. **Firefox**: 点击"Advanced" → "Accept the Risk and Continue"
3. **Safari**: 点击"Show Details" → "visit this website"

## 生产环境

### 使用有效证书

1. **Let's Encrypt（推荐）**:

```yaml
server:
  tls:
    acme:
      enabled: true
      domains:
        - "api.yourdomain.com"
      email: "admin@yourdomain.com"
      directory_url: "https://acme-v02.api.letsencrypt.org/directory"
```

2. **自定义证书**:

```yaml
server:
  tls:
    enabled: true
    cert_path: "/path/to/your/cert.pem"
    key_path: "/path/to/your/key.pem"
```

## 故障排除

### 证书问题

```bash
# 检查证书有效性
openssl x509 -in certs/cert.pem -text -noout

# 验证私钥匹配
openssl rsa -in certs/key.pem -check

# 重新生成证书
rm certs/*.pem
./scripts/quickstart.sh
```

### 连接问题

```bash
# 检查端口占用
netstat -tlnp | grep -E '8443|9443'

# 查看服务日志
docker-compose logs -f gemini-proxy

# 检查防火墙
sudo ufw status
```

### 常见错误

1. **SSL_ERROR_BAD_CERT_DOMAIN**: 使用 `-k` 参数或添加证书到信任列表
2. **Connection refused**: 检查服务是否启动，端口是否正确
3. **Certificate expired**: 重新生成证书（自签证书有效期1年）

## 监控和日志

### HTTPS 连接监控

```bash
# 查看 TLS 连接统计
ss -tlnp | grep -E '8443|9443'

# 检查证书到期时间
openssl x509 -in certs/cert.pem -noout -dates
```

### 性能监控

- Prometheus: http://localhost:9091
- Grafana: http://localhost:3000 (admin/admin)

## 安全建议

1. **定期更新证书**（自签证书1年有效期）
2. **生产环境使用有效证书**
3. **启用 HSTS**（生产环境）
4. **定期轮换 JWT 密钥**
5. **监控 TLS 连接日志**

## 更多信息

- [项目文档](./README.md)
- [API 文档](./docs/API.md)
- [部署指南](./docs/DEPLOYMENT.md)