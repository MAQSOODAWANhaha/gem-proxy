// src/persistence/session_store.rs
//! 会话状态持久化存储
//! 
//! 提供用户会话的持久化存储，支持会话恢复、跨服务器实例共享会话状态

use super::{DataStore, FileSystemStore, PersistenceConfig, PersistenceError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Duration, Utc};

/// 持久化会话数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentSession {
    /// 会话ID
    pub session_id: String,
    /// 用户ID
    pub user_id: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_activity: DateTime<Utc>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 是否激活
    pub is_active: bool,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 客户端信息
    pub client_info: ClientInfo,
    /// 会话数据
    pub data: HashMap<String, String>,
    /// 权限信息
    pub permissions: Vec<String>,
}

/// 客户端信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// IP地址
    pub ip_address: String,
    /// 用户代理
    pub user_agent: Option<String>,
    /// 设备类型
    pub device_type: Option<String>,
    /// 地理位置信息
    pub location: Option<String>,
}

/// 会话统计信息
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionStatistics {
    /// 总会话数
    pub total_sessions: usize,
    /// 活跃会话数
    pub active_sessions: usize,
    /// 过期会话数
    pub expired_sessions: usize,
    /// 按用户分组的会话数
    pub sessions_by_user: HashMap<String, usize>,
    /// 按设备类型分组的会话数
    pub sessions_by_device: HashMap<String, usize>,
    /// 会话持续时间统计
    pub session_duration_stats: DurationStats,
    /// 最近登录时间线
    pub recent_logins: Vec<LoginActivity>,
}

/// 持续时间统计
#[derive(Debug, Serialize, Deserialize)]
pub struct DurationStats {
    pub average_duration_minutes: f64,
    pub max_duration_minutes: f64,
    pub min_duration_minutes: f64,
    pub total_duration_hours: f64,
}

/// 登录活动
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginActivity {
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub ip_address: String,
    pub device_type: Option<String>,
    pub success: bool,
}

/// 会话查询条件
#[derive(Debug, Default)]
pub struct SessionQuery {
    pub user_id: Option<String>,
    pub is_active: Option<bool>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub ip_address: Option<String>,
    pub device_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 会话存储管理器
pub struct SessionStore {
    /// 会话数据存储
    session_store: FileSystemStore<PersistentSession>,
    /// 登录活动存储
    activity_store: FileSystemStore<Vec<LoginActivity>>,
    /// 内存缓存
    cache: Arc<RwLock<HashMap<String, PersistentSession>>>,
    /// 配置
    config: SessionStoreConfig,
}

/// 会话存储配置
#[derive(Debug, Clone)]
pub struct SessionStoreConfig {
    /// 会话默认过期时间（分钟）
    pub default_session_timeout: i64,
    /// 启用内存缓存
    pub enable_cache: bool,
    /// 自动清理间隔（秒）
    pub cleanup_interval: u64,
    /// 最大会话数
    pub max_sessions: usize,
    /// 活动日志保留天数
    pub activity_retention_days: i64,
}

impl Default for SessionStoreConfig {
    fn default() -> Self {
        Self {
            default_session_timeout: 60, // 1小时
            enable_cache: true,
            cleanup_interval: 300, // 5分钟
            max_sessions: 10000,
            activity_retention_days: 30,
        }
    }
}

impl SessionStore {
    /// 创建新的会话存储
    pub fn new(persistence_config: PersistenceConfig, store_config: SessionStoreConfig) -> Self {
        let session_store = FileSystemStore::new(persistence_config.clone(), "sessions".to_string());
        let activity_store = FileSystemStore::new(persistence_config, "session_activities".to_string());
        
        Self {
            session_store,
            activity_store,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: store_config,
        }
    }
    
    /// 初始化会话存储
    pub async fn initialize(&self) -> Result<(), PersistenceError> {
        if self.config.enable_cache {
            self.load_active_sessions_to_cache().await?;
        }
        
        // 清理过期会话
        self.cleanup_expired_sessions().await.ok();
        
        tracing::info!("会话存储初始化完成");
        Ok(())
    }
    
    /// 创建新会话
    pub async fn create_session(
        &self,
        user_id: &str,
        client_info: ClientInfo,
        permissions: Vec<String>,
        custom_timeout: Option<Duration>,
    ) -> Result<PersistentSession, PersistenceError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let timeout = custom_timeout.unwrap_or_else(|| Duration::minutes(self.config.default_session_timeout));
        
        let session = PersistentSession {
            session_id: session_id.clone(),
            user_id: user_id.to_string(),
            created_at: now,
            last_activity: now,
            expires_at: now + timeout,
            is_active: true,
            refresh_token: Some(uuid::Uuid::new_v4().to_string()),
            client_info,
            data: HashMap::new(),
            permissions,
        };
        
