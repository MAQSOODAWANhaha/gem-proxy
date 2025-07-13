// src/persistence/mod.rs
//! 数据持久化模块
//! 
//! 提供统一的数据持久化接口，支持权重预设、配置历史、会话状态等数据的存储和检索

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub mod storage;
pub mod weight_presets;
pub mod config_history;
pub mod session_store;

/// 持久化错误类型
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("数据不存在: {0}")]
    DataNotFound(String),
    
    #[error("数据格式错误: {0}")]
    InvalidFormat(String),
    
    #[error("权限错误: {0}")]
    PermissionError(String),
}

/// 持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// 数据存储根目录
    pub data_dir: PathBuf,
    /// 是否启用数据压缩
    pub enable_compression: bool,
    /// 备份保留天数
    pub backup_retention_days: u32,
    /// 自动备份间隔（秒）
    pub auto_backup_interval: u64,
    /// 最大文件大小（字节）
    pub max_file_size: u64,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
            enable_compression: false,
            backup_retention_days: 30,
            auto_backup_interval: 3600, // 1小时
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// 通用数据存储接口
#[async_trait::async_trait]
pub trait DataStore<T> {
    /// 保存数据
    async fn save(&self, key: &str, data: &T) -> Result<(), PersistenceError>;
    
    /// 加载数据
    async fn load(&self, key: &str) -> Result<T, PersistenceError>;
    
    /// 删除数据
    async fn delete(&self, key: &str) -> Result<(), PersistenceError>;
    
    /// 列出所有键
    async fn list_keys(&self) -> Result<Vec<String>, PersistenceError>;
    
    /// 检查数据是否存在
    async fn exists(&self, key: &str) -> Result<bool, PersistenceError>;
}

/// 文件系统存储实现
pub struct FileSystemStore<T> {
    config: PersistenceConfig,
    namespace: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> FileSystemStore<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(config: PersistenceConfig, namespace: String) -> Self {
        Self {
            config,
            namespace,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// 获取文件路径
    fn get_file_path(&self, key: &str) -> PathBuf {
        self.config.data_dir
            .join(&self.namespace)
            .join(format!("{}.json", key))
    }
    
    /// 确保目录存在
    async fn ensure_directory(&self) -> Result<(), PersistenceError> {
        let dir = self.config.data_dir.join(&self.namespace);
        if !dir.exists() {
            fs::create_dir_all(&dir).await?;
        }
        Ok(())
    }
    
    /// 创建备份
    async fn create_backup(&self, key: &str) -> Result<(), PersistenceError> {
        let original_path = self.get_file_path(key);
        if original_path.exists() {
            let backup_path = self.config.data_dir
                .join(&self.namespace)
                .join("backups")
                .join(format!("{}_{}.json", key, chrono::Utc::now().timestamp()));
            
            if let Some(parent) = backup_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            
            fs::copy(&original_path, &backup_path).await?;
        }
        Ok(())
    }
    
    /// 清理过期备份
    pub async fn cleanup_old_backups(&self) -> Result<(), PersistenceError> {
        let backup_dir = self.config.data_dir
            .join(&self.namespace)
            .join("backups");
        
        if !backup_dir.exists() {
            return Ok(());
        }
        
        let mut entries = fs::read_dir(&backup_dir).await?;
        let cutoff_time = chrono::Utc::now() - chrono::Duration::days(self.config.backup_retention_days as i64);
        
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if let Ok(modified) = metadata.modified() {
                let modified_time = chrono::DateTime::<chrono::Utc>::from(modified);
                if modified_time < cutoff_time {
                    fs::remove_file(entry.path()).await?;
                }
            }
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl<T> DataStore<T> for FileSystemStore<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
{
    async fn save(&self, key: &str, data: &T) -> Result<(), PersistenceError> {
        self.ensure_directory().await?;
        
        // 创建备份
        self.create_backup(key).await.ok(); // 忽略备份错误
        
        let file_path = self.get_file_path(key);
        let json_data = serde_json::to_string_pretty(data)?;
        
        // 检查文件大小
        if json_data.len() as u64 > self.config.max_file_size {
            return Err(PersistenceError::InvalidFormat(
                format!("文件大小超过限制: {} bytes", json_data.len())
            ));
        }
        
        // 写入临时文件，然后重命名（原子操作）
        let temp_path = file_path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(json_data.as_bytes()).await?;
        file.sync_all().await?;
        
        fs::rename(&temp_path, &file_path).await?;
        
        Ok(())
    }
    
    async fn load(&self, key: &str) -> Result<T, PersistenceError> {
        let file_path = self.get_file_path(key);
        
        if !file_path.exists() {
            return Err(PersistenceError::DataNotFound(key.to_string()));
        }
        
        let mut file = fs::File::open(&file_path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        
        let data = serde_json::from_str(&contents)?;
        Ok(data)
    }
    
    async fn delete(&self, key: &str) -> Result<(), PersistenceError> {
        let file_path = self.get_file_path(key);
        
        if file_path.exists() {
            // 创建备份
            self.create_backup(key).await.ok();
            fs::remove_file(&file_path).await?;
        }
        
        Ok(())
    }
    
    async fn list_keys(&self) -> Result<Vec<String>, PersistenceError> {
        let dir = self.config.data_dir.join(&self.namespace);
        
        if !dir.exists() {
            return Ok(vec![]);
        }
        
        let mut entries = fs::read_dir(&dir).await?;
        let mut keys = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    if let Some(key) = stem.to_str() {
                        keys.push(key.to_string());
                    }
                }
            }
        }
        
        Ok(keys)
    }
    
    async fn exists(&self, key: &str) -> Result<bool, PersistenceError> {
        let file_path = self.get_file_path(key);
        Ok(file_path.exists())
    }
}

/// 数据存储管理器
pub struct StorageManager {
    config: PersistenceConfig,
}

impl StorageManager {
    pub fn new(config: PersistenceConfig) -> Self {
        Self { config }
    }
    
