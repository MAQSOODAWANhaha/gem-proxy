# 数据持久化集成指南

本文档介绍如何在 Gemini Proxy 项目中集成和使用数据持久化功能。

## 🎯 功能概述

数据持久化模块为 Gemini Proxy 提供了统一的数据存储抽象层，支持以下核心功能：

- **权重预设管理**: 保存和管理负载均衡权重预设
- **配置历史记录**: 追踪配置变更历史和版本控制
- **会话状态持久化**: 管理用户会话和认证状态
- **通用存储接口**: 支持多种存储后端的抽象层

## 📁 模块结构

```
src/persistence/
├── mod.rs              # 主模块和通用接口
├── storage.rs          # 存储抽象层和工厂
├── weight_presets.rs   # 权重预设存储
├── config_history.rs   # 配置历史管理
└── session_store.rs    # 会话状态存储
```

## 🔧 基本配置

### 1. 持久化配置

```rust
use gemini_proxy::persistence::PersistenceConfig;

let config = PersistenceConfig {
    data_dir: PathBuf::from("./data"),
    enable_compression: false,
    backup_retention_days: 30,
    auto_backup_interval: 3600, // 1小时
    max_file_size: 10 * 1024 * 1024, // 10MB
};
```

### 2. 存储管理器初始化

```rust
use gemini_proxy::persistence::StorageManager;

let storage_manager = StorageManager::new(config);
storage_manager.initialize().await?;
```

## 🎛️ 权重预设管理

### 创建和管理预设

```rust
use gemini_proxy::persistence::weight_presets::*;

// 创建存储实例
let preset_store = WeightPresetStore::new(config, true);
preset_store.initialize().await?;

// 创建权重预设
let mut weights = HashMap::new();
weights.insert("gemini-pro".to_string(), 300);
weights.insert("gemini-flash".to_string(), 700);

let preset = WeightPreset {
    id: uuid::Uuid::new_v4().to_string(),
    name: "生产环境配置".to_string(),
    description: "优化的生产环境权重分配".to_string(),
    weights,
    created_by: "admin".to_string(),
    created_at: chrono::Utc::now().timestamp() as u64,
    tags: vec!["production".to_string()],
};

// 保存预设
preset_store.save_preset(&preset).await?;
```

### 查询和应用预设

```rust
// 按标签查询
let query = PresetQuery {
    tags: Some(vec!["production".to_string()]),
    ..Default::default()
};
let presets = preset_store.query_presets(&query).await?;

// 应用预设到配置
for preset in presets {
    println!("预设: {} - {}", preset.name, preset.description);
    // 应用权重配置...
}
```

## 📜 配置历史记录

### 记录配置变更

```rust
use gemini_proxy::persistence::config_history::*;

let history_store = ConfigHistoryStore::new(
    config,
    ConfigHistoryConfig::default()
);
history_store.initialize().await?;

// 记录配置变更
let change_id = history_store.record_change(
    "admin",                           // 操作者
    ConfigChangeType::Update,          // 变更类型
    "更新API密钥权重",                 // 描述
    Some(old_config_json),             // 旧配置
    new_config_json,                   // 新配置
    vec!["api_keys".to_string()],      // 变更字段
    ChangeSource::WebUI,               // 变更来源
    None,                              // 元数据
).await?;
```

### 配置版本管理

```rust
// 创建配置快照
let snapshot_id = history_store.create_snapshot(
    "admin",
    "重要变更后快照",
    config_json,
    false,                 // 非自动快照
    Some(change_id),       // 关联的变更ID
).await?;

// 回滚到指定版本
let rollback_id = history_store.rollback_to_version(
    target_version,
    "admin",
    "回滚到稳定版本"
).await?;
```

## 🔐 会话状态管理

### 创建和管理会话

```rust
use gemini_proxy::persistence::session_store::*;

let session_store = SessionStore::new(
    config,
    SessionStoreConfig::default()
);
session_store.initialize().await?;

// 创建用户会话
let client_info = ClientInfo {
    ip_address: "192.168.1.100".to_string(),
    user_agent: Some("Gemini-Admin/1.0".to_string()),
    device_type: Some("desktop".to_string()),
    location: None,
};

let session = session_store.create_session(
    "admin_user",
    client_info,
    vec!["read".to_string(), "write".to_string()],
    None, // 使用默认超时
).await?;
```

### 会话数据操作

```rust
// 设置会话数据
session_store.set_session_data(
    &session.session_id,
    "preferences",
    r#"{"theme": "dark", "language": "zh"}"#
).await?;

// 获取会话数据
if let Some(prefs) = session_store.get_session_data(
    &session.session_id,
    "preferences"
).await? {
    println!("用户偏好: {}", prefs);
}

// 刷新会话
let refreshed = session_store.refresh_session(
    &session.session_id,
    Some(Duration::hours(2)) // 延长2小时
).await?;
```

## 🔧 高级功能

### 存储统计和监控

```rust
// 获取存储统计
let storage_stats = storage_manager.get_storage_stats().await?;
println!("存储使用: {} 文件, {} 字节", 
    storage_stats.file_count, 
    storage_stats.total_size
);

// 权重预设统计
let preset_stats = preset_store.get_statistics().await?;
println!("预设统计: {} 个，平均 {:.1} 个密钥/预设", 
    preset_stats.total_presets,
    preset_stats.average_keys_per_preset
);

// 会话统计
let session_stats = session_store.get_statistics().await?;
println!("会话: {} 总计, {} 活跃, 平均时长 {:.1} 分钟",
    session_stats.total_sessions,
    session_stats.active_sessions,
    session_stats.session_duration_stats.average_duration_minutes
);
```