        // 保存到存储
        self.session_store.save(&session_id, &session).await?;
        
        // 更新缓存
        if self.config.enable_cache {
            self.cache.write().await.insert(session_id.clone(), session.clone());
        }
        
        // 记录登录活动
        self.record_login_activity(&session, true).await.ok();
        
        tracing::info!("已创建会话: {} (用户: {})", session_id, user_id);
        Ok(session)
    }
    
    /// 获取会话
    pub async fn get_session(&self, session_id: &str) -> Result<Option<PersistentSession>, PersistenceError> {
        // 首先从缓存获取
        if self.config.enable_cache {
            let cache = self.cache.read().await;
            if let Some(session) = cache.get(session_id) {
                return Ok(Some(session.clone()));
            }
        }
        
        // 从存储加载
        match self.session_store.load(session_id).await {
            Ok(session) => {
                // 检查是否过期
                if session.expires_at < Utc::now() {
                    return Ok(None);
                }
                
                // 更新缓存
                if self.config.enable_cache {
                    self.cache.write().await.insert(session_id.to_string(), session.clone());
                }
                
                Ok(Some(session))
            }
            Err(PersistenceError::DataNotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    /// 更新会话活动时间
    pub async fn update_session_activity(&self, session_id: &str) -> Result<(), PersistenceError> {
        let mut session = self.get_session(session_id).await?
            .ok_or_else(|| PersistenceError::DataNotFound(session_id.to_string()))?;
        
        session.last_activity = Utc::now();
        
        // 保存更新
        self.session_store.save(session_id, &session).await?;
        
        // 更新缓存
        if self.config.enable_cache {
            self.cache.write().await.insert(session_id.to_string(), session);
        }
        
        Ok(())
    }
    
    /// 设置会话数据
    pub async fn set_session_data(
        &self,
        session_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), PersistenceError> {
        let mut session = self.get_session(session_id).await?
            .ok_or_else(|| PersistenceError::DataNotFound(session_id.to_string()))?;
        
        session.data.insert(key.to_string(), value.to_string());
        session.last_activity = Utc::now();
        
        // 保存更新
        self.session_store.save(session_id, &session).await?;
        
        // 更新缓存
        if self.config.enable_cache {
            self.cache.write().await.insert(session_id.to_string(), session);
        }
        
        Ok(())
    }
    
    /// 获取会话数据
    pub async fn get_session_data(&self, session_id: &str, key: &str) -> Result<Option<String>, PersistenceError> {
        let session = self.get_session(session_id).await?
            .ok_or_else(|| PersistenceError::DataNotFound(session_id.to_string()))?;
        
        Ok(session.data.get(key).cloned())
    }
    
    /// 删除会话
    pub async fn delete_session(&self, session_id: &str) -> Result<(), PersistenceError> {
        // 从存储删除
        self.session_store.delete(session_id).await?;
        
        // 从缓存删除
        if self.config.enable_cache {
            self.cache.write().await.remove(session_id);
        }
        
        tracing::info!("已删除会话: {}", session_id);
        Ok(())
    }
    
    /// 使会话失效
    pub async fn invalidate_session(&self, session_id: &str) -> Result<(), PersistenceError> {
        let mut session = self.get_session(session_id).await?
            .ok_or_else(|| PersistenceError::DataNotFound(session_id.to_string()))?;
        
        session.is_active = false;
        session.expires_at = Utc::now(); // 立即过期
        
        // 保存更新
        self.session_store.save(session_id, &session).await?;
        
        // 从缓存删除
        if self.config.enable_cache {
            self.cache.write().await.remove(session_id);
        }
        
        tracing::info!("已使会话失效: {}", session_id);
        Ok(())
    }
    
    /// 刷新会话（延长过期时间）
    pub async fn refresh_session(
        &self,
        session_id: &str,
        extend_duration: Option<Duration>,
    ) -> Result<PersistentSession, PersistenceError> {
        let mut session = self.get_session(session_id).await?
            .ok_or_else(|| PersistenceError::DataNotFound(session_id.to_string()))?;
        
        let extension = extend_duration.unwrap_or_else(|| Duration::minutes(self.config.default_session_timeout));
        session.expires_at = Utc::now() + extension;
        session.last_activity = Utc::now();
        
        // 保存更新
        self.session_store.save(session_id, &session).await?;
        
        // 更新缓存
        if self.config.enable_cache {
            self.cache.write().await.insert(session_id.to_string(), session.clone());
        }
        
        tracing::debug!("已刷新会话: {}", session_id);
        Ok(session)
    }
    
    /// 查询会话
    pub async fn query_sessions(&self, query: &SessionQuery) -> Result<Vec<PersistentSession>, PersistenceError> {
        let session_ids = self.session_store.list_keys().await?;
        let mut matching_sessions = Vec::new();
        
        for session_id in session_ids {
            if let Ok(Some(session)) = self.get_session(&session_id).await {
                if self.matches_session_query(&session, query) {
                    matching_sessions.push(session);
                }
            }
        }
        
        // 按最后活动时间倒序排序
        matching_sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        
        // 分页处理
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(matching_sessions.len());
        
        let results = matching_sessions
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();
        
        Ok(results)
    }
    
    /// 获取用户的所有活跃会话
    pub async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<PersistentSession>, PersistenceError> {
        let query = SessionQuery {
            user_id: Some(user_id.to_string()),
            is_active: Some(true),
            ..Default::default()
        };
        
        self.query_sessions(&query).await
    }
    
    /// 使用户的所有会话失效
    pub async fn invalidate_user_sessions(&self, user_id: &str) -> Result<usize, PersistenceError> {
        let sessions = self.get_user_sessions(user_id).await?;
        let mut invalidated_count = 0;
        
        for session in sessions {
            if self.invalidate_session(&session.session_id).await.is_ok() {
                invalidated_count += 1;
            }
        }
        
        tracing::info!("已使用户 {} 的 {} 个会话失效", user_id, invalidated_count);
        Ok(invalidated_count)
    }
    
    /// 清理过期会话
    pub async fn cleanup_expired_sessions(&self) -> Result<usize, PersistenceError> {
        let session_ids = self.session_store.list_keys().await?;
        let now = Utc::now();
        let mut cleaned_count = 0;
        
        for session_id in session_ids {
            if let Ok(session) = self.session_store.load(&session_id).await {
                if session.expires_at < now || !session.is_active {
                    self.session_store.delete(&session_id).await.ok();
                    
                    // 从缓存删除
                    if self.config.enable_cache {
                        self.cache.write().await.remove(&session_id);
                    }
                    
                    cleaned_count += 1;
                }
            }
        }
        
        if cleaned_count > 0 {
            tracing::info!("已清理 {} 个过期会话", cleaned_count);
        }
        
        Ok(cleaned_count)
    }
    
    /// 获取会话统计信息
    pub async fn get_statistics(&self) -> Result<SessionStatistics, PersistenceError> {
        let session_ids = self.session_store.list_keys().await?;
        let now = Utc::now();
        let mut sessions = Vec::new();
        
        for session_id in session_ids {
            if let Ok(session) = self.session_store.load(&session_id).await {
                sessions.push(session);
            }
        }
        
        let total_sessions = sessions.len();
        let active_sessions = sessions.iter().filter(|s| s.is_active && s.expires_at > now).count();
        let expired_sessions = sessions.iter().filter(|s| s.expires_at <= now).count();
        
        // 按用户统计
        let mut sessions_by_user = HashMap::new();
        for session in &sessions {
            *sessions_by_user.entry(session.user_id.clone()).or_insert(0) += 1;
        }
        
        // 按设备类型统计
        let mut sessions_by_device = HashMap::new();
        for session in &sessions {
            let device = session.client_info.device_type.as_ref()
                .unwrap_or(&"unknown".to_string()).clone();
            *sessions_by_device.entry(device).or_insert(0) += 1;
        }
        
        // 会话持续时间统计
        let durations: Vec<f64> = sessions
            .iter()
            .map(|s| (s.last_activity - s.created_at).num_minutes() as f64)
            .collect();
        
        let session_duration_stats = if !durations.is_empty() {
            DurationStats {
                average_duration_minutes: durations.iter().sum::<f64>() / durations.len() as f64,
                max_duration_minutes: durations.iter().copied().fold(0.0, f64::max),
                min_duration_minutes: durations.iter().copied().fold(f64::INFINITY, f64::min),
                total_duration_hours: durations.iter().sum::<f64>() / 60.0,
            }
        } else {
            DurationStats {
                average_duration_minutes: 0.0,
                max_duration_minutes: 0.0,
                min_duration_minutes: 0.0,
                total_duration_hours: 0.0,
            }
        };
        
        // 获取最近登录活动
        let recent_logins = self.get_recent_login_activities(10).await.unwrap_or_default();
        
        Ok(SessionStatistics {
            total_sessions,
            active_sessions,
            expired_sessions,
            sessions_by_user,
            sessions_by_device,
            session_duration_stats,
            recent_logins,
        })
    }
    
    // 私有辅助方法
    
    /// 加载活跃会话到缓存
    async fn load_active_sessions_to_cache(&self) -> Result<(), PersistenceError> {
        let session_ids = self.session_store.list_keys().await?;
        let now = Utc::now();
        let mut cache = self.cache.write().await;
        
        for session_id in session_ids {
            if let Ok(session) = self.session_store.load(&session_id).await {
                if session.is_active && session.expires_at > now {
                    cache.insert(session_id, session);
                }
            }
        }
        
        tracing::info!("已加载 {} 个活跃会话到缓存", cache.len());
        Ok(())
    }
    
    /// 检查会话是否匹配查询条件
    fn matches_session_query(&self, session: &PersistentSession, query: &SessionQuery) -> bool {
        if let Some(ref user_id) = query.user_id {
            if session.user_id != *user_id {
                return false;
            }
        }
        
        if let Some(is_active) = query.is_active {
            if session.is_active != is_active {
                return false;
            }
        }
        
        if let Some(created_after) = query.created_after {
            if session.created_at < created_after {
                return false;
            }
        }
        
        if let Some(created_before) = query.created_before {
            if session.created_at > created_before {
                return false;
            }
        }
        
        if let Some(ref ip_address) = query.ip_address {
            if session.client_info.ip_address != *ip_address {
                return false;
            }
        }
        
        if let Some(ref device_type) = query.device_type {
            if session.client_info.device_type.as_ref() != Some(device_type) {
                return false;
            }
        }
        
        true
    }
    
    /// 记录登录活动
    async fn record_login_activity(&self, session: &PersistentSession, success: bool) -> Result<(), PersistenceError> {
        let activity = LoginActivity {
            timestamp: session.created_at,
            user_id: session.user_id.clone(),
            ip_address: session.client_info.ip_address.clone(),
            device_type: session.client_info.device_type.clone(),
            success,
        };
        
        let today = Utc::now().format("%Y-%m-%d").to_string();
        
        // 获取今天的活动记录
        let mut activities = match self.activity_store.load(&today).await {
            Ok(activities) => activities,
            Err(PersistenceError::DataNotFound(_)) => Vec::new(),
            Err(e) => return Err(e),
        };
        
        activities.push(activity);
        
        // 保存更新的活动记录
        self.activity_store.save(&today, &activities).await?;
        
        Ok(())
    }
    
    /// 获取最近的登录活动
    async fn get_recent_login_activities(&self, limit: usize) -> Result<Vec<LoginActivity>, PersistenceError> {
        let activity_keys = self.activity_store.list_keys().await?;
        let mut all_activities = Vec::new();
        
        // 获取最近几天的活动记录
        for key in activity_keys.iter().rev().take(7) { // 最近7天
            if let Ok(activities) = self.activity_store.load(key).await {
                all_activities.extend(activities);
            }
        }
        
        // 按时间倒序排序并限制数量
        all_activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_activities.truncate(limit);
        
        Ok(all_activities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_session_operations() {
        let temp_dir = tempdir().unwrap();
        let persistence_config = PersistenceConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let session_store = SessionStore::new(persistence_config, SessionStoreConfig::default());
        session_store.initialize().await.unwrap();
        
        let client_info = ClientInfo {
            ip_address: "127.0.0.1".to_string(),
            user_agent: Some("TestAgent/1.0".to_string()),
            device_type: Some("desktop".to_string()),
            location: None,
        };
        
        // 测试创建会话
        let session = session_store.create_session(
            "user1",
            client_info,
            vec!["read".to_string(), "write".to_string()],
            None,
        ).await.unwrap();
        
        assert_eq!(session.user_id, "user1");
        assert!(session.is_active);
        
        // 测试获取会话
        let retrieved = session_store.get_session(&session.session_id).await.unwrap().unwrap();
        assert_eq!(retrieved.user_id, "user1");
        
        // 测试设置会话数据
        session_store.set_session_data(&session.session_id, "test_key", "test_value").await.unwrap();
        
        // 测试获取会话数据
        let value = session_store.get_session_data(&session.session_id, "test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
        
        // 测试刷新会话
        let refreshed = session_store.refresh_session(&session.session_id, None).await.unwrap();
        assert!(refreshed.expires_at > session.expires_at);
        
        // 测试查询会话
        let query = SessionQuery {
            user_id: Some("user1".to_string()),
            ..Default::default()
        };
        let sessions = session_store.query_sessions(&query).await.unwrap();
        assert_eq!(sessions.len(), 1);
        
        // 测试统计信息
        let stats = session_store.get_statistics().await.unwrap();
        assert_eq!(stats.active_sessions, 1);
        assert!(stats.sessions_by_user.contains_key("user1"));
        
        // 测试使会话失效
        session_store.invalidate_session(&session.session_id).await.unwrap();
        let invalidated = session_store.get_session(&session.session_id).await.unwrap();
        assert!(invalidated.is_none());
    }
}