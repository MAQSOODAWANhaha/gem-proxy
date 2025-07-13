// src/api/mod.rs
// 仅导出实际使用的模块，清理未使用的导入

pub mod config;
pub mod handlers;
pub mod weight_management;
pub mod load_balancing_stats;
pub mod auth;

// 未来功能模块（暂时保留声明但不导出）
// pub mod intelligent_optimization;  // 智能优化功能（未实现）
// pub mod weight_audit;             // 权重审计功能（未实现）
// pub mod weight_tools;             // 权重工具功能（未实现）

// 仅导出核心API模块，避免未使用的导入警告
// 如果需要特定类型，请在使用处显式导入
// pub use config::*;     // 移除：未使用的通配符导入
// pub use handlers::*;   // 移除：未使用的通配符导入  
// pub use weight_management::*;  // 移除：未使用的通配符导入
// pub use load_balancing_stats::*; // 移除：未使用的通配符导入
// pub use auth::*;       // 移除：未使用的通配符导入