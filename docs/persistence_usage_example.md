# 持久化功能使用示例

这个文档展示了如何在 Gemini Proxy 项目中使用数据持久化功能。

## 代码示例

```rust

// 注意：这是一个演示示例，实际使用时需要根据项目结构调整导入路径
// use gemini_proxy::persistence::*;
// use gemini_proxy::persistence::weight_presets::*;
// use gemini_proxy::persistence::config_history::*;
// use gemini_proxy::persistence::session_store::*;

// 为演示目的，使用本地导入
use std::collections::HashMap;
use std::path::PathBuf;
use std::collections::HashMap;
// use tempfile::tempdir;
// use tokio;

// 模拟类型定义（实际使用时应使用真实的类型）
struct PersistenceConfig {
    data_dir: PathBuf,
    enable_compression: bool,
    backup_retention_days: u32,
    auto_backup_interval: u64,
    max_file_size: u64,
}

struct StorageManager;
struct WeightPresetStore;
struct WeightPreset;
struct PresetQuery;
struct ConfigHistoryStore;
struct ConfigHistoryConfig;
struct SessionStore;
struct SessionStoreConfig;
struct ClientInfo;

// 模拟枚举
enum ConfigChangeType {
    Update,
}

enum ChangeSource {
    WebUI,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🚀 Gemini Proxy 持久化功能演示");
    println!("================================");
    
    // 创建临时目录用于演示
    let temp_dir = tempdir()?;
    let persistence_config = PersistenceConfig {
        data_dir: temp_dir.path().to_path_buf(),
        enable_compression: false,
        backup_retention_days: 7,
        auto_backup_interval: 3600,
        max_file_size: 1024 * 1024, // 1MB
    };
    
    // 演示存储管理器
    demo_storage_manager(&persistence_config).await?;
    
    // 演示权重预设存储
    demo_weight_presets(&persistence_config).await?;
    
    // 演示配置历史记录
    demo_config_history(&persistence_config).await?;
    
    // 演示会话状态管理
    demo_session_store(&persistence_config).await?;
    
    println!("\n✅ 所有持久化功能演示完成！");
    
    Ok(())
}

async fn demo_storage_manager(config: &PersistenceConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📦 存储管理器演示");
    println!("------------------");
    
    let storage_manager = StorageManager::new(config.clone());
    
    // 初始化存储
    storage_manager.initialize().await?;
    println!("✓ 存储管理器初始化完成");
    
    // 获取存储统计
    let stats = storage_manager.get_storage_stats().await?;
    println!("✓ 存储统计: {} 个文件, {} 字节", stats.file_count, stats.total_size);
    
    Ok(())
}

async fn demo_weight_presets(config: &PersistenceConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚖️  权重预设存储演示");
    println!("--------------------");
    
    let preset_store = WeightPresetStore::new(config.clone(), true);
    preset_store.initialize().await?;
    
    // 创建示例权重预设
    let mut weights = HashMap::new();
    weights.insert("gemini-pro".to_string(), 300);
    weights.insert("gemini-pro-vision".to_string(), 200);
    weights.insert("gemini-flash".to_string(), 500);
    
    let preset = WeightPreset {
        id: uuid::Uuid::new_v4().to_string(),
        name: "生产环境均衡配置".to_string(),
        description: "适用于生产环境的权重分配方案，优先使用 Flash 模型".to_string(),
        weights: weights.clone(),
        created_by: "admin".to_string(),
        created_at: chrono::Utc::now().timestamp() as u64,
        tags: vec!["production".to_string(), "balanced".to_string()],
    };
    
    // 保存预设
    preset_store.save_preset(&preset).await?;
    println!("✓ 已保存权重预设: {}", preset.name);
    
    // 查询预设
    let query = PresetQuery {
        tags: Some(vec!["production".to_string()]),
        ..Default::default()
    };
    let results = preset_store.query_presets(&query).await?;
    println!("✓ 查询到 {} 个生产环境预设", results.len());
    
    // 复制预设
    let duplicated = preset_store.duplicate_preset(
        &preset.id,
        "开发环境测试配置",
        "developer"
    ).await?;
    println!("✓ 已复制预设: {}", duplicated.name);
    
    // 获取统计信息
    let stats = preset_store.get_statistics().await?;
    println!("✓ 预设统计: 总计 {} 个，平均每个预设 {:.1} 个密钥", 
        stats.total_presets, stats.average_keys_per_preset);
    
    // 导出预设
    let exported = preset_store.export_presets(None).await?;
    println!("✓ 已导出预设数据，大小: {} 字节", exported.len());
    
    Ok(())
}

async fn demo_config_history(config: &PersistenceConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📜 配置历史记录演示");
    println!("--------------------");
    
    let history_store = ConfigHistoryStore::new(
        config.clone(),
        ConfigHistoryConfig::default()
    );
    history_store.initialize().await?;
    
    // 记录配置变更
    let old_config = r#"{"api_keys": [{"id": "key1", "weight": 100}]}"#;
    let new_config = r#"{"api_keys": [{"id": "key1", "weight": 150}, {"id": "key2", "weight": 100}]}"#;
    
    let change_id = history_store.record_change(
        "admin",
        ConfigChangeType::Update,
        "增加新的 API 密钥并调整权重",
        Some(old_config),
        new_config,
        vec!["api_keys".to_string()],
        ChangeSource::WebUI,
        None,
    ).await?;
    println!("✓ 已记录配置变更: {}", change_id);
    
    // 创建配置快照
    let snapshot_id = history_store.create_snapshot(
        "admin",
        "重要配置变更后的快照",
        new_config,
        false,
        Some(change_id),
    ).await?;
    println!("✓ 已创建配置快照: {}", snapshot_id);
    
    // 查询变更历史
    let query = ConfigHistoryQuery {
        operator: Some("admin".to_string()),
        limit: Some(10),
        ..Default::default()
    };
    let changes = history_store.query_changes(&query).await?;
    println!("✓ 查询到 {} 条配置变更记录", changes.len());
    
    // 获取统计信息
    let stats = history_store.get_statistics(Some(30)).await?;
    println!("✓ 配置历史统计: {} 次变更，平均每日 {:.1} 次", 
        stats.total_changes, stats.average_changes_per_day);
    
    // 模拟回滚操作
    if let Some(latest_change) = changes.first() {
        let rollback_id = history_store.rollback_to_version(
            latest_change.version,
            "admin",
            "测试回滚功能"
        ).await?;
        println!("✓ 已创建回滚记录: {}", rollback_id);
    }
    
    Ok(())
}

async fn demo_session_store(config: &PersistenceConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔐 会话状态管理演示");
    println!("--------------------");
    
    let session_store = SessionStore::new(
        config.clone(),
        SessionStoreConfig::default()
    );
    session_store.initialize().await?;
    
    // 创建客户端信息
    let client_info = ClientInfo {
        ip_address: "192.168.1.100".to_string(),
        user_agent: Some("Gemini-Proxy-Admin/1.0".to_string()),
        device_type: Some("desktop".to_string()),
        location: Some("Beijing, China".to_string()),
    };
    
    // 创建用户会话
    let session = session_store.create_session(
        "admin_user",
        client_info,
        vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        None,
    ).await?;
    println!("✓ 已创建用户会话: {} (用户: {})", session.session_id, session.user_id);
    
    // 设置会话数据
    session_store.set_session_data(&session.session_id, "last_action", "view_dashboard").await?;
    session_store.set_session_data(&session.session_id, "preferences", "dark_mode").await?;
    println!("✓ 已设置会话数据");
    
    // 获取会话数据
    if let Some(action) = session_store.get_session_data(&session.session_id, "last_action").await? {
        println!("✓ 用户最后操作: {}", action);
    }
    
    // 更新会话活动
    session_store.update_session_activity(&session.session_id).await?;
    println!("✓ 已更新会话活动时间");
    
    // 查询用户会话
    let user_sessions = session_store.get_user_sessions("admin_user").await?;
    println!("✓ 用户有 {} 个活跃会话", user_sessions.len());
    
    // 刷新会话
    let refreshed_session = session_store.refresh_session(&session.session_id, None).await?;
    println!("✓ 会话已刷新，新的过期时间: {}", refreshed_session.expires_at);
    
    // 获取会话统计
    let stats = session_store.get_statistics().await?;
    println!("✓ 会话统计: {} 个总会话，{} 个活跃会话", 
        stats.total_sessions, stats.active_sessions);
    println!("  平均会话持续时间: {:.1} 分钟", stats.session_duration_stats.average_duration_minutes);
    
    // 清理过期会话
    let cleaned = session_store.cleanup_expired_sessions().await?;
    println!("✓ 已清理 {} 个过期会话", cleaned);
    
    Ok(())
}
```

## 功能说明

这个示例展示了以下持久化功能：

1. **存储管理器**: 初始化和管理数据存储
2. **权重预设**: 保存、查询和管理负载均衡权重配置
3. **配置历史**: 记录配置变更和版本控制
4. **会话管理**: 用户会话状态的持久化存储

## 使用指南

要在你的项目中使用这些功能，请参考 [persistence_integration.md](./persistence_integration.md) 获取详细的集成指南。