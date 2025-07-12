#!/bin/bash
# Gemini Proxy 部署脚本

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查依赖
check_dependencies() {
    log_info "检查部署依赖..."
    
    if ! command -v docker &> /dev/null; then
        log_error "Docker 未安装，请先安装 Docker"
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose 未安装，请先安装 Docker Compose"
        exit 1
    fi
    
    log_info "依赖检查完成"
}

# 创建必要的目录
create_directories() {
    log_info "创建必要的目录..."
    
    mkdir -p logs certs
    chmod 755 logs certs
    
    log_info "目录创建完成"
}

# 检查配置文件
check_config() {
    log_info "检查配置文件..."
    
    if [ ! -f "config/proxy.yaml" ]; then
        log_error "配置文件 config/proxy.yaml 不存在"
        log_info "请复制 config/proxy.yaml.example 并修改为适合的配置"
        exit 1
    fi
    
    # 检查配置文件格式
    if ! docker run --rm -v "$(pwd)/config:/config" -w /config mikefarah/yq eval '.' proxy.yaml > /dev/null 2>&1; then
        log_error "配置文件格式错误"
        exit 1
    fi
    
    log_info "配置文件检查完成"
}

# 构建镜像
build_image() {
    log_info "构建 Docker 镜像..."
    
    docker build -t gemini-proxy:latest .
    
    if [ $? -eq 0 ]; then
        log_info "镜像构建成功"
    else
        log_error "镜像构建失败"
        exit 1
    fi
}

# 部署服务
deploy_services() {
    log_info "部署服务..."
    
    # 停止现有服务
    docker-compose down --remove-orphans
    
    # 启动新服务
    docker-compose up -d
    
    if [ $? -eq 0 ]; then
        log_info "服务部署成功"
    else
        log_error "服务部署失败"
        exit 1
    fi
}

# 健康检查
health_check() {
    log_info "进行健康检查..."
    
    # 等待服务启动
    sleep 10
    
    # 检查主服务
    for i in {1..30}; do
        if curl -f -k https://localhost:8443/health > /dev/null 2>&1 || curl -f http://localhost:9090/health > /dev/null 2>&1; then
            log_info "主服务健康检查通过"
            break
        fi
        if [ $i -eq 30 ]; then
            log_error "主服务健康检查失败"
            docker-compose logs gemini-proxy
            exit 1
        fi
        sleep 2
    done
    
    # 检查 Prometheus
    if curl -f http://localhost:9091/-/healthy > /dev/null 2>&1; then
        log_info "Prometheus 健康检查通过"
    else
        log_warn "Prometheus 健康检查失败"
    fi
    
    # 检查 Grafana
    if curl -f http://localhost:3000/api/health > /dev/null 2>&1; then
        log_info "Grafana 健康检查通过"
    else
        log_warn "Grafana 健康检查失败"
    fi
}

# 显示部署信息
show_deployment_info() {
    log_info "部署完成！"
    echo ""
    echo "服务访问地址："
    echo "  - 主服务 (代理): http://localhost:8080"
    echo "  - API 管理界面: http://localhost:9090"
    echo "  - Prometheus: http://localhost:9091"
    echo "  - Grafana: http://localhost:3000 (admin/admin)"
    echo ""
    echo "查看服务状态: docker-compose ps"
    echo "查看服务日志: docker-compose logs -f gemini-proxy"
    echo "停止服务: docker-compose down"
}

# 主函数
main() {
    log_info "开始部署 Gemini Proxy..."
    
    # 检查是否在项目根目录
    if [ ! -f "Dockerfile" ] || [ ! -f "docker-compose.yml" ]; then
        log_error "请在项目根目录运行此脚本"
        exit 1
    fi
    
    check_dependencies
    create_directories
    check_config
    build_image
    deploy_services
    health_check
    show_deployment_info
    
    log_info "部署完成！"
}

# 解析命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-build)
            SKIP_BUILD=true
            shift
            ;;
        --help|-h)
            echo "用法: $0 [选项]"
            echo "选项:"
            echo "  --no-build    跳过镜像构建"
            echo "  --help, -h    显示帮助信息"
            exit 0
            ;;
        *)
            log_error "未知参数: $1"
            exit 1
            ;;
    esac
done

# 如果指定跳过构建，则重新定义build_image函数
if [ "$SKIP_BUILD" = true ]; then
    build_image() {
        log_info "跳过镜像构建"
    }
fi

# 运行主函数
main