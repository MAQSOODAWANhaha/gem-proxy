// src/utils/health_check.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: u64,
    pub checks: HashMap<String, CheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub status: String,
    pub message: String,
    pub duration_ms: u64,
}

#[derive(Debug)]
pub struct HealthChecker {
    api_keys_total: usize,
    api_keys_available: usize,
    config_loaded: bool,
}

impl HealthChecker {
    pub fn new(api_keys_total: usize, api_keys_available: usize, config_loaded: bool) -> Self {
        Self {
            api_keys_total,
            api_keys_available,
            config_loaded,
        }
    }

    pub async fn check_health(&self) -> HealthStatus {
        let mut checks = HashMap::new();
        let mut overall_status = "healthy";

        // System check
        let system_result = self.check_system().await;
        if system_result.status != "healthy" {
            overall_status = "unhealthy";
        }
        checks.insert("system".to_string(), system_result);

        // Configuration check
        let config_result = self.check_config().await;
        if config_result.status != "healthy" {
            overall_status = "unhealthy";
        }
        checks.insert("configuration".to_string(), config_result);

        // API Keys check
        let api_keys_result = self.check_api_keys().await;
        if api_keys_result.status != "healthy" {
            overall_status = "unhealthy";
        }
        checks.insert("api_keys".to_string(), api_keys_result);

        HealthStatus {
            status: overall_status.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checks,
        }
    }

    async fn check_system(&self) -> CheckResult {
        let start = std::time::Instant::now();
        
        // Check system memory and basic functionality
        let result = std::panic::catch_unwind(|| {
            // Simple memory allocation test
            let _test_vec: Vec<u8> = vec![0; 1024];
            "healthy"
        });

        let duration = start.elapsed();

        match result {
            Ok(_) => CheckResult {
                status: "healthy".to_string(),
                message: "System is operational".to_string(),
                duration_ms: duration.as_millis() as u64,
            },
            Err(_) => CheckResult {
                status: "unhealthy".to_string(),
                message: "System check failed".to_string(),
                duration_ms: duration.as_millis() as u64,
            },
        }
    }

    async fn check_config(&self) -> CheckResult {
        let start = std::time::Instant::now();
        
        let (status, message) = if self.config_loaded {
            ("healthy", "Configuration loaded successfully")
        } else {
            ("unhealthy", "Configuration not loaded")
        };

        CheckResult {
            status: status.to_string(),
            message: message.to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    async fn check_api_keys(&self) -> CheckResult {
        let start = std::time::Instant::now();
        
        let status = if self.api_keys_available == 0 {
            "unhealthy"
        } else if self.api_keys_available < self.api_keys_total / 2 {
            "degraded"
        } else {
            "healthy"
        };

        let message = format!(
            "{}/{} API keys available",
            self.api_keys_available, self.api_keys_total
        );

        CheckResult {
            status: status.to_string(),
            message,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new(0, 0, false)
    }
}