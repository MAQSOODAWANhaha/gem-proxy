#!/bin/bash
# Gemini Proxy 备份脚本

set -e

# 颜色输出
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
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

# 默认配置
BACKUP_DIR="${BACKUP_DIR:-./backups}"
KEEP_DAYS="${KEEP_DAYS:-7}"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_NAME="gemini-proxy-backup-${TIMESTAMP}"

# 创建备份目录
create_backup_dir() {
    if [ ! -d "$BACKUP_DIR" ]; then
        mkdir -p "$BACKUP_DIR"
        log_info "创建备份目录: $BACKUP_DIR"
    fi
}

# 备份配置文件
backup_config() {
    log_info "备份配置文件..."
    
    if [ -d "config" ]; then
        cp -r config "$BACKUP_DIR/${BACKUP_NAME}-config"
        log_info "配置文件备份完成"
    else
        log_warn "配置目录不存在，跳过配置备份"
    fi
}

# 备份 Docker 数据卷
backup_volumes() {
    log_info "备份 Docker 数据卷..."
    
    # 创建数据卷备份目录
    VOLUME_BACKUP_DIR="$BACKUP_DIR/${BACKUP_NAME}-volumes"
    mkdir -p "$VOLUME_BACKUP_DIR"
    
    # 备份 Prometheus 数据
    if docker volume inspect gemini-proxy_prometheus_data > /dev/null 2>&1; then
        log_info "备份 Prometheus 数据..."
        docker run --rm -v gemini-proxy_prometheus_data:/data -v "$(pwd)/$VOLUME_BACKUP_DIR:/backup" alpine tar czf /backup/prometheus_data.tar.gz -C /data .
    fi
    
    # 备份 Grafana 数据
    if docker volume inspect gemini-proxy_grafana_data > /dev/null 2>&1; then
        log_info "备份 Grafana 数据..."
        docker run --rm -v gemini-proxy_grafana_data:/data -v "$(pwd)/$VOLUME_BACKUP_DIR:/backup" alpine tar czf /backup/grafana_data.tar.gz -C /data .
    fi
    
    # 备份 Redis 数据
    if docker volume inspect gemini-proxy_redis_data > /dev/null 2>&1; then
        log_info "备份 Redis 数据..."
        docker run --rm -v gemini-proxy_redis_data:/data -v "$(pwd)/$VOLUME_BACKUP_DIR:/backup" alpine tar czf /backup/redis_data.tar.gz -C /data .
    fi
    
    log_info "数据卷备份完成"
}

# 备份日志文件
backup_logs() {
    log_info "备份日志文件..."
    
    if [ -d "logs" ]; then
        cp -r logs "$BACKUP_DIR/${BACKUP_NAME}-logs"
        log_info "日志文件备份完成"
    else
        log_warn "日志目录不存在，跳过日志备份"
    fi
}

# 备份证书文件
backup_certs() {
    log_info "备份证书文件..."
    
    if [ -d "certs" ]; then
        cp -r certs "$BACKUP_DIR/${BACKUP_NAME}-certs"
        log_info "证书文件备份完成"
    else
        log_warn "证书目录不存在，跳过证书备份"
    fi
}

# 创建压缩包
create_archive() {
    log_info "创建备份压缩包..."
    
    cd "$BACKUP_DIR"
    tar czf "${BACKUP_NAME}.tar.gz" ${BACKUP_NAME}-*
    
    # 删除临时目录
    rm -rf ${BACKUP_NAME}-*
    
    log_info "备份压缩包创建完成: $BACKUP_DIR/${BACKUP_NAME}.tar.gz"
}

# 清理旧备份
cleanup_old_backups() {
    log_info "清理 ${KEEP_DAYS} 天前的备份..."
    
    find "$BACKUP_DIR" -name "gemini-proxy-backup-*.tar.gz" -mtime +${KEEP_DAYS} -delete
    
    log_info "旧备份清理完成"
}

