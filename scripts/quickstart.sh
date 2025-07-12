#!/bin/bash
# Gemini Proxy 快速启动脚本
# 自动生成自签证书并启动 HTTPS 服务

set -e

# 颜色输出
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# 检查先决条件
check_prerequisites() {
    log_step "检查系统先决条件..."
    
    # 检查 Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker 未安装，请先安装 Docker"
        exit 1
    fi
    
    # 检查 Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose 未安装，请先安装 Docker Compose"
        exit 1
    fi
    
    # 检查 Rust (可选，仅本地开发需要)
    if command -v cargo &> /dev/null; then
        log_info "✅ Rust 环境已安装"
    else
        log_warn "⚠️ Rust 未安装，仅支持 Docker 部署"
    fi
    
    log_info "✅ 先决条件检查完成"
}

# 创建必要的目录
create_directories() {
    log_step "创建必要的目录..."
    
    mkdir -p certs logs
    chmod 755 certs logs
    
    log_info "✅ 目录创建完成"
}

# 检查配置文件
check_config() {
    log_step "检查配置文件..."
    
    if [ ! -f "config/proxy.yaml" ]; then
        if [ -f "config/proxy.yaml.example" ]; then
            log_warn "配置文件不存在，使用示例配置"
            cp config/proxy.yaml.example config/proxy.yaml
            log_info "✅ 已创建配置文件 config/proxy.yaml"
            log_warn "⚠️ 请编辑 config/proxy.yaml 配置您的 Gemini API 密钥"
        else
            log_error "示例配置文件不存在"
            exit 1
        fi
    else
        log_info "✅ 配置文件已存在"
    fi
    
    # 检查 API 密钥是否已配置
    if grep -q "your-gemini-api-key" config/proxy.yaml; then
        log_warn "⚠️ 检测到默认 API 密钥，请更新为实际的 Gemini API 密钥"
        log_warn "   编辑 config/proxy.yaml 文件中的 gemini.api_keys 部分"
    fi
}

# 生成自签证书
generate_certificates() {
    log_step "检查 SSL 证书..."
    
    if [ -f "certs/cert.pem" ] && [ -f "certs/key.pem" ]; then
        log_info "✅ SSL 证书已存在"
    else
        log_info "生成自签 SSL 证书..."
        
        # 使用 OpenSSL 生成自签证书
        if command -v openssl &> /dev/null; then
            # 创建证书配置
            cat > certs/cert.conf << EOF
[req]
default_bits = 2048
prompt = no
distinguished_name = req_distinguished_name
req_extensions = v3_req

[req_distinguished_name]
C = CN
ST = Beijing
L = Beijing
O = Gemini Proxy
OU = IT Department
CN = localhost

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = gemini-proxy
DNS.3 = gemini-proxy.local
DNS.4 = proxy.local
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

            # 生成主服务私钥和证书
            openssl genrsa -out certs/key.pem 2048
            openssl req -new -x509 -key certs/key.pem -out certs/cert.pem -days 365 -config certs/cert.conf -extensions v3_req
            
            # 生成 API 服务器私钥和证书
            openssl genrsa -out certs/api-key.pem 2048
            openssl req -new -x509 -key certs/api-key.pem -out certs/api-cert.pem -days 365 -config certs/cert.conf -extensions v3_req
            
            # 设置权限
            chmod 600 certs/key.pem certs/api-key.pem
            chmod 644 certs/cert.pem certs/api-cert.pem
            
            # 清理临时文件
            rm certs/cert.conf
            
            log_info "✅ 自签 SSL 证书生成成功"
            log_info "📄 主服务证书: certs/cert.pem"
            log_info "🔑 主服务私钥: certs/key.pem"
            log_info "📄 API 服务证书: certs/api-cert.pem"
            log_info "🔑 API 服务私钥: certs/api-key.pem"
            log_info "🌐 有效域名: localhost, gemini-proxy, *.local"
        else
            log_warn "OpenSSL 未安装，将在服务启动时自动生成证书"
        fi
    fi
}

# 启动服务
start_services() {
    log_step "启动 Gemini Proxy 服务..."
    
    # 停止现有服务
    if docker-compose ps | grep -q "Up"; then
        log_info "停止现有服务..."
        docker-compose down
    fi
    
    # 构建并启动服务
    log_info "构建 Docker 镜像..."
    docker-compose build --no-cache
    
    log_info "启动服务..."
    docker-compose up -d
    
    log_info "✅ 服务启动完成"
}

# 健康检查
health_check() {
    log_step "等待服务启动..."
    
    sleep 10
    
    # 检查服务状态
    for i in {1..30}; do
        if curl -f -k https://localhost:8443/health > /dev/null 2>&1 || curl -f -k https://localhost:9443/health > /dev/null 2>&1; then
            log_info "✅ 服务健康检查通过"
            break
        fi
        if [ $i -eq 30 ]; then
            log_error "❌ 服务健康检查失败"
            log_error "查看日志: docker-compose logs gemini-proxy"
            exit 1
        fi
        echo -n "."
        sleep 2
    done
}

# 显示服务信息
show_service_info() {
    log_step "🎉 Gemini Proxy 启动成功！"
    echo ""
    echo "📋 服务访问地址："
    echo "  🌐 代理服务 (HTTPS): https://localhost:8443"
    echo "  🔧 管理界面 (HTTPS): https://localhost:9443"
    echo "  📊 Prometheus:       http://localhost:9091"
    echo "  📈 Grafana:          http://localhost:3000 (admin/admin)"
    echo ""
    echo "📝 快速测试："
    echo "  curl -k -H \"Authorization: Bearer your-jwt-token\" \\"
    echo "       -H \"Content-Type: application/json\" \\"
    echo "       -d '{\"contents\":[{\"parts\":[{\"text\":\"Hello\"}]}]}' \\"
    echo "       https://localhost:8443/v1/models/gemini-1.5-pro:generateContent"
    echo ""
    echo "🔧 常用命令："
    echo "  查看服务状态: docker-compose ps"
    echo "  查看日志:     docker-compose logs -f gemini-proxy"
    echo "  停止服务:     docker-compose down"
    echo "  重启服务:     docker-compose restart"
    echo ""
    echo "⚠️ 注意事项："
    echo "  • 当前使用自签证书，浏览器可能显示安全警告"
    echo "  • 主服务和管理界面都启用了 HTTPS"
    echo "  • 请在 config/proxy.yaml 中配置您的 Gemini API 密钥"
    echo "  • 生产环境建议使用有效的 SSL 证书"
    echo ""
    echo "📖 更多帮助: https://github.com/your-org/gem-proxy"
}

# 主函数
main() {
    echo "🚀 Gemini Proxy 快速启动脚本"
    echo "================================"
    
    # 检查是否在项目根目录
    if [ ! -f "Dockerfile" ] || [ ! -f "docker-compose.yml" ]; then
        log_error "请在项目根目录运行此脚本"
        exit 1
    fi
    
    check_prerequisites
    create_directories
    check_config
    generate_certificates
    start_services
    health_check
    show_service_info
}

# 解析命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            echo "用法: $0 [选项]"
            echo "选项:"
            echo "  --help, -h    显示帮助信息"
            echo ""
            echo "此脚本将自动："
            echo "  1. 检查系统先决条件"
            echo "  2. 创建必要的目录"
            echo "  3. 生成自签 SSL 证书"
            echo "  4. 启动所有服务"
            echo "  5. 进行健康检查"
            exit 0
            ;;
        *)
            log_error "未知参数: $1"
            exit 1
            ;;
    esac
done

# 运行主函数
main