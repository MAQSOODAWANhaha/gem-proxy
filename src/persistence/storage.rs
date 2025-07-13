// src/persistence/storage.rs
//! 通用存储接口和实现
//! 
//! 提供统一的存储抽象层，支持多种存储后端

use super::{PersistenceConfig, PersistenceError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 存储后端类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    /// 文件系统存储
    FileSystem,
    /// 内存存储（仅用于测试）
    Memory,
    /// Redis存储（未来扩展）
    Redis { url: String },
    /// 数据库存储（未来扩展）
    Database { connection_string: String },
}

/// 存储操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl<T> StorageResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// 批量操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub errors: Vec<(String, String)>, // (key, error_message)
}

/// 存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    pub total_keys: usize,
    pub total_size_bytes: u64,
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
    pub read_operations: u64,
    pub write_operations: u64,
    pub delete_operations: u64,
    pub error_count: u64,
    pub average_operation_time_ms: f64,
}

/// 高级存储接口
#[async_trait::async_trait]
pub trait AdvancedStorage<T>: Send + Sync {
    /// 批量保存数据
    async fn batch_save(&self, items: Vec<(String, T)>) -> Result<BatchResult, PersistenceError>;
    
    /// 批量加载数据
    async fn batch_load(&self, keys: Vec<String>) -> Result<HashMap<String, T>, PersistenceError>;
    
    /// 批量删除数据
    async fn batch_delete(&self, keys: Vec<String>) -> Result<BatchResult, PersistenceError>;
    
    /// 根据前缀查询键
    async fn list_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, PersistenceError>;
    
    /// 获取存储统计信息
    async fn get_metrics(&self) -> Result<StorageMetrics, PersistenceError>;
    
    /// 检查存储健康状态
    async fn health_check(&self) -> Result<bool, PersistenceError>;
    
    /// 清理存储（压缩、整理等）
    async fn optimize(&self) -> Result<(), PersistenceError>;
    
    /// 备份数据
    async fn backup(&self, backup_path: &str) -> Result<(), PersistenceError>;
    
    /// 恢复数据
    async fn restore(&self, backup_path: &str) -> Result<(), PersistenceError>;
}

/// 内存存储实现（用于测试和缓存）
pub struct MemoryStorage<T> {
    data: Arc<RwLock<HashMap<String, T>>>,
    metrics: Arc<RwLock<StorageMetrics>>,
}

impl<T> MemoryStorage<T>
where
    T: Clone + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(StorageMetrics {
                total_keys: 0,
                total_size_bytes: 0,
                last_accessed: None,
                read_operations: 0,
                write_operations: 0,
                delete_operations: 0,
                error_count: 0,
                average_operation_time_ms: 0.0,
            })),
        }
    }
    
    async fn update_metrics<F>(&self, operation: F) 
    where
        F: FnOnce(&mut StorageMetrics),
    {
        let mut metrics = self.metrics.write().await;
        operation(&mut *metrics);
        metrics.last_accessed = Some(chrono::Utc::now());
    }
}

#[async_trait::async_trait]
impl<T> super::DataStore<T> for MemoryStorage<T>
where
    T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    async fn save(&self, key: &str, data: &T) -> Result<(), PersistenceError> {
        let start_time = std::time::Instant::now();
        
        let mut storage = self.data.write().await;
        storage.insert(key.to_string(), data.clone());
        
        self.update_metrics(|metrics| {
            metrics.write_operations += 1;
            metrics.total_keys = storage.len();
            let elapsed = start_time.elapsed().as_millis() as f64;
            metrics.average_operation_time_ms = 
                (metrics.average_operation_time_ms + elapsed) / 2.0;
        }).await;
        
        Ok(())
    }
    
    async fn load(&self, key: &str) -> Result<T, PersistenceError> {
        let start_time = std::time::Instant::now();
        
        let storage = self.data.read().await;
        let result = storage.get(key)
            .cloned()
            .ok_or_else(|| PersistenceError::DataNotFound(key.to_string()));
        
        self.update_metrics(|metrics| {
            metrics.read_operations += 1;
            if result.is_err() {
                metrics.error_count += 1;
            }
            let elapsed = start_time.elapsed().as_millis() as f64;
            metrics.average_operation_time_ms = 
                (metrics.average_operation_time_ms + elapsed) / 2.0;
        }).await;
        
        result
    }
    
    async fn delete(&self, key: &str) -> Result<(), PersistenceError> {
        let start_time = std::time::Instant::now();
        
        let mut storage = self.data.write().await;
        storage.remove(key);
        
        self.update_metrics(|metrics| {
            metrics.delete_operations += 1;
            metrics.total_keys = storage.len();
            let elapsed = start_time.elapsed().as_millis() as f64;
            metrics.average_operation_time_ms = 
                (metrics.average_operation_time_ms + elapsed) / 2.0;
        }).await;
        
        Ok(())
    }
    
    async fn list_keys(&self) -> Result<Vec<String>, PersistenceError> {
        let storage = self.data.read().await;
        Ok(storage.keys().cloned().collect())
    }
    
    async fn exists(&self, key: &str) -> Result<bool, PersistenceError> {
        let storage = self.data.read().await;
        Ok(storage.contains_key(key))
    }
}