# 恢复备份
restore_backup() {
    local backup_file="$1"
    
    if [ -z "$backup_file" ]; then
        log_error "请指定要恢复的备份文件"
        echo "用法: $0 --restore <backup-file>"
        exit 1
    fi
    
    if [ ! -f "$backup_file" ]; then
        log_error "备份文件不存在: $backup_file"
        exit 1
    fi
    
    log_info "恢复备份: $backup_file"
    
    # 停止服务
    log_info "停止服务..."
    docker-compose down
    
    # 解压备份
    TEMP_DIR=$(mktemp -d)
    tar xzf "$backup_file" -C "$TEMP_DIR"
    
    # 恢复配置
    if [ -d "$TEMP_DIR"/*-config ]; then
        log_info "恢复配置文件..."
        rm -rf config
        cp -r "$TEMP_DIR"/*-config config
    fi
    
    # 恢复证书
    if [ -d "$TEMP_DIR"/*-certs ]; then
        log_info "恢复证书文件..."
        rm -rf certs
        cp -r "$TEMP_DIR"/*-certs certs
    fi
    
    # 恢复数据卷
    if [ -d "$TEMP_DIR"/*-volumes ]; then
        log_info "恢复数据卷..."
        
        # 恢复 Prometheus 数据
        if [ -f "$TEMP_DIR"/*-volumes/prometheus_data.tar.gz ]; then
            docker volume rm -f gemini-proxy_prometheus_data
            docker volume create gemini-proxy_prometheus_data
            docker run --rm -v gemini-proxy_prometheus_data:/data -v "$TEMP_DIR"/*-volumes:/backup alpine tar xzf /backup/prometheus_data.tar.gz -C /data
        fi
        
        # 恢复 Grafana 数据
        if [ -f "$TEMP_DIR"/*-volumes/grafana_data.tar.gz ]; then
            docker volume rm -f gemini-proxy_grafana_data
            docker volume create gemini-proxy_grafana_data
            docker run --rm -v gemini-proxy_grafana_data:/data -v "$TEMP_DIR"/*-volumes:/backup alpine tar xzf /backup/grafana_data.tar.gz -C /data
        fi
        
        # 恢复 Redis 数据
        if [ -f "$TEMP_DIR"/*-volumes/redis_data.tar.gz ]; then
            docker volume rm -f gemini-proxy_redis_data
            docker volume create gemini-proxy_redis_data
            docker run --rm -v gemini-proxy_redis_data:/data -v "$TEMP_DIR"/*-volumes:/backup alpine tar xzf /backup/redis_data.tar.gz -C /data
        fi
    fi
    
    # 清理临时目录
    rm -rf "$TEMP_DIR"
    
    log_info "备份恢复完成"
    log_info "请运行 'docker-compose up -d' 启动服务"
}

# 显示帮助信息
show_help() {
    echo "Gemini Proxy 备份脚本"
    echo ""
    echo "用法:"
    echo "  $0 [选项]                   # 创建备份"
    echo "  $0 --restore <backup-file>  # 恢复备份"
    echo ""
    echo "选项:"
    echo "  --backup-dir <dir>    备份目录 (默认: ./backups)"
    echo "  --keep-days <days>    保留天数 (默认: 7)"
    echo "  --no-volumes          跳过数据卷备份"
    echo "  --restore <file>      恢复指定备份"
    echo "  --help, -h            显示帮助信息"
    echo ""
    echo "环境变量:"
    echo "  BACKUP_DIR            备份目录"
    echo "  KEEP_DAYS             保留天数"
}

# 主函数
main() {
    log_info "开始备份 Gemini Proxy..."
    
    create_backup_dir
    backup_config
    
    if [ "$SKIP_VOLUMES" != "true" ]; then
        backup_volumes
    fi
    
    backup_logs
    backup_certs
    create_archive
    cleanup_old_backups
    
    log_info "备份完成: $BACKUP_DIR/${BACKUP_NAME}.tar.gz"
}

# 解析命令行参数
SKIP_VOLUMES=false
RESTORE_MODE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --backup-dir)
            BACKUP_DIR="$2"
            shift 2
            ;;
        --keep-days)
            KEEP_DAYS="$2"
            shift 2
            ;;
        --no-volumes)
            SKIP_VOLUMES=true
            shift
            ;;
        --restore)
            RESTORE_MODE=true
            RESTORE_FILE="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            log_error "未知参数: $1"
            show_help
            exit 1
            ;;
    esac
done

# 执行相应操作
if [ "$RESTORE_MODE" = true ]; then
    restore_backup "$RESTORE_FILE"
else
    main
fi