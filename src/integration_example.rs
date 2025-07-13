// src/integration_example.rs
//! 统一错误系统集成示例
//! 
//! 展示如何在现有代码中集成新的错误处理系统

use crate::error::{
    GeminiProxyError, ErrorSeverity,
    logging::{ErrorLogger, create_production_logger},
    recovery::{ErrorRecoveryManager, create_production_recovery_manager},
    migration::HybridErrorHandler,
};
use crate::config::{ProxyConfig, ConfigValidator};
use std::path::PathBuf;

/// 集成示例管理器
pub struct IntegratedErrorDemo {
    /// 错误日志记录器
    logger: ErrorLogger,
    /// 错误恢复管理器
    recovery_manager: ErrorRecoveryManager,
    /// 混合错误处理器（支持新旧两种系统）
    hybrid_handler: HybridErrorHandler,
}

impl IntegratedErrorDemo {
    /// 创建集成示例
    pub fn new() -> Self {
        // 创建日志目录
        let log_dir = PathBuf::from("logs");
        std::fs::create_dir_all(&log_dir).ok();

        // 初始化各个组件
        let logger = create_production_logger(log_dir);
        let recovery_manager = create_production_recovery_manager();
        let hybrid_handler = HybridErrorHandler::new(1000);

        Self {
            logger,
            recovery_manager,
            hybrid_handler,
        }
    }

    /// 演示配置加载中的错误处理
    pub async fn demo_config_loading(&self) -> Result<(), String> {
        println!("🔧 演示配置加载错误处理...");

        // 尝试加载配置文件
        match ProxyConfig::from_file_enhanced("config/proxy.yaml") {
            Ok(config) => {
                println!("✅ 配置加载成功");
                
                // 验证配置
                if let Err(validation_error) = ConfigValidator::validate_proxy_config(&config) {
                    println!("❌ 配置验证失败");
                    self.logger.log_error(&validation_error).await?;
                    
                    // 展示验证错误的详细信息
                    if let GeminiProxyError::Validation { fields, .. } = validation_error {
                        println!("验证错误详情:");
                        for field in fields {
                            println!("  - {}: {}", field.field, field.message);
                        }
                    }
                }
            }
            Err(error) => {
                println!("❌ 配置加载失败: {}", error);
                self.logger.log_error(&error).await?;
            }
        }

        Ok(())
    }

    /// 演示网络错误恢复
    pub async fn demo_network_error_recovery(&self) -> Result<(), String> {
        println!("\n🌐 演示网络错误恢复...");

        // 模拟网络请求
        let network_operation = || async {
            // 模拟随机的网络失败
            if rand::random::<f64>() < 0.7 {
                Err(GeminiProxyError::network("连接超时")
                    .with_severity(ErrorSeverity::Error)
                    .with_retryable(true)
                    .with_recovery_hint("检查网络连接或稍后重试"))
            } else {
                Ok("网络请求成功".to_string())
            }
        };

        // 使用恢复管理器处理网络错误
        let initial_error = GeminiProxyError::network("初始网络错误");
        match self.recovery_manager.attempt_recovery(&initial_error, network_operation).await {
            Ok(result) => {
                println!("✅ 网络请求最终成功: {}", result);
            }
            Err(final_error) => {
                println!("❌ 网络请求最终失败: {}", final_error);
                self.logger.log_error(&final_error).await?;
            }
        }

        Ok(())
    }

    /// 演示API密钥错误处理
    pub async fn demo_api_key_error_handling(&self) -> Result<(), String> {
        println!("\n🔑 演示API密钥错误处理...");

        // 模拟API密钥错误
        let api_error = GeminiProxyError::load_balancer("API密钥已过期")
            .with_severity(ErrorSeverity::Critical)
            .with_metadata("key_id", "gemini-key-001")
            .with_metadata("error_code", "401")
            .with_recovery_hint("请更新API密钥");

        // 记录错误
        self.logger.log_error(&api_error).await?;

        // 报告熔断器状态
        self.recovery_manager.report_operation_result("gemini_api", false).await;

        // 检查熔断器状态
        if !self.recovery_manager.check_circuit_breaker("gemini_api").await {
            println!("⚠️  API熔断器已激活，暂停请求");
        }

        Ok(())
    }

