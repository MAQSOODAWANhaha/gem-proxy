#!/bin/bash
# HTTPS 配置验证脚本

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

# 验证证书文件
verify_certificates() {
    log_step "验证 SSL 证书文件..."
    
    if [ -f "certs/cert.pem" ] && [ -f "certs/key.pem" ]; then
        log_info "✅ 主服务证书文件存在"
        
        # 验证证书有效性
        if openssl x509 -in certs/cert.pem -text -noout > /dev/null 2>&1; then
            log_info "✅ 主服务证书格式正确"
            
            # 显示证书信息
            log_info "📄 主服务证书信息:"
            openssl x509 -in certs/cert.pem -text -noout | grep -A 5 "Subject:"
            openssl x509 -in certs/cert.pem -text -noout | grep -A 10 "Subject Alternative Name"
        else
            log_error "❌ 主服务证书格式错误"
        fi
    else
        log_warn "⚠️ 主服务证书文件不存在"
    fi
    
    if [ -f "certs/api-cert.pem" ] && [ -f "certs/api-key.pem" ]; then
        log_info "✅ API 服务证书文件存在"
        
        # 验证证书有效性
        if openssl x509 -in certs/api-cert.pem -text -noout > /dev/null 2>&1; then
            log_info "✅ API 服务证书格式正确"
            
            # 显示证书信息
            log_info "📄 API 服务证书信息:"
            openssl x509 -in certs/api-cert.pem -text -noout | grep -A 5 "Subject:"
            openssl x509 -in certs/api-cert.pem -text -noout | grep -A 10 "Subject Alternative Name"
        else
            log_error "❌ API 服务证书格式错误"
        fi
    else
        log_warn "⚠️ API 服务证书文件不存在"
    fi
}

# 验证配置文件
verify_config() {
    log_step "验证配置文件..."
    
    if [ -f "config/proxy.yaml" ]; then
        log_info "✅ 配置文件存在"
        
        # 检查 TLS 配置
        if grep -q "enabled: true" config/proxy.yaml; then
            log_info "✅ TLS 已启用"
        else
            log_warn "⚠️ TLS 未启用"
        fi
        
        # 检查端口配置
        main_port=$(grep "port:" config/proxy.yaml | head -1 | awk '{print $2}')
        api_port=$(grep "prometheus_port:" config/proxy.yaml | awk '{print $2}')
        
        log_info "🌐 主服务端口: $main_port"
        log_info "🔧 API 服务端口: $api_port"
        
        if [ "$main_port" = "8443" ] && [ "$api_port" = "9443" ]; then
            log_info "✅ 端口配置正确"
        else
            log_warn "⚠️ 端口配置可能需要检查"
        fi
    else
        log_error "❌ 配置文件不存在"
    fi
}

# 验证 Docker 配置
verify_docker() {
    log_step "验证 Docker 配置..."
    
    if [ -f "docker-compose.yml" ]; then
        log_info "✅ Docker Compose 文件存在"
        
        # 检查端口映射
        if grep -q "8443:8443" docker-compose.yml && grep -q "9443:9443" docker-compose.yml; then
            log_info "✅ Docker 端口映射正确"
        else
            log_warn "⚠️ Docker 端口映射需要检查"
        fi
    fi
    
    if [ -f "Dockerfile" ]; then
        log_info "✅ Dockerfile 存在"
        
        # 检查暴露端口
        if grep -q "EXPOSE 8443 9443" Dockerfile; then
            log_info "✅ Dockerfile 端口暴露正确"
        else
            log_warn "⚠️ Dockerfile 端口暴露需要检查"
        fi
    fi
}

# 测试 HTTPS 连接
test_https() {
    log_step "测试 HTTPS 连接..."
    
    # 检查服务是否运行
    if docker-compose ps | grep -q "Up"; then
        log_info "🔍 检测到服务运行，测试连接..."
        
        # 测试主服务
        if curl -f -k -m 5 https://localhost:8443/health > /dev/null 2>&1; then
            log_info "✅ 主服务 HTTPS 连接成功"
        else
            log_warn "⚠️ 主服务 HTTPS 连接失败（可能服务未启动）"
        fi
        
        # 测试 API 服务
        if curl -f -k -m 5 https://localhost:9443/health > /dev/null 2>&1; then
            log_info "✅ API 服务 HTTPS 连接成功"
        else
            log_warn "⚠️ API 服务 HTTPS 连接失败（可能服务未启动）"
        fi
    else
        log_info "ℹ️ 服务未运行，跳过连接测试"
        log_info "   使用 './scripts/quickstart.sh' 启动服务后再次测试"
    fi
}

# 显示使用指南
show_usage() {
    log_step "📖 HTTPS 使用指南"
    echo ""
    echo "🔧 启动服务:"
    echo "  ./scripts/quickstart.sh"
    echo ""
    echo "🌐 访问地址:"
    echo "  主服务:     https://localhost:8443"
    echo "  管理界面:   https://localhost:9443"
    echo "  Prometheus: http://localhost:9091"
    echo "  Grafana:    http://localhost:3000"
    echo ""
    echo "🧪 测试命令:"
    echo "  curl -k https://localhost:8443/health"
    echo "  curl -k https://localhost:9443/health"
    echo ""
    echo "⚠️ 注意:"
    echo "  • 使用 -k 参数忽略自签证书警告"
    echo "  • 浏览器访问时可能需要手动接受证书"
    echo "  • 生产环境建议使用有效的 SSL 证书"
}

# 主函数
main() {
    echo "🔒 HTTPS 配置验证工具"
    echo "======================="
    echo ""
    
    # 检查是否在项目根目录
    if [ ! -f "config/proxy.yaml" ]; then
        log_error "请在项目根目录运行此脚本"
        exit 1
    fi
    
    verify_certificates
    verify_config
    verify_docker
    test_https
    show_usage
    
    echo ""
    log_info "🎉 HTTPS 配置验证完成"
}

# 解析命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            echo "用法: $0 [选项]"
            echo "选项:"
            echo "  --help, -h    显示帮助信息"
            echo ""
            echo "此脚本将验证:"
            echo "  1. SSL 证书文件"
            echo "  2. 配置文件"
            echo "  3. Docker 配置"
            echo "  4. HTTPS 连接（如果服务运行）"
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