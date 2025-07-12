// src/utils/performance.rs
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 性能监控器
#[derive(Debug)]
#[allow(dead_code)]
pub struct PerformanceMonitor {
    request_count: AtomicU64,
    total_response_time: AtomicU64,
    error_count: AtomicU64,
    start_time: Instant,
    response_times: Arc<RwLock<Vec<Duration>>>,
}

impl PerformanceMonitor {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
            total_response_time: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            start_time: Instant::now(),
            response_times: Arc::new(RwLock::new(Vec::with_capacity(1000))),
        }
    }

    #[allow(dead_code)]
    pub async fn record_request(&self, response_time: Duration, success: bool) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        if success {
            self.total_response_time.fetch_add(
                response_time.as_millis() as u64, 
                Ordering::Relaxed
            );
        } else {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }

        // 保留最近1000个响应时间用于百分位计算
        let mut times = self.response_times.write().await;
        if times.len() >= 1000 {
            times.remove(0);
        }
        times.push(response_time);
    }

    pub fn get_request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    pub fn get_error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    pub fn get_success_rate(&self) -> f64 {
        let total = self.get_request_count();
        if total == 0 {
            return 1.0;
        }
        let errors = self.get_error_count();
        (total - errors) as f64 / total as f64
    }

    pub fn get_average_response_time(&self) -> Duration {
        let total_requests = self.get_request_count() - self.get_error_count();
        if total_requests == 0 {
            return Duration::from_millis(0);
        }
        let total_time = self.total_response_time.load(Ordering::Relaxed);
        Duration::from_millis(total_time / total_requests)
    }

    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub async fn get_p99_response_time(&self) -> Duration {
        let times = self.response_times.read().await;
        if times.is_empty() {
            return Duration::from_millis(0);
        }

        let mut sorted_times = times.clone();
        sorted_times.sort();
        
        let index = (sorted_times.len() as f64 * 0.99) as usize;
        sorted_times.get(index.saturating_sub(1))
            .copied()
            .unwrap_or(Duration::from_millis(0))
    }

    pub fn get_qps(&self) -> f64 {
        let uptime_secs = self.get_uptime().as_secs_f64();
        if uptime_secs == 0.0 {
            return 0.0;
        }
        self.get_request_count() as f64 / uptime_secs
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 连接池监控
#[derive(Debug)]
#[allow(dead_code)]
pub struct ConnectionPoolMonitor {
    active_connections: AtomicU64,
    max_connections: u64,
    connection_errors: AtomicU64,
}

impl ConnectionPoolMonitor {
    #[allow(dead_code)]
    pub fn new(max_connections: u64) -> Self {
        Self {
            active_connections: AtomicU64::new(0),
            max_connections,
            connection_errors: AtomicU64::new(0),
        }
    }

    #[allow(dead_code)]
    pub fn acquire_connection(&self) -> bool {
        let current = self.active_connections.load(Ordering::Relaxed);
        if current >= self.max_connections {
            return false;
        }
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        true
    }

    #[allow(dead_code)]
    pub fn release_connection(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn record_connection_error(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub fn get_connection_usage(&self) -> f64 {
        self.get_active_connections() as f64 / self.max_connections as f64
    }

    #[allow(dead_code)]
    pub fn get_connection_errors(&self) -> u64 {
        self.connection_errors.load(Ordering::Relaxed)
    }
}

/// 内存使用监控
#[derive(Debug)]
#[allow(dead_code)]
pub struct MemoryMonitor {
    peak_memory_usage: AtomicU64,
}

impl MemoryMonitor {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            peak_memory_usage: AtomicU64::new(0),
        }
    }

    pub fn update_memory_usage(&self, current_usage: u64) {
        self.peak_memory_usage.fetch_max(current_usage, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn get_peak_memory_usage(&self) -> u64 {
        self.peak_memory_usage.load(Ordering::Relaxed)
    }

    pub fn get_current_memory_usage(&self) -> Option<u64> {
        // 简单的内存使用情况检查
        // 在生产环境中可以使用更精确的方法
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(status) = fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(size_str) = line.split_whitespace().nth(1) {
                            if let Ok(size_kb) = size_str.parse::<u64>() {
                                return Some(size_kb * 1024); // 转换为字节
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 综合性能统计
#[derive(Debug, serde::Serialize)]
pub struct PerformanceStats {
    pub qps: f64,
    pub success_rate: f64,
    pub avg_response_time_ms: u64,
    pub p99_response_time_ms: u64,
    pub active_connections: u64,
    pub connection_usage: f64,
    pub memory_usage_bytes: Option<u64>,
    pub uptime_seconds: u64,
}

/// 性能优化工具
#[allow(dead_code)]
pub struct PerformanceOptimizer {
    performance_monitor: Arc<PerformanceMonitor>,
    connection_monitor: Arc<ConnectionPoolMonitor>,
    memory_monitor: Arc<MemoryMonitor>,
}

impl PerformanceOptimizer {
    #[allow(dead_code)]
    pub fn new(max_connections: u64) -> Self {
        Self {
            performance_monitor: Arc::new(PerformanceMonitor::new()),
            connection_monitor: Arc::new(ConnectionPoolMonitor::new(max_connections)),
            memory_monitor: Arc::new(MemoryMonitor::new()),
        }
    }

    pub async fn get_performance_stats(&self) -> PerformanceStats {
        let memory_usage = self.memory_monitor.get_current_memory_usage();
        if let Some(usage) = memory_usage {
            self.memory_monitor.update_memory_usage(usage);
        }

        PerformanceStats {
            qps: self.performance_monitor.get_qps(),
            success_rate: self.performance_monitor.get_success_rate(),
            avg_response_time_ms: self.performance_monitor.get_average_response_time().as_millis() as u64,
            p99_response_time_ms: self.performance_monitor.get_p99_response_time().await.as_millis() as u64,
            active_connections: self.connection_monitor.get_active_connections(),
            connection_usage: self.connection_monitor.get_connection_usage(),
            memory_usage_bytes: memory_usage,
            uptime_seconds: self.performance_monitor.get_uptime().as_secs(),
        }
    }

    #[allow(dead_code)]
    pub fn get_performance_monitor(&self) -> Arc<PerformanceMonitor> {
        self.performance_monitor.clone()
    }

    #[allow(dead_code)]
    pub fn get_connection_monitor(&self) -> Arc<ConnectionPoolMonitor> {
        self.connection_monitor.clone()
    }

    #[allow(dead_code)]
    pub fn get_memory_monitor(&self) -> Arc<MemoryMonitor> {
        self.memory_monitor.clone()
    }

    #[allow(dead_code)]
    pub async fn log_performance_summary(&self) {
        let stats = self.get_performance_stats().await;
        
        tracing::info!(
            qps = stats.qps,
            success_rate = stats.success_rate,
            avg_response_time_ms = stats.avg_response_time_ms,
            p99_response_time_ms = stats.p99_response_time_ms,
            active_connections = stats.active_connections,
            connection_usage = stats.connection_usage,
            memory_usage_mb = stats.memory_usage_bytes.map(|b| b / 1024 / 1024),
            uptime_hours = stats.uptime_seconds / 3600,
            "Performance summary"
        );
    }
}