#[async_trait::async_trait]
impl<T> AdvancedStorage<T> for MemoryStorage<T>
where
    T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    async fn batch_save(&self, items: Vec<(String, T)>) -> Result<BatchResult, PersistenceError> {
        let mut storage = self.data.write().await;
        let total_operations = items.len();
        let mut successful_operations = 0;
        let mut errors = Vec::new();
        
        for (key, value) in items {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                storage.insert(key.clone(), value);
            })) {
                Ok(()) => successful_operations += 1,
                Err(_) => errors.push((key, "插入失败".to_string())),
            }
        }
        
        self.update_metrics(|metrics| {
            metrics.write_operations += successful_operations as u64;
            metrics.total_keys = storage.len();
        }).await;
        
        Ok(BatchResult {
            total_operations,
            successful_operations,
            failed_operations: total_operations - successful_operations,
            errors,
        })
    }
    
    async fn batch_load(&self, keys: Vec<String>) -> Result<HashMap<String, T>, PersistenceError> {
        let storage = self.data.read().await;
        let mut result = HashMap::new();
        
        for key in keys {
            if let Some(value) = storage.get(&key) {
                result.insert(key, value.clone());
            }
        }
        
        self.update_metrics(|metrics| {
            metrics.read_operations += result.len() as u64;
        }).await;
        
        Ok(result)
    }
    
    async fn batch_delete(&self, keys: Vec<String>) -> Result<BatchResult, PersistenceError> {
        let mut storage = self.data.write().await;
        let total_operations = keys.len();
        let mut successful_operations = 0;
        
        for key in keys {
            if storage.remove(&key).is_some() {
                successful_operations += 1;
            }
        }
        
        self.update_metrics(|metrics| {
            metrics.delete_operations += successful_operations as u64;
            metrics.total_keys = storage.len();
        }).await;
        
        Ok(BatchResult {
            total_operations,
            successful_operations,
            failed_operations: total_operations - successful_operations,
            errors: Vec::new(),
        })
    }
    
    async fn list_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, PersistenceError> {
        let storage = self.data.read().await;
        let matching_keys = storage
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect();
        
        Ok(matching_keys)
    }
    
    async fn get_metrics(&self) -> Result<StorageMetrics, PersistenceError> {
        let metrics = self.metrics.read().await;
        Ok(metrics.clone())
    }
    
    async fn health_check(&self) -> Result<bool, PersistenceError> {
        // 内存存储总是健康的
        Ok(true)
    }
    
    async fn optimize(&self) -> Result<(), PersistenceError> {
        // 内存存储不需要优化
        Ok(())
    }
    
    async fn backup(&self, backup_path: &str) -> Result<(), PersistenceError> {
        let storage = self.data.read().await;
        let serialized = serde_json::to_string_pretty(&*storage)?;
        
        tokio::fs::write(backup_path, serialized).await
            .map_err(|e| PersistenceError::IoError(e))?;
        
        Ok(())
    }
    
    async fn restore(&self, backup_path: &str) -> Result<(), PersistenceError> {
        let content = tokio::fs::read_to_string(backup_path).await
            .map_err(|e| PersistenceError::IoError(e))?;
        
        let data: HashMap<String, T> = serde_json::from_str(&content)?;
        
        let mut storage = self.data.write().await;
        *storage = data;
        
        self.update_metrics(|metrics| {
            metrics.total_keys = storage.len();
        }).await;
        
        Ok(())
    }
}

/// 存储工厂
pub struct StorageFactory;

