// src/persistence/config_history.rs
//! 配置变更历史记录
//! 
//! 记录和管理配置文件的历史变更，支持版本控制、回滚和变更审计

use super::{DataStore, FileSystemStore, PersistenceConfig, PersistenceError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 配置变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeRecord {
    /// 记录ID
    pub id: String,
    /// 版本号
    pub version: u32,
    /// 变更时间戳
    pub timestamp: u64,
    /// 操作者
    pub operator: String,
    /// 变更类型
    pub change_type: ConfigChangeType,
    /// 变更描述
    pub description: String,
    /// 变更前的配置（JSON格式）
    pub previous_config: Option<String>,
    /// 变更后的配置（JSON格式）
    pub new_config: String,
    /// 变更的具体字段
    pub changed_fields: Vec<String>,
    /// 变更来源
    pub source: ChangeSource,
    /// 附加元数据
    pub metadata: HashMap<String, String>,
}

/// 配置变更类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigChangeType {
    /// 初始创建
    Create,
    /// 字段更新
    Update,
    /// 字段删除
    Delete,
    /// 批量变更
    BatchUpdate,
    /// 从备份恢复
    Restore,
    /// 自动同步
    AutoSync,
}

/// 变更来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeSource {
    /// Web管理界面
    WebUI,
    /// API调用
    API,
    /// 配置文件直接编辑
    FileEdit,
    /// 自动化脚本
    Script,
    /// 系统自动变更
    System,
}

/// 配置快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// 快照ID
    pub id: String,
    /// 版本号
    pub version: u32,
    /// 创建时间
    pub timestamp: u64,
    /// 创建者
    pub created_by: String,
    /// 快照描述
    pub description: String,
    /// 配置内容（JSON格式）
    pub config_content: String,
    /// 是否为自动快照
    pub is_auto_snapshot: bool,
    /// 相关变更记录ID
    pub related_change_id: Option<String>,
}

/// 配置历史查询条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigHistoryQuery {
    pub operator: Option<String>,
    pub change_type: Option<ConfigChangeType>,
    pub source: Option<ChangeSource>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub changed_field: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 配置历史统计
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigHistoryStats {
    pub total_changes: usize,
    pub changes_by_type: HashMap<String, usize>,
    pub changes_by_source: HashMap<String, usize>,
    pub changes_by_operator: HashMap<String, usize>,
    pub most_changed_fields: Vec<(String, usize)>,
    pub change_frequency: Vec<TimeSeriesPoint>,
    pub average_changes_per_day: f64,
}

/// 时间序列数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: u64,
    pub value: f64,
    pub label: String,
}

/// 配置历史管理器
pub struct ConfigHistoryStore {
    /// 变更记录存储
    changes_store: FileSystemStore<ConfigChangeRecord>,
    /// 快照存储
    snapshots_store: FileSystemStore<ConfigSnapshot>,
    /// 内存索引（用于快速查询）
    change_index: Arc<RwLock<HashMap<String, Vec<String>>>>, // field_name -> change_ids
    /// 版本计数器
    version_counter: Arc<RwLock<u32>>,
    /// 配置
    config: ConfigHistoryConfig,
}

/// 配置历史管理配置
#[derive(Debug, Clone)]
pub struct ConfigHistoryConfig {
    /// 历史记录保留天数
    pub retention_days: u32,
    /// 最大记录数
    pub max_records: usize,
    /// 自动快照间隔（秒）
    pub auto_snapshot_interval: u64,
    /// 启用压缩
    pub enable_compression: bool,
}

impl Default for ConfigHistoryConfig {
    fn default() -> Self {
        Self {
            retention_days: 90,
            max_records: 10000,
            auto_snapshot_interval: 3600, // 1小时
            enable_compression: false,
        }
    }
}

impl ConfigHistoryStore {
    /// 创建新的配置历史存储
    pub fn new(persistence_config: PersistenceConfig, history_config: ConfigHistoryConfig) -> Self {
        let changes_store = FileSystemStore::new(persistence_config.clone(), "config_changes".to_string());
        let snapshots_store = FileSystemStore::new(persistence_config, "config_snapshots".to_string());
        
        Self {
            changes_store,
            snapshots_store,
            change_index: Arc::new(RwLock::new(HashMap::new())),
            version_counter: Arc::new(RwLock::new(0)),
            config: history_config,
        }
    }
    
