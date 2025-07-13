// 核心负载均衡模块
pub mod unified_key_manager;  // 统一密钥管理器（主要使用）

// 向后兼容和备用实现（保留但不导出以避免警告）
pub mod key_manager;          // 旧版密钥管理器（已被 unified_key_manager 替代）
pub mod weighted_round_robin; // 旧版轮询调度器（已被 unified_key_manager 集成）

// 未来功能模块（保留声明）
pub mod scheduler;   // 调度策略（未实现）
pub mod optimizer;   // 权重优化器（未实现）
pub mod audit;       // 审计系统（未实现）
pub mod tools;       // 管理工具（未实现）

// 仅导出当前使用的统一管理器
pub use unified_key_manager::*;

// 移除未使用的导入以消除编译警告
// pub use key_manager::*;          // 移除：已被 unified_key_manager 替代
// pub use weighted_round_robin::*; // 移除：已被 unified_key_manager 替代
// pub use scheduler::*;            // 移除：未使用的调度器
// pub use optimizer::*;  // 移除：未使用的优化器
// pub use audit::*;      // 移除：未使用的审计系统
// pub use tools::*;      // 移除：未使用的工具集
