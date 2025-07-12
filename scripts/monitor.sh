#!/bin/bash
# Gemini Proxy 监控脚本

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

log_debug() {
    echo -e "${BLUE}[DEBUG]${NC} $1"
}

# 检查服务状态
check_service_status() {
    log_info "检查服务状态..."
    
    # 检查 Docker 容器状态
    if docker-compose ps | grep -q "Up"; then
        log_info "Docker 容器运行正常"
    else
        log_error "Docker 容器未运行"
        return 1
    fi
    
    # 检查主服务健康状态 (HTTPS 主服务或 HTTP 管理接口)
    if curl -f -k https://localhost:8443/health > /dev/null 2>&1 || curl -f http://localhost:9090/health > /dev/null 2>&1; then
        log_info "主服务健康检查通过"
    else
        log_error "主服务健康检查失败"
        return 1
    fi
    
    return 0
}

# 获取性能指标
get_performance_metrics() {
    log_info "获取性能指标..."
    
    # 获取QPS
    local qps=$(curl -s http://localhost:9090/performance | jq -r '.qps // 0')
    # 获取成功率
    local success_rate=$(curl -s http://localhost:9090/performance | jq -r '.success_rate // 0')
    # 获取平均响应时间
    local avg_response_time=$(curl -s http://localhost:9090/performance | jq -r '.avg_response_time_ms // 0')
    # 获取活跃连接数
    local active_connections=$(curl -s http://localhost:9090/performance | jq -r '.active_connections // 0')
    
    echo "性能指标:"
    echo "  QPS: $qps"
    echo "  成功率: $(echo "scale=2; $success_rate * 100" | bc)%"
    echo "  平均响应时间: ${avg_response_time}ms"
    echo "  活跃连接数: $active_connections"
}

# 获取错误统计
get_error_statistics() {
    log_info "获取错误统计..."
    
    local error_stats=$(curl -s http://localhost:9090/errors)
    local total_errors=$(echo "$error_stats" | jq -r '.total_errors // 0')
    local recent_errors=$(echo "$error_stats" | jq -r '.recent_errors // 0')
    
    echo "错误统计:"
    echo "  总错误数: $total_errors"
    echo "  最近1小时错误数: $recent_errors"
    
    # 按严重级别统计
    echo "  按严重级别:"
    echo "$error_stats" | jq -r '.by_severity | to_entries[] | "    \(.key): \(.value)"' 2>/dev/null || echo "    无数据"
}

# 检查资源使用情况
check_resource_usage() {
    log_info "检查资源使用情况..."
    
    # 检查容器资源使用
    echo "容器资源使用:"
    docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}"
    
    # 检查磁盘使用
    echo -e "\n磁盘使用:"
    df -h . | tail -n 1
    
    # 检查日志文件大小
    if [ -d "logs" ]; then
        echo -e "\n日志文件大小:"
        du -sh logs/*
    fi
}

# 检查 API 密钥状态
check_api_keys() {
    log_info "检查 API 密钥状态..."
    
    local health_data=$(curl -s http://localhost:9090/health)
    echo "$health_data" | jq '.api_keys[] | "API密钥 \(.id): \(if .is_healthy then "健康" else "异常" end) (失败次数: \(.failure_count))"' -r 2>/dev/null || echo "无法获取API密钥状态"
}

# 实时监控
real_time_monitor() {
    log_info "开始实时监控 (按 Ctrl+C 停止)..."
    
    while true; do
        clear
        echo "=== Gemini Proxy 实时监控 $(date) ==="
        echo ""
        
        if check_service_status > /dev/null 2>&1; then
            get_performance_metrics
            echo ""
            get_error_statistics
            echo ""
            check_api_keys
        else
            log_error "服务未运行或异常"
        fi
        
        echo ""
        echo "按 Ctrl+C 停止监控"
        sleep 5
    done
}

# 生成监控报告
generate_report() {
    local report_file="monitoring-report-$(date +%Y%m%d_%H%M%S).txt"
    
    log_info "生成监控报告: $report_file"
    
    {
        echo "=== Gemini Proxy 监控报告 ==="
        echo "生成时间: $(date)"
        echo ""
        
        echo "=== 服务状态 ==="
        if check_service_status > /dev/null 2>&1; then
            echo "状态: 正常"
        else
            echo "状态: 异常"
        fi
        echo ""
        
        echo "=== 性能指标 ==="
        get_performance_metrics
        echo ""
        
        echo "=== 错误统计 ==="
        get_error_statistics
        echo ""
        
        echo "=== 资源使用 ==="
        check_resource_usage
        echo ""
        
        echo "=== API 密钥状态 ==="
        check_api_keys
        echo ""
        
        echo "=== Docker 容器状态 ==="
        docker-compose ps
        echo ""
        
        echo "=== 最近日志 (最后100行) ==="
        docker-compose logs --tail=100 gemini-proxy
        
    } > "$report_file"
    
    log_info "监控报告已生成: $report_file"
}

# 检查告警条件
check_alerts() {
    log_info "检查告警条件..."
    
    local alerts=()
    
    # 检查服务状态
    if ! check_service_status > /dev/null 2>&1; then
        alerts+=("🔴 服务状态异常")
    fi
    
    # 检查成功率
    local success_rate=$(curl -s http://localhost:9090/performance | jq -r '.success_rate // 1')
    if (( $(echo "$success_rate < 0.95" | bc -l) )); then
        alerts+=("🟡 成功率低于95%: $(echo "scale=2; $success_rate * 100" | bc)%")
    fi
    
    # 检查响应时间
    local avg_response_time=$(curl -s http://localhost:9090/performance | jq -r '.avg_response_time_ms // 0')
    if (( $(echo "$avg_response_time > 2000" | bc -l) )); then
        alerts+=("🟡 平均响应时间过长: ${avg_response_time}ms")
    fi
    
    # 检查最近错误
    local recent_errors=$(curl -s http://localhost:9090/errors | jq -r '.recent_errors // 0')
    if (( recent_errors > 10 )); then
        alerts+=("🟡 最近1小时错误数过多: $recent_errors")
    fi
    
    # 输出告警
    if [ ${#alerts[@]} -eq 0 ]; then
        log_info "✅ 无告警"
    else
        log_warn "发现 ${#alerts[@]} 个告警:"
        for alert in "${alerts[@]}"; do
            echo "  $alert"
        done
    fi
}

# 显示帮助信息
show_help() {
    echo "Gemini Proxy 监控脚本"
    echo ""
    echo "用法:"
    echo "  $0 [命令] [选项]"
    echo ""
    echo "命令:"
    echo "  status      显示服务状态"
    echo "  metrics     显示性能指标"
    echo "  errors      显示错误统计"
    echo "  resources   显示资源使用情况"
    echo "  keys        显示API密钥状态"
    echo "  watch       实时监控"
    echo "  report      生成监控报告"
    echo "  alerts      检查告警条件"
    echo ""
    echo "选项:"
    echo "  --help, -h  显示帮助信息"
}

# 主函数
main() {
    case "$1" in
        status)
            check_service_status
            ;;
        metrics)
            get_performance_metrics
            ;;
        errors)
            get_error_statistics
            ;;
        resources)
            check_resource_usage
            ;;
        keys)
            check_api_keys
            ;;
        watch)
            real_time_monitor
            ;;
        report)
            generate_report
            ;;
        alerts)
            check_alerts
            ;;
        --help|-h)
            show_help
            ;;
        "")
            # 默认显示概要信息
            check_service_status && echo ""
            get_performance_metrics && echo ""
            check_alerts
            ;;
        *)
            log_error "未知命令: $1"
            show_help
            exit 1
            ;;
    esac
}

# 检查依赖
if ! command -v jq &> /dev/null; then
    log_warn "jq 未安装，某些功能可能无法正常工作"
fi

if ! command -v bc &> /dev/null; then
    log_warn "bc 未安装，某些计算功能可能无法正常工作"
fi

# 运行主函数
main "$@"