    /// 初始化存储
    pub async fn initialize(&self) -> Result<(), PersistenceError> {
        // 重建索引
        self.rebuild_index().await?;
        
        // 获取最新版本号
        let latest_version = self.get_latest_version().await?;
        *self.version_counter.write().await = latest_version;
        
        tracing::info!("配置历史存储初始化完成，当前版本: {}", latest_version);
        Ok(())
    }
    
    /// 记录配置变更
    pub async fn record_change(
        &self,
        operator: &str,
        change_type: ConfigChangeType,
        description: &str,
        previous_config: Option<&str>,
        new_config: &str,
        changed_fields: Vec<String>,
        source: ChangeSource,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<String, PersistenceError> {
        let record_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp() as u64;
        
        // 获取新版本号
        let version = {
            let mut counter = self.version_counter.write().await;
            *counter += 1;
            *counter
        };
        
        let record = ConfigChangeRecord {
            id: record_id.clone(),
            version,
            timestamp,
            operator: operator.to_string(),
            change_type,
            description: description.to_string(),
            previous_config: previous_config.map(|s| s.to_string()),
            new_config: new_config.to_string(),
            changed_fields: changed_fields.clone(),
            source,
            metadata: metadata.unwrap_or_default(),
        };
        
        // 保存记录
        self.changes_store.save(&record_id, &record).await?;
        
        // 更新索引
        self.update_index(&changed_fields, &record_id).await;
        
        // 清理旧记录
        self.cleanup_old_records().await.ok();
        
        tracing::info!("已记录配置变更: {} (版本: {})", description, version);
        Ok(record_id)
    }
    
    /// 创建配置快照
    pub async fn create_snapshot(
        &self,
        created_by: &str,
        description: &str,
        config_content: &str,
        is_auto_snapshot: bool,
        related_change_id: Option<String>,
    ) -> Result<String, PersistenceError> {
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp() as u64;
        let version = *self.version_counter.read().await;
        
        let snapshot = ConfigSnapshot {
            id: snapshot_id.clone(),
            version,
            timestamp,
            created_by: created_by.to_string(),
            description: description.to_string(),
            config_content: config_content.to_string(),
            is_auto_snapshot,
            related_change_id,
        };
        
        self.snapshots_store.save(&snapshot_id, &snapshot).await?;
        
        tracing::info!("已创建配置快照: {}", description);
        Ok(snapshot_id)
    }
    
    /// 查询配置变更历史
    pub async fn query_changes(&self, query: &ConfigHistoryQuery) -> Result<Vec<ConfigChangeRecord>, PersistenceError> {
        let all_change_ids = self.changes_store.list_keys().await?;
        let mut matching_records = Vec::new();
        
        for change_id in all_change_ids {
            match self.changes_store.load(&change_id).await {
                Ok(record) => {
                    if self.matches_change_query(&record, query) {
                        matching_records.push(record);
                    }
                }
                Err(e) => {
                    tracing::warn!("加载变更记录 {} 失败: {}", change_id, e);
                }
            }
        }
        
        // 按时间戳倒序排序
        matching_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // 分页处理
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(matching_records.len());
        
        let results = matching_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();
        
        Ok(results)
    }
    
    /// 获取指定版本的配置
    pub async fn get_config_by_version(&self, version: u32) -> Result<Option<String>, PersistenceError> {
        let query = ConfigHistoryQuery {
            limit: Some(1),
            ..Default::default()
        };
        
        let changes = self.query_changes(&query).await?;
        
        for change in changes {
            if change.version == version {
                return Ok(Some(change.new_config));
            }
        }
        
        Ok(None)
    }
    
    /// 获取最新配置
    pub async fn get_latest_config(&self) -> Result<Option<String>, PersistenceError> {
        let latest_version = *self.version_counter.read().await;
        self.get_config_by_version(latest_version).await
    }
    
    /// 获取配置快照
    pub async fn get_snapshot(&self, snapshot_id: &str) -> Result<ConfigSnapshot, PersistenceError> {
        self.snapshots_store.load(snapshot_id).await
    }
    
    /// 列出所有快照
    pub async fn list_snapshots(&self, limit: Option<usize>) -> Result<Vec<ConfigSnapshot>, PersistenceError> {
        let snapshot_ids = self.snapshots_store.list_keys().await?;
        let mut snapshots = Vec::new();
        
        for id in snapshot_ids {
            match self.snapshots_store.load(&id).await {
                Ok(snapshot) => snapshots.push(snapshot),
                Err(e) => {
                    tracing::warn!("加载快照 {} 失败: {}", id, e);
                }
            }
        }
        
        // 按时间戳倒序排序
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if let Some(limit) = limit {
            snapshots.truncate(limit);
        }
        
        Ok(snapshots)
    }
    
    /// 回滚到指定版本
    pub async fn rollback_to_version(
        &self,
        target_version: u32,
        operator: &str,
        reason: &str,
    ) -> Result<String, PersistenceError> {
        let target_config = self.get_config_by_version(target_version).await?
            .ok_or_else(|| PersistenceError::DataNotFound(format!("版本 {}", target_version)))?;
        
        let current_config = self.get_latest_config().await?
            .unwrap_or_default();
        
        // 记录回滚操作
        let mut metadata = HashMap::new();
        metadata.insert("target_version".to_string(), target_version.to_string());
        metadata.insert("rollback_reason".to_string(), reason.to_string());
        
        let change_id = self.record_change(
            operator,
            ConfigChangeType::Restore,
            &format!("回滚到版本 {}: {}", target_version, reason),
            Some(&current_config),
            &target_config,
            vec!["*".to_string()], // 表示全部字段
            ChangeSource::WebUI,
            Some(metadata),
        ).await?;
        
        Ok(change_id)
    }
    
    /// 获取配置历史统计
    pub async fn get_statistics(&self, days: Option<u32>) -> Result<ConfigHistoryStats, PersistenceError> {
        let cutoff_time = if let Some(days) = days {
            chrono::Utc::now().timestamp() as u64 - (days as u64 * 24 * 3600)
        } else {
            0
        };
        
        let query = ConfigHistoryQuery {
            start_time: Some(cutoff_time),
            ..Default::default()
        };
        
        let changes = self.query_changes(&query).await?;
        let total_changes = changes.len();
        
        // 按类型统计
        let mut changes_by_type = HashMap::new();
        for change in &changes {
            let type_name = format!("{:?}", change.change_type);
            *changes_by_type.entry(type_name).or_insert(0) += 1;
        }
        
        // 按来源统计
        let mut changes_by_source = HashMap::new();
        for change in &changes {
            let source_name = format!("{:?}", change.source);
            *changes_by_source.entry(source_name).or_insert(0) += 1;
        }
        
        // 按操作者统计
        let mut changes_by_operator = HashMap::new();
        for change in &changes {
            *changes_by_operator.entry(change.operator.clone()).or_insert(0) += 1;
        }
        
        // 最常变更的字段
        let mut field_counts = HashMap::new();
        for change in &changes {
            for field in &change.changed_fields {
                *field_counts.entry(field.clone()).or_insert(0) += 1;
            }
        }
        
        let mut most_changed_fields: Vec<(String, usize)> = field_counts.into_iter().collect();
        most_changed_fields.sort_by(|a, b| b.1.cmp(&a.1));
        most_changed_fields.truncate(10);
        
        // 变更频率时间序列
        let mut daily_changes = HashMap::new();
        for change in &changes {
            let date = chrono::DateTime::from_timestamp(change.timestamp as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            *daily_changes.entry(date).or_insert(0) += 1;
        }
        
        let mut change_frequency: Vec<TimeSeriesPoint> = daily_changes
            .into_iter()
            .map(|(date, count)| TimeSeriesPoint {
                timestamp: 0, // 简化实现
                value: count as f64,
                label: date,
            })
            .collect();
        change_frequency.sort_by(|a, b| a.label.cmp(&b.label));
        
        // 计算平均每日变更次数
        let days_span = if days.is_some() { days.unwrap() as f64 } else { 30.0 };
        let average_changes_per_day = total_changes as f64 / days_span;
        
        Ok(ConfigHistoryStats {
            total_changes,
            changes_by_type,
            changes_by_source,
            changes_by_operator,
            most_changed_fields,
            change_frequency,
            average_changes_per_day,
        })
    }
    
    /// 导出配置历史
    pub async fn export_history(&self, query: &ConfigHistoryQuery) -> Result<String, PersistenceError> {
        let changes = self.query_changes(query).await?;
        let export_data = serde_json::to_string_pretty(&changes)?;
        Ok(export_data)
    }
    
    // 私有辅助方法
    
    /// 检查变更记录是否匹配查询条件
    fn matches_change_query(&self, record: &ConfigChangeRecord, query: &ConfigHistoryQuery) -> bool {
        if let Some(ref operator) = query.operator {
            if !record.operator.contains(operator) {
                return false;
            }
        }
        
        if let Some(ref change_type) = query.change_type {
            if record.change_type != *change_type {
                return false;
            }
        }
        
        if let Some(ref source) = query.source {
            if record.source != *source {
                return false;
            }
        }
        
        if let Some(start_time) = query.start_time {
            if record.timestamp < start_time {
                return false;
            }
        }
        
        if let Some(end_time) = query.end_time {
            if record.timestamp > end_time {
                return false;
            }
        }
        
        if let Some(ref field) = query.changed_field {
            if !record.changed_fields.contains(field) {
                return false;
            }
        }
        
        true
    }
    
    /// 重建索引
    async fn rebuild_index(&self) -> Result<(), PersistenceError> {
        let change_ids = self.changes_store.list_keys().await?;
        let mut index = HashMap::new();
        
        for change_id in change_ids {
            if let Ok(record) = self.changes_store.load(&change_id).await {
                for field in &record.changed_fields {
                    index.entry(field.clone())
                        .or_insert_with(Vec::new)
                        .push(change_id.clone());
                }
            }
        }
        
        *self.change_index.write().await = index;
        Ok(())
    }
    
    /// 更新索引
    async fn update_index(&self, changed_fields: &[String], change_id: &str) {
        let mut index = self.change_index.write().await;
        for field in changed_fields {
            index.entry(field.clone())
                .or_insert_with(Vec::new)
                .push(change_id.to_string());
        }
    }
    
    /// 获取最新版本号
    async fn get_latest_version(&self) -> Result<u32, PersistenceError> {
        let change_ids = self.changes_store.list_keys().await?;
        let mut max_version = 0u32;
        
        for change_id in change_ids {
            if let Ok(record) = self.changes_store.load(&change_id).await {
                max_version = max_version.max(record.version);
            }
        }
        
        Ok(max_version)
    }
    
    /// 清理旧记录
    async fn cleanup_old_records(&self) -> Result<(), PersistenceError> {
        let cutoff_time = chrono::Utc::now().timestamp() as u64 - (self.config.retention_days as u64 * 24 * 3600);
        let change_ids = self.changes_store.list_keys().await?;
        
        let mut deletion_count = 0;
        for change_id in change_ids {
            if let Ok(record) = self.changes_store.load(&change_id).await {
                if record.timestamp < cutoff_time {
                    self.changes_store.delete(&change_id).await.ok();
                    deletion_count += 1;
                }
            }
        }
        
        if deletion_count > 0 {
            tracing::info!("已清理 {} 条过期的配置变更记录", deletion_count);
            self.rebuild_index().await?;
        }
        
        Ok(())
    }
}

impl Default for ConfigHistoryQuery {
    fn default() -> Self {
        Self {
            operator: None,
            change_type: None,
            source: None,
            start_time: None,
            end_time: None,
            changed_field: None,
            limit: None,
            offset: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_config_history_operations() {
        let temp_dir = tempdir().unwrap();
        let persistence_config = PersistenceConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let history_store = ConfigHistoryStore::new(persistence_config, ConfigHistoryConfig::default());
        history_store.initialize().await.unwrap();
        
        // 测试记录变更
        let change_id = history_store.record_change(
            "admin",
            ConfigChangeType::Update,
            "更新API密钥配置",
            Some(r#"{"api_keys": []}"#),
            r#"{"api_keys": [{"id": "key1", "weight": 100}]}"#,
            vec!["api_keys".to_string()],
            ChangeSource::WebUI,
            None,
        ).await.unwrap();
        
        assert!(!change_id.is_empty());
        
        // 测试查询变更
        let query = ConfigHistoryQuery {
            operator: Some("admin".to_string()),
            ..Default::default()
        };
        let changes = history_store.query_changes(&query).await.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].operator, "admin");
        
        // 测试创建快照
        let snapshot_id = history_store.create_snapshot(
            "admin",
            "测试快照",
            r#"{"api_keys": [{"id": "key1", "weight": 100}]}"#,
            false,
            Some(change_id),
        ).await.unwrap();
        
        // 测试获取快照
        let snapshot = history_store.get_snapshot(&snapshot_id).await.unwrap();
        assert_eq!(snapshot.description, "测试快照");
        
        // 测试统计信息
        let stats = history_store.get_statistics(Some(30)).await.unwrap();
        assert_eq!(stats.total_changes, 1);
        assert!(stats.changes_by_operator.contains_key("admin"));
    }
}