    /// 演示旧错误系统的兼容性
    pub async fn demo_legacy_compatibility(&self) -> Result<(), String> {
        println!("\n🔄 演示新旧错误系统兼容性...");

        // 创建旧的错误类型
        use crate::utils::error::{ProxyError, ErrorSeverity as OldSeverity, ErrorContext as OldContext};

        let old_error = ProxyError::Authentication {
            message: "JWT令牌无效".to_string(),
        };

        let old_context = OldContext::new(
            old_error,
            OldSeverity::Medium,
            "auth_middleware",
        );

        // 使用混合处理器处理旧错误
        self.hybrid_handler.handle_legacy_error(old_context).await;

        println!("✅ 旧错误已通过兼容层处理");
        Ok(())
    }

    /// 演示错误聚合功能
    pub async fn demo_error_aggregation(&self) -> Result<(), String> {
        println!("\n📊 演示错误聚合功能...");

        // 连续产生相同类型的错误
        for i in 1..=5 {
            let error = GeminiProxyError::network(format!("连接失败 #{}", i))
                .with_severity(ErrorSeverity::Warning)
                .with_metadata("attempt", i.to_string());

            self.logger.log_error(&error).await?;
        }

        // 记录聚合的错误摘要
        self.logger.log_aggregated_errors().await?;

        println!("✅ 错误聚合演示完成");
        Ok(())
    }

    /// 获取综合统计信息
    pub async fn get_comprehensive_stats(&self) -> Result<(), String> {
        println!("\n📈 获取综合错误统计...");

        // 获取新系统的统计
        let logging_stats = self.logger.get_statistics().await;
        let recovery_stats = self.recovery_manager.get_statistics().await;

        // 获取混合处理器的统计（包含新旧系统）
        let hybrid_stats = self.hybrid_handler.get_statistics().await;

        println!("错误日志统计:");
        println!("  总记录数: {}", logging_stats.total_errors_logged);
        println!("  抑制数: {}", logging_stats.suppressed_errors);

        println!("错误恢复统计:");
        println!("  恢复尝试: {}", recovery_stats.total_recovery_attempts);
        println!("  成功率: {:.1}%", recovery_stats.recovery_rate * 100.0);
        println!("  熔断器触发: {}", recovery_stats.circuit_breaker_trips);

        println!("混合处理器统计:");
        println!("  总错误数: {}", hybrid_stats.total_errors);
        println!("  最近错误数: {}", hybrid_stats.recent_errors);

        Ok(())
    }

    /// 运行完整的集成演示
    pub async fn run_full_demo(&self) -> Result<(), String> {
        println!("🚀 开始统一错误系统集成演示");
        println!("=====================================");

        // 运行各种演示
        self.demo_config_loading().await?;
        self.demo_network_error_recovery().await?;
        self.demo_api_key_error_handling().await?;
        self.demo_legacy_compatibility().await?;
        self.demo_error_aggregation().await?;

        // 显示统计信息
        self.get_comprehensive_stats().await?;

        println!("\n✅ 统一错误系统集成演示完成！");
        println!("=====================================");

        Ok(())
    }
}

/// 在主函数中运行演示的辅助函数
pub async fn run_error_system_demo() {
    let demo = IntegratedErrorDemo::new();
    
    if let Err(e) = demo.run_full_demo().await {
        eprintln!("演示运行失败: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_demo() {
        let demo = IntegratedErrorDemo::new();
        
        // 测试基本功能
        assert!(demo.demo_config_loading().await.is_ok());
        assert!(demo.demo_legacy_compatibility().await.is_ok());
    }

    #[tokio::test]
    async fn test_error_logging() {
        let demo = IntegratedErrorDemo::new();
        
        let test_error = GeminiProxyError::config("测试配置错误")
            .with_severity(ErrorSeverity::Error);

        assert!(demo.logger.log_error(&test_error).await.is_ok());
    }

    #[tokio::test]
    async fn test_error_recovery() {
        let demo = IntegratedErrorDemo::new();
        
        let test_operation = || async {
            Ok("测试成功".to_string())
        };

        let test_error = GeminiProxyError::network("测试网络错误");
        let result = demo.recovery_manager.attempt_recovery(&test_error, test_operation).await;
        
        assert!(result.is_ok());
    }
}