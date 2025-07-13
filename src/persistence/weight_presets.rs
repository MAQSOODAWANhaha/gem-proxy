// src/persistence/weight_presets.rs
//! 权重预设持久化存储
//! 
//! 提供权重预设的持久化功能，包括预设的创建、更新、删除和查询

use super::{DataStore, FileSystemStore, PersistenceConfig, PersistenceError};
use crate::load_balancer::tools::WeightPreset;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 权重预设存储管理器
pub struct WeightPresetStore {
    /// 文件存储
    file_store: FileSystemStore<WeightPreset>,
    /// 内存缓存
    cache: Arc<RwLock<HashMap<String, WeightPreset>>>,
    /// 是否启用缓存
    enable_cache: bool,
}

/// 预设查询条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetQuery {
    pub tags: Option<Vec<String>>,
    pub created_by: Option<String>,
    pub created_after: Option<u64>,
    pub created_before: Option<u64>,
    pub name_contains: Option<String>,
}

/// 预设统计信息
#[derive(Debug, Serialize, Deserialize)]
pub struct PresetStatistics {
    pub total_presets: usize,
    pub presets_by_creator: HashMap<String, usize>,
    pub most_used_tags: Vec<(String, usize)>,
    pub average_keys_per_preset: f64,
    pub creation_timeline: Vec<(String, u64)>, // (date, count)
}

impl WeightPresetStore {
    /// 创建新的权重预设存储
    pub fn new(config: PersistenceConfig, enable_cache: bool) -> Self {
        let file_store = FileSystemStore::new(config, "weight_presets".to_string());
        
        Self {
            file_store,
            cache: Arc::new(RwLock::new(HashMap::new())),
            enable_cache,
        }
    }
    
    /// 初始化存储（加载所有预设到缓存）
    pub async fn initialize(&self) -> Result<(), PersistenceError> {
        if !self.enable_cache {
            return Ok(());
        }
        
        let keys = self.file_store.list_keys().await?;
        let mut cache = self.cache.write().await;
        
        for key in keys {
            match self.file_store.load(&key).await {
                Ok(preset) => {
                    cache.insert(key, preset);
                }
                Err(e) => {
                    tracing::warn!("加载预设 {} 失败: {}", key, e);
                }
            }
        }
        
        tracing::info!("已加载 {} 个权重预设到缓存", cache.len());
        Ok(())
    }
    
    /// 保存权重预设
    pub async fn save_preset(&self, preset: &WeightPreset) -> Result<(), PersistenceError> {
        // 保存到文件
        self.file_store.save(&preset.id, preset).await?;
        
        // 更新缓存
        if self.enable_cache {
            self.cache.write().await.insert(preset.id.clone(), preset.clone());
        }
        
        tracing::info!("已保存权重预设: {}", preset.name);
        Ok(())
    }
    
    /// 加载权重预设
    pub async fn load_preset(&self, preset_id: &str) -> Result<WeightPreset, PersistenceError> {
        // 首先尝试从缓存加载
        if self.enable_cache {
            let cache = self.cache.read().await;
            if let Some(preset) = cache.get(preset_id) {
                return Ok(preset.clone());
            }
        }
        
        // 从文件加载
        let preset = self.file_store.load(preset_id).await?;
        
        // 更新缓存
        if self.enable_cache {
            self.cache.write().await.insert(preset_id.to_string(), preset.clone());
        }
        
        Ok(preset)
    }
    
    /// 删除权重预设
    pub async fn delete_preset(&self, preset_id: &str) -> Result<(), PersistenceError> {
        // 从文件删除
        self.file_store.delete(preset_id).await?;
        
        // 从缓存删除
        if self.enable_cache {
            self.cache.write().await.remove(preset_id);
        }
        
        tracing::info!("已删除权重预设: {}", preset_id);
        Ok(())
    }
    
    /// 检查预设是否存在
    pub async fn exists(&self, preset_id: &str) -> Result<bool, PersistenceError> {
        if self.enable_cache {
            let cache = self.cache.read().await;
            if cache.contains_key(preset_id) {
                return Ok(true);
            }
        }
        
        self.file_store.exists(preset_id).await
    }
    
    /// 列出所有预设ID
    pub async fn list_preset_ids(&self) -> Result<Vec<String>, PersistenceError> {
        if self.enable_cache {
            let cache = self.cache.read().await;
            return Ok(cache.keys().cloned().collect());
        }
        
        self.file_store.list_keys().await
    }
    
    /// 列出所有预设
    pub async fn list_all_presets(&self) -> Result<Vec<WeightPreset>, PersistenceError> {
        if self.enable_cache {
            let cache = self.cache.read().await;
            return Ok(cache.values().cloned().collect());
        }
        
        let keys = self.file_store.list_keys().await?;
        let mut presets = Vec::new();
        
        for key in keys {
            match self.file_store.load(&key).await {
                Ok(preset) => presets.push(preset),
                Err(e) => {
                    tracing::warn!("加载预设 {} 失败: {}", key, e);
                }
            }
        }
        
        Ok(presets)
    }
    