impl StorageFactory {
    /// 创建存储实例
    pub fn create_storage<T>(
        backend: StorageBackend,
        _config: PersistenceConfig,
    ) -> Result<Box<dyn AdvancedStorage<T>>, PersistenceError>
    where
        T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
    {
        match backend {
            StorageBackend::Memory => {
                Ok(Box::new(MemoryStorage::new()))
            }
            StorageBackend::FileSystem => {
                // 这里可以创建基于FileSystemStore的高级存储实现
                // 为了简化，暂时返回内存存储
                Ok(Box::new(MemoryStorage::new()))
            }
            StorageBackend::Redis { .. } => {
                Err(PersistenceError::InvalidFormat(
                    "Redis存储尚未实现".to_string()
                ))
            }
            StorageBackend::Database { .. } => {
                Err(PersistenceError::InvalidFormat(
                    "数据库存储尚未实现".to_string()
                ))
            }
        }
    }
}

/// 存储代理（支持多后端）
pub struct StorageProxy<T> {
    primary: Box<dyn AdvancedStorage<T>>,
    secondary: Option<Box<dyn AdvancedStorage<T>>>,
    write_through: bool,
}

impl<T> StorageProxy<T>
where
    T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
{
    pub fn new(
        primary: Box<dyn AdvancedStorage<T>>,
        secondary: Option<Box<dyn AdvancedStorage<T>>>,
        write_through: bool,
    ) -> Self {
        Self {
            primary,
            secondary,
            write_through,
        }
    }
    
    /// 写入数据到主存储和辅助存储
    pub async fn save(&self, key: &str, data: &T) -> Result<(), PersistenceError> {
        // 首先写入主存储
        self.primary.batch_save(vec![(key.to_string(), data.clone())]).await?;
        
        // 如果启用写透模式且有辅助存储，也写入辅助存储
        if self.write_through {
            if let Some(ref secondary) = self.secondary {
                secondary.batch_save(vec![(key.to_string(), data.clone())]).await.ok();
            }
        }
        
        Ok(())
    }
    
    /// 从存储读取数据（优先从主存储读取）
    pub async fn load(&self, key: &str) -> Result<T, PersistenceError> {
        // 首先尝试从主存储读取
        match self.primary.batch_load(vec![key.to_string()]).await {
            Ok(mut data) => {
                if let Some(value) = data.remove(key) {
                    return Ok(value);
                }
            }
            Err(_) => {}
        }
        
        // 如果主存储失败，尝试从辅助存储读取
        if let Some(ref secondary) = self.secondary {
            if let Ok(mut data) = secondary.batch_load(vec![key.to_string()]).await {
                if let Some(value) = data.remove(key) {
                    // 回写到主存储
                    self.primary.batch_save(vec![(key.to_string(), value.clone())]).await.ok();
                    return Ok(value);
                }
            }
        }
        
        Err(PersistenceError::DataNotFound(key.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        value: String,
        number: i32,
    }
    
    #[tokio::test]
    async fn test_memory_storage() {
        let storage: MemoryStorage<TestData> = MemoryStorage::new();
        
        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };
        
        use crate::persistence::DataStore;
        
        // 测试保存和加载
        storage.save("test_key", &test_data).await.unwrap();
        let loaded = storage.load("test_key").await.unwrap();
        assert_eq!(test_data, loaded);
        
        // 测试批量操作
        let batch_data = vec![
            ("key1".to_string(), TestData { value: "value1".to_string(), number: 1 }),
            ("key2".to_string(), TestData { value: "value2".to_string(), number: 2 }),
        ];
        
        let result = storage.batch_save(batch_data).await.unwrap();
        assert_eq!(result.successful_operations, 2);
        
        let loaded_batch = storage.batch_load(vec!["key1".to_string(), "key2".to_string()]).await.unwrap();
        assert_eq!(loaded_batch.len(), 2);
        
        // 测试统计信息
        let metrics = storage.get_metrics().await.unwrap();
        assert!(metrics.write_operations >= 3);
        assert!(metrics.read_operations >= 1);
        
        // 测试健康检查
        assert!(storage.health_check().await.unwrap());
    }
    
    #[tokio::test]
    async fn test_storage_proxy() {
        let primary: Box<dyn AdvancedStorage<TestData>> = Box::new(MemoryStorage::new());
        let secondary: Box<dyn AdvancedStorage<TestData>> = Box::new(MemoryStorage::new());
        
        let proxy = StorageProxy::new(primary, Some(secondary), true);
        
        let test_data = TestData {
            value: "proxy_test".to_string(),
            number: 100,
        };
        
        // 测试写入
        proxy.save("proxy_key", &test_data).await.unwrap();
        
        // 测试读取
        let loaded = proxy.load("proxy_key").await.unwrap();
        assert_eq!(test_data, loaded);
    }
}