### 数据导出和备份

```rust
// 导出权重预设
let exported_presets = preset_store.export_presets(None).await?;
std::fs::write("backup/presets.json", exported_presets)?;

// 导出配置历史
let history_query = ConfigHistoryQuery {
    start_time: Some(start_timestamp),
    end_time: Some(end_timestamp),
    ..Default::default()
};
let exported_history = history_store.export_history(&history_query).await?;
std::fs::write("backup/config_history.json", exported_history)?;
```

### 数据清理和维护

```rust
// 清理过期会话
let cleaned_sessions = session_store.cleanup_expired_sessions().await?;
println!("清理了 {} 个过期会话", cleaned_sessions);

// 清理权重预设缓存
preset_store.clear_cache().await;
preset_store.reload_cache().await?;
```

## 🔌 与现有系统集成

### 1. 在权重管理中集成

```rust
// 在 WeightManagementToolkit 中集成预设存储
use gemini_proxy::persistence::weight_presets::WeightPresetStore;

impl WeightManagementToolkit {
    pub fn with_persistence(
        audit_system: Arc<RwLock<WeightAuditSystem>>,
        config: ToolkitConfig,
        preset_store: Arc<WeightPresetStore>,
    ) -> Self {
        // 集成持久化功能...
    }
    
    pub async fn save_current_as_preset(
        &self,
        name: &str,
        description: &str,
        current_weights: &HashMap<String, u32>,
    ) -> Result<String, String> {
        // 将当前权重保存为预设...
    }
}
```

### 2. 在认证系统中集成

```rust
// 在 AuthState 中集成会话存储
use gemini_proxy::persistence::session_store::SessionStore;

impl AuthState {
    pub fn with_persistence(
        config: Arc<ProxyConfig>,
        session_store: Arc<SessionStore>,
    ) -> Self {
        // 集成会话持久化...
    }
    
    pub async fn restore_session(&self, session_id: &str) -> Option<UserSession> {
        // 从持久化存储恢复会话...
    }
}
```

### 3. 在配置管理中集成

```rust
// 在配置更新时记录历史
impl ConfigState {
    pub async fn update_config_with_history(
        &self,
        new_config: ProxyConfig,
        operator: &str,
        reason: &str,
    ) -> Result<(), String> {
        let old_config_json = serde_json::to_string(&self.current_config)?;
        let new_config_json = serde_json::to_string(&new_config)?;
        
        // 记录配置变更
        self.history_store.record_change(
            operator,
            ConfigChangeType::Update,
            reason,
            Some(&old_config_json),
            &new_config_json,
            vec!["*".to_string()],
            ChangeSource::API,
            None,
        ).await?;
        
        // 应用配置...
        self.current_config = new_config;
        Ok(())
    }
}
```

## 📊 性能考虑

### 1. 缓存策略

- **权重预设**: 启用内存缓存以提高查询性能
- **会话状态**: 活跃会话保存在内存中，定期同步到磁盘
- **配置历史**: 使用索引加速字段查询

### 2. 存储优化

```rust
// 配置存储参数
let config = PersistenceConfig {
    enable_compression: true,     // 启用压缩节省空间
    max_file_size: 50 * 1024 * 1024, // 限制单文件大小
    auto_backup_interval: 1800,   // 30分钟备份间隔
    ..Default::default()
};

// 定期清理
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        // 执行清理任务
        session_store.cleanup_expired_sessions().await.ok();
        preset_store.clear_cache().await;
    }
});
```

## 🛡️ 安全考虑

### 1. 数据加密

```rust
// 对敏感数据加密存储
use argon2::{Argon2, PasswordHasher};

impl SessionStore {
    async fn encrypt_session_data(&self, data: &str) -> Result<String, Error> {
        // 实现数据加密逻辑
    }
}
```

### 2. 访问控制

```rust
// 基于权限的数据访问
impl WeightPresetStore {
    pub async fn save_preset_with_permission(
        &self,
        preset: &WeightPreset,
        user_permissions: &[String],
    ) -> Result<(), PersistenceError> {
        if !user_permissions.contains(&"preset_write".to_string()) {
            return Err(PersistenceError::PermissionError(
                "无权限保存预设".to_string()
            ));
        }
        self.save_preset(preset).await
    }
}
```

## 🔍 故障排除

### 常见问题

1. **存储初始化失败**
   - 检查数据目录权限
   - 确认磁盘空间充足

2. **会话恢复失败**
   - 验证会话ID格式
   - 检查会话是否过期

3. **预设保存失败**
   - 确认预设ID唯一性
   - 检查权重数据格式

### 调试技巧

```rust
// 启用详细日志
use tracing::{debug, info, warn, error};

// 在关键操作处添加日志
debug!("正在保存预设: {}", preset.name);
info!("预设保存成功: {}", preset_id);
warn!("会话即将过期: {}", session_id);
error!("配置历史记录失败: {}", error);
```

## 📚 相关文档

- [API 文档](../api_reference.md)
- [配置指南](../configuration.md)
- [部署指南](../deployment.md)
- [性能优化](../performance.md)

## 🤝 贡献指南

欢迎提交问题和改进建议！请参考：

1. [贡献指南](../CONTRIBUTING.md)
2. [代码规范](../code_style.md)
3. [测试指南](../testing.md)