    /// 创建存储实例
    pub fn create_store<T>(&self, namespace: &str) -> FileSystemStore<T>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        FileSystemStore::new(self.config.clone(), namespace.to_string())
    }
    
    /// 初始化存储目录
    pub async fn initialize(&self) -> Result<(), PersistenceError> {
        if !self.config.data_dir.exists() {
            fs::create_dir_all(&self.config.data_dir).await?;
        }
        
        // 创建基本子目录
        let subdirs = ["weight_presets", "config_history", "sessions", "metrics"];
        for subdir in &subdirs {
            let dir_path = self.config.data_dir.join(subdir);
            if !dir_path.exists() {
                fs::create_dir_all(&dir_path).await?;
            }
        }
        
        Ok(())
    }
    
    /// 获取存储统计信息
    pub async fn get_storage_stats(&self) -> Result<StorageStats, PersistenceError> {
        let mut stats = StorageStats::default();
        
        if !self.config.data_dir.exists() {
            return Ok(stats);
        }
        
        fn calculate_dir_size(dir: std::fs::ReadDir) -> std::io::Result<(u64, usize)> {
            let mut total_size = 0u64;
            let mut file_count = 0usize;
            
            for entry in dir {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_file() {
                    total_size += metadata.len();
                    file_count += 1;
                }
            }
            
            Ok((total_size, file_count))
        }
        
        // 使用同步IO来计算统计信息（简化实现）
        if let Ok(entries) = std::fs::read_dir(&self.config.data_dir) {
            if let Ok((size, count)) = calculate_dir_size(entries) {
                stats.total_size = size;
                stats.file_count = count;
            }
        }
        
        Ok(stats)
    }
}

/// 存储统计信息
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_size: u64,
    pub file_count: usize,
    pub last_cleanup: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        value: String,
        number: i32,
    }
    
    #[tokio::test]
    async fn test_file_system_store() {
        let temp_dir = tempdir().unwrap();
        let config = PersistenceConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let store: FileSystemStore<TestData> = FileSystemStore::new(config, "test".to_string());
        
        let test_data = TestData {
            value: "测试数据".to_string(),
            number: 42,
        };
        
        // 测试保存
        store.save("test_key", &test_data).await.unwrap();
        
        // 测试加载
        let loaded_data = store.load("test_key").await.unwrap();
        assert_eq!(test_data, loaded_data);
        
        // 测试存在性检查
        assert!(store.exists("test_key").await.unwrap());
        assert!(!store.exists("non_existent").await.unwrap());
        
        // 测试列出键
        let keys = store.list_keys().await.unwrap();
        assert!(keys.contains(&"test_key".to_string()));
        
        // 测试删除
        store.delete("test_key").await.unwrap();
        assert!(!store.exists("test_key").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_storage_manager() {
        let temp_dir = tempdir().unwrap();
        let config = PersistenceConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = StorageManager::new(config);
        manager.initialize().await.unwrap();
        
        // 测试目录创建
        assert!(temp_dir.path().join("weight_presets").exists());
        assert!(temp_dir.path().join("config_history").exists());
        assert!(temp_dir.path().join("sessions").exists());
        
        // 测试存储统计
        let stats = manager.get_storage_stats().await.unwrap();
        assert_eq!(stats.file_count, 0);
    }
}