    /// 按条件查询预设
    pub async fn query_presets(&self, query: &PresetQuery) -> Result<Vec<WeightPreset>, PersistenceError> {
        let all_presets = self.list_all_presets().await?;
        
        let filtered_presets = all_presets
            .into_iter()
            .filter(|preset| self.matches_query(preset, query))
            .collect();
        
        Ok(filtered_presets)
    }
    
    /// 检查预设是否匹配查询条件
    fn matches_query(&self, preset: &WeightPreset, query: &PresetQuery) -> bool {
        // 检查标签
        if let Some(ref tags) = query.tags {
            if !tags.iter().any(|tag| preset.tags.contains(tag)) {
                return false;
            }
        }
        
        // 检查创建者
        if let Some(ref created_by) = query.created_by {
            if preset.created_by != *created_by {
                return false;
            }
        }
        
        // 检查创建时间范围
        if let Some(created_after) = query.created_after {
            if preset.created_at < created_after {
                return false;
            }
        }
        
        if let Some(created_before) = query.created_before {
            if preset.created_at > created_before {
                return false;
            }
        }
        
        // 检查名称包含
        if let Some(ref name_contains) = query.name_contains {
            if !preset.name.contains(name_contains) {
                return false;
            }
        }
        
        true
    }
    
    /// 更新预设
    pub async fn update_preset(&self, preset_id: &str, updated_preset: &WeightPreset) -> Result<(), PersistenceError> {
        // 确保ID一致
        if preset_id != updated_preset.id {
            return Err(PersistenceError::InvalidFormat(
                "预设ID不匹配".to_string()
            ));
        }
        
        // 检查预设是否存在
        if !self.exists(preset_id).await? {
            return Err(PersistenceError::DataNotFound(preset_id.to_string()));
        }
        
        self.save_preset(updated_preset).await
    }
    
    /// 复制预设
    pub async fn duplicate_preset(
        &self, 
        source_id: &str, 
        new_name: &str,
        created_by: &str
    ) -> Result<WeightPreset, PersistenceError> {
        let source_preset = self.load_preset(source_id).await?;
        
        let new_preset = WeightPreset {
            id: uuid::Uuid::new_v4().to_string(),
            name: new_name.to_string(),
            description: format!("复制自: {}", source_preset.name),
            weights: source_preset.weights,
            created_by: created_by.to_string(),
            created_at: chrono::Utc::now().timestamp() as u64,
            tags: source_preset.tags,
        };
        
        self.save_preset(&new_preset).await?;
        Ok(new_preset)
    }
    
    /// 获取预设统计信息
    pub async fn get_statistics(&self) -> Result<PresetStatistics, PersistenceError> {
        let presets = self.list_all_presets().await?;
        let total_presets = presets.len();
        
        // 按创建者统计
        let mut presets_by_creator = HashMap::new();
        for preset in &presets {
            *presets_by_creator.entry(preset.created_by.clone()).or_insert(0) += 1;
        }
        
        // 统计标签使用频率
        let mut tag_counts = HashMap::new();
        for preset in &presets {
            for tag in &preset.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        
        let mut most_used_tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
        most_used_tags.sort_by(|a, b| b.1.cmp(&a.1));
        most_used_tags.truncate(10); // 只保留前10个
        
        // 计算平均密钥数
        let total_keys: usize = presets.iter().map(|p| p.weights.len()).sum();
        let average_keys_per_preset = if total_presets > 0 {
            total_keys as f64 / total_presets as f64
        } else {
            0.0
        };
        
        // 创建时间线统计（按天分组）
        let mut daily_counts = HashMap::new();
        for preset in &presets {
            let date = chrono::DateTime::from_timestamp(preset.created_at as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            *daily_counts.entry(date).or_insert(0) += 1;
        }
        
        let mut creation_timeline: Vec<(String, u64)> = daily_counts
            .into_iter()
            .map(|(date, count)| (date, count as u64))
            .collect();
        creation_timeline.sort_by(|a, b| a.0.cmp(&b.0));
        
        Ok(PresetStatistics {
            total_presets,
            presets_by_creator,
            most_used_tags,
            average_keys_per_preset,
            creation_timeline,
        })
    }
    
    /// 导出预设
    pub async fn export_presets(&self, preset_ids: Option<Vec<String>>) -> Result<String, PersistenceError> {
        let presets = if let Some(ids) = preset_ids {
            let mut selected_presets = Vec::new();
            for id in ids {
                match self.load_preset(&id).await {
                    Ok(preset) => selected_presets.push(preset),
                    Err(e) => {
                        tracing::warn!("导出时加载预设 {} 失败: {}", id, e);
                    }
                }
            }
            selected_presets
        } else {
            self.list_all_presets().await?
        };
        
        let export_data = serde_json::to_string_pretty(&presets)?;
        Ok(export_data)
    }
    
    /// 导入预设
    pub async fn import_presets(&self, json_data: &str, overwrite: bool) -> Result<ImportResult, PersistenceError> {
        let presets: Vec<WeightPreset> = serde_json::from_str(json_data)?;
        let mut result = ImportResult::default();
        
        for preset in presets {
            match self.exists(&preset.id).await {
                Ok(exists) => {
                    if exists && !overwrite {
                        result.skipped.push(preset.id);
                        continue;
                    }
                    
                    match self.save_preset(&preset).await {
                        Ok(()) => {
                            if exists {
                                result.updated.push(preset.id);
                            } else {
                                result.created.push(preset.id);
                            }
                        }
                        Err(e) => {
                            result.failed.push((preset.id, e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    result.failed.push((preset.id, e.to_string()));
                }
            }
        }
        
        Ok(result)
    }
    
    /// 清理缓存
    pub async fn clear_cache(&self) {
        if self.enable_cache {
            self.cache.write().await.clear();
            tracing::info!("已清理权重预设缓存");
        }
    }
    
    /// 重新加载缓存
    pub async fn reload_cache(&self) -> Result<(), PersistenceError> {
        if self.enable_cache {
            self.clear_cache().await;
            self.initialize().await?;
        }
        Ok(())
    }
}

/// 导入结果
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ImportResult {
    pub created: Vec<String>,
    pub updated: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<(String, String)>, // (preset_id, error_message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::collections::HashMap;
    
    fn create_test_preset(id: &str, name: &str, created_by: &str) -> WeightPreset {
        let mut weights = HashMap::new();
        weights.insert("key1".to_string(), 100);
        weights.insert("key2".to_string(), 200);
        
        WeightPreset {
            id: id.to_string(),
            name: name.to_string(),
            description: "测试预设".to_string(),
            weights,
            created_by: created_by.to_string(),
            created_at: chrono::Utc::now().timestamp() as u64,
            tags: vec!["test".to_string(), "demo".to_string()],
        }
    }
    
    #[tokio::test]
    async fn test_preset_store_operations() {
        let temp_dir = tempdir().unwrap();
        let config = PersistenceConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let store = WeightPresetStore::new(config, true);
        store.initialize().await.unwrap();
        
        let preset = create_test_preset("test1", "测试预设1", "admin");
        
        // 测试保存
        store.save_preset(&preset).await.unwrap();
        
        // 测试加载
        let loaded = store.load_preset("test1").await.unwrap();
        assert_eq!(preset.name, loaded.name);
        
        // 测试存在性检查
        assert!(store.exists("test1").await.unwrap());
        assert!(!store.exists("non_existent").await.unwrap());
        
        // 测试列出所有预设
        let all_presets = store.list_all_presets().await.unwrap();
        assert_eq!(all_presets.len(), 1);
        
        // 测试查询
        let query = PresetQuery {
            created_by: Some("admin".to_string()),
            ..Default::default()
        };
        let results = store.query_presets(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        
        // 测试复制
        let duplicated = store.duplicate_preset("test1", "复制的预设", "user1").await.unwrap();
        assert_eq!(duplicated.name, "复制的预设");
        assert_eq!(duplicated.created_by, "user1");
        
        // 测试统计
        let stats = store.get_statistics().await.unwrap();
        assert_eq!(stats.total_presets, 2);
        assert!(stats.presets_by_creator.contains_key("admin"));
        
        // 测试删除
        store.delete_preset("test1").await.unwrap();
        assert!(!store.exists("test1").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_preset_import_export() {
        let temp_dir = tempdir().unwrap();
        let config = PersistenceConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let store = WeightPresetStore::new(config, true);
        store.initialize().await.unwrap();
        
        let preset1 = create_test_preset("test1", "预设1", "admin");
        let preset2 = create_test_preset("test2", "预设2", "user1");
        
        store.save_preset(&preset1).await.unwrap();
        store.save_preset(&preset2).await.unwrap();
        
        // 测试导出
        let exported = store.export_presets(None).await.unwrap();
        assert!(exported.contains("预设1"));
        assert!(exported.contains("预设2"));
        
        // 清除存储
        store.delete_preset("test1").await.unwrap();
        store.delete_preset("test2").await.unwrap();
        
        // 测试导入
        let result = store.import_presets(&exported, false).await.unwrap();
        assert_eq!(result.created.len(), 2);
        assert_eq!(result.failed.len(), 0);
        
        // 验证导入结果
        assert!(store.exists("test1").await.unwrap());
        assert!(store.exists("test2").await.unwrap());
    }
}

impl Default for PresetQuery {
    fn default() -> Self {
        Self {
            tags: None,
            created_by: None,
            created_after: None,
            created_before: None,
            name_contains: None,
        }
    }
}