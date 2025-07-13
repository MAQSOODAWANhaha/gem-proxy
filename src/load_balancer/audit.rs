use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::load_balancer::ApiKey;

/// 权重变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightChangeRecord {
    pub id: String,
    pub timestamp: u64,
    pub operator: String, // 操作者
    pub operation_type: OperationType,
    pub target_key_id: String,
    pub old_weight: u32,
    pub new_weight: u32,
    pub reason: String,
    pub source: ChangeSource,
    pub metadata: HashMap<String, String>,
}

/// 操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationType {
    /// 手动权重调整
    Manual,
    /// 智能优化
    Intelligent,
    /// 批量调整
    Batch,
    /// 回滚操作
    Rollback,
    /// 系统自动调整
    Automatic,
}

/// 变更来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeSource {
    /// Web管理界面
    WebUI,
    /// API调用
    API,
    /// 配置文件
    ConfigFile,
    /// 智能优化器
    Optimizer,
    /// 监控系统
    Monitor,
}

/// 权重变更快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightSnapshot {
    pub snapshot_id: String,
    pub timestamp: u64,
    pub weights: HashMap<String, u32>,
    pub description: String,
    pub created_by: String,
}

/// 审计查询条件
#[derive(Debug, Clone, Deserialize)]
pub struct AuditQuery {
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub operator: Option<String>,
    pub operation_type: Option<OperationType>,
    pub target_key_id: Option<String>,
    pub source: Option<ChangeSource>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 审计统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_changes: usize,
    pub changes_by_type: HashMap<String, usize>,
    pub changes_by_source: HashMap<String, usize>,
    pub changes_by_operator: HashMap<String, usize>,
    pub most_changed_keys: Vec<KeyChangeStats>,
    pub change_frequency: Vec<TimeSeriesPoint>,
}

/// 密钥变更统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyChangeStats {
    pub key_id: String,
    pub change_count: usize,
    pub total_weight_change: i32,
    pub last_change_time: u64,
}

/// 时间序列数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: u64,
    pub value: f64,
}

/// 权重变更审计系统
pub struct WeightAuditSystem {
    /// 变更记录存储
    change_records: Arc<RwLock<Vec<WeightChangeRecord>>>,
    /// 快照存储
    snapshots: Arc<RwLock<Vec<WeightSnapshot>>>,
    /// 配置
    config: AuditConfig,
}

/// 审计配置
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// 记录保留天数
    pub retention_days: u32,
    /// 最大记录数
    pub max_records: usize,
    /// 自动快照间隔（秒）
    pub auto_snapshot_interval: u64,
    /// 启用变更通知
    pub enable_notifications: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            retention_days: 30,
            max_records: 10000,
            auto_snapshot_interval: 3600, // 1小时
            enable_notifications: true,
        }
    }
}

impl WeightAuditSystem {
    pub fn new(config: AuditConfig) -> Self {
        Self {
            change_records: Arc::new(RwLock::new(Vec::new())),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// 记录权重变更
    pub async fn record_weight_change(
        &self,
        operator: &str,
        operation_type: OperationType,
        target_key_id: &str,
        old_weight: u32,
        new_weight: u32,
        reason: &str,
        source: ChangeSource,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<String, String> {
        let record_id = self.generate_record_id();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let record = WeightChangeRecord {
            id: record_id.clone(),
            timestamp,
            operator: operator.to_string(),
            operation_type,
            target_key_id: target_key_id.to_string(),
            old_weight,
            new_weight,
            reason: reason.to_string(),
            source,
            metadata: metadata.unwrap_or_default(),
        };

        let mut records = self.change_records.write().await;
        records.push(record);

        // 清理过期记录
        self.cleanup_old_records(&mut records).await;

        Ok(record_id)
    }

    /// 创建权重快照
    pub async fn create_snapshot(
        &self,
        weights: &HashMap<String, u32>,
        description: &str,
        created_by: &str,
    ) -> Result<String, String> {
        let snapshot_id = self.generate_snapshot_id();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let snapshot = WeightSnapshot {
            snapshot_id: snapshot_id.clone(),
            timestamp,
            weights: weights.clone(),
            description: description.to_string(),
            created_by: created_by.to_string(),
        };

        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot);

        // 限制快照数量
        if snapshots.len() > 100 {
            snapshots.remove(0);
        }

        Ok(snapshot_id)
    }

    /// 查询审计记录
    pub async fn query_audit_records(&self, query: &AuditQuery) -> Vec<WeightChangeRecord> {
        let records = self.change_records.read().await;
        let mut filtered_records: Vec<WeightChangeRecord> = records
            .iter()
            .filter(|record| {
                // 时间范围过滤
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

                // 操作者过滤
                if let Some(ref operator) = query.operator {
                    if !record.operator.contains(operator) {
                        return false;
                    }
                }

                // 操作类型过滤
                if let Some(ref operation_type) = query.operation_type {
                    if record.operation_type != *operation_type {
                        return false;
                    }
                }

                // 目标密钥过滤
                if let Some(ref target_key_id) = query.target_key_id {
                    if record.target_key_id != *target_key_id {
                        return false;
                    }
                }

                // 来源过滤
                if let Some(ref source) = query.source {
                    if record.source != *source {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // 按时间倒序排序
        filtered_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // 分页处理
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(filtered_records.len());
        
        filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect()
    }

    /// 获取审计统计信息
    pub async fn get_audit_statistics(&self, days: Option<u32>) -> AuditStatistics {
        let records = self.change_records.read().await;
        let cutoff_time = if let Some(days) = days {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - (days as u64 * 24 * 3600)
        } else {
            0
        };

        let filtered_records: Vec<&WeightChangeRecord> = records
            .iter()
            .filter(|r| r.timestamp >= cutoff_time)
            .collect();

        let total_changes = filtered_records.len();

        // 按类型统计
        let mut changes_by_type = HashMap::new();
        for record in &filtered_records {
            let type_name = format!("{:?}", record.operation_type);
            *changes_by_type.entry(type_name).or_insert(0) += 1;
        }

        // 按来源统计
        let mut changes_by_source = HashMap::new();
        for record in &filtered_records {
            let source_name = format!("{:?}", record.source);
            *changes_by_source.entry(source_name).or_insert(0) += 1;
        }

        // 按操作者统计
        let mut changes_by_operator = HashMap::new();
        for record in &filtered_records {
            *changes_by_operator.entry(record.operator.clone()).or_insert(0) += 1;
        }

        // 最常变更的密钥
        let mut key_stats = HashMap::new();
        for record in &filtered_records {
            let stats = key_stats.entry(record.target_key_id.clone()).or_insert(KeyChangeStats {
                key_id: record.target_key_id.clone(),
                change_count: 0,
                total_weight_change: 0,
                last_change_time: 0,
            });
            stats.change_count += 1;
            stats.total_weight_change += record.new_weight as i32 - record.old_weight as i32;
            stats.last_change_time = stats.last_change_time.max(record.timestamp);
        }

        let mut most_changed_keys: Vec<KeyChangeStats> = key_stats.into_values().collect();
        most_changed_keys.sort_by(|a, b| b.change_count.cmp(&a.change_count));
        most_changed_keys.truncate(10);

        // 变更频率时间序列（按小时统计）
        let mut hourly_changes = HashMap::new();
        for record in &filtered_records {
            let hour = record.timestamp / 3600 * 3600; // 按小时对齐
            *hourly_changes.entry(hour).or_insert(0) += 1;
        }

        let mut change_frequency: Vec<TimeSeriesPoint> = hourly_changes
            .into_iter()
            .map(|(timestamp, count)| TimeSeriesPoint {
                timestamp,
                value: count as f64,
            })
            .collect();
        change_frequency.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        AuditStatistics {
            total_changes,
            changes_by_type,
            changes_by_source,
            changes_by_operator,
            most_changed_keys,
            change_frequency,
        }
    }

    /// 获取快照列表
    pub async fn get_snapshots(&self, limit: Option<usize>) -> Vec<WeightSnapshot> {
        let snapshots = self.snapshots.read().await;
        let mut sorted_snapshots = snapshots.clone();
        sorted_snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if let Some(limit) = limit {
            sorted_snapshots.truncate(limit);
        }
        
        sorted_snapshots
    }

    /// 根据快照ID获取快照
    pub async fn get_snapshot(&self, snapshot_id: &str) -> Option<WeightSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.iter().find(|s| s.snapshot_id == snapshot_id).cloned()
    }

    /// 回滚到指定快照
    pub async fn rollback_to_snapshot(
        &self,
        snapshot_id: &str,
        operator: &str,
        reason: &str,
    ) -> Result<HashMap<String, u32>, String> {
        let snapshot = self.get_snapshot(snapshot_id).await
            .ok_or_else(|| format!("Snapshot {} not found", snapshot_id))?;

        // 记录回滚操作
        for (key_id, &new_weight) in &snapshot.weights {
            // 这里需要获取当前权重，实际实现时需要从KeyManager获取
            let old_weight = 100; // 占位符，实际应从系统获取
            
            let mut metadata = HashMap::new();
            metadata.insert("snapshot_id".to_string(), snapshot_id.to_string());
            metadata.insert("rollback_reason".to_string(), reason.to_string());

            self.record_weight_change(
                operator,
                OperationType::Rollback,
                key_id,
                old_weight,
                new_weight,
                &format!("回滚到快照: {}", snapshot.description),
                ChangeSource::WebUI,
                Some(metadata),
            ).await?;
        }

        Ok(snapshot.weights.clone())
    }

    /// 生成记录ID
    fn generate_record_id(&self) -> String {
        format!("rec_{}", SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos())
    }

    /// 生成快照ID
    fn generate_snapshot_id(&self) -> String {
        format!("snap_{}", SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos())
    }

    /// 清理过期记录
    async fn cleanup_old_records(&self, records: &mut Vec<WeightChangeRecord>) {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - (self.config.retention_days as u64 * 24 * 3600);

        records.retain(|record| record.timestamp > cutoff_time);

        // 限制记录数量
        if records.len() > self.config.max_records {
            let excess = records.len() - self.config.max_records;
            records.drain(0..excess);
        }
    }

    /// 导出审计记录
    pub async fn export_audit_records(
        &self,
        query: &AuditQuery,
        format: ExportFormat,
    ) -> Result<String, String> {
        let records = self.query_audit_records(query).await;
        
        match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(&records)
                    .map_err(|e| format!("JSON序列化失败: {}", e))
            }
            ExportFormat::Csv => {
                let mut csv_content = String::from("时间,操作者,操作类型,目标密钥,旧权重,新权重,原因,来源\n");
                for record in records {
                    let timestamp = chrono::DateTime::from_timestamp(record.timestamp as i64, 0)
                        .unwrap_or_default()
                        .format("%Y-%m-%d %H:%M:%S");
                    csv_content.push_str(&format!(
                        "{},{},{},{},{},{},{},{:?}\n",
                        timestamp,
                        record.operator,
                        format!("{:?}", record.operation_type),
                        record.target_key_id,
                        record.old_weight,
                        record.new_weight,
                        record.reason.replace(",", ";"),
                        record.source
                    ));
                }
                Ok(csv_content)
            }
        }
    }

    /// 获取权重变更趋势
    pub async fn get_weight_change_trend(&self, key_id: &str, days: u32) -> Vec<WeightTrendPoint> {
        let records = self.change_records.read().await;
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - (days as u64 * 24 * 3600);

        let mut trend_points = Vec::new();
        let mut current_weight = 100; // 默认初始权重

        for record in records.iter() {
            if record.target_key_id == key_id && record.timestamp >= cutoff_time {
                current_weight = record.new_weight;
                trend_points.push(WeightTrendPoint {
                    timestamp: record.timestamp,
                    weight: current_weight,
                    operation_type: record.operation_type.clone(),
                    operator: record.operator.clone(),
                });
            }
        }

        trend_points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        trend_points
    }
}

/// 导出格式
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
}

/// 权重趋势数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightTrendPoint {
    pub timestamp: u64,
    pub weight: u32,
    pub operation_type: OperationType,
    pub operator: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_weight_change() {
        let audit_system = WeightAuditSystem::new(AuditConfig::default());
        
        let record_id = audit_system.record_weight_change(
            "admin",
            OperationType::Manual,
            "key1",
            100,
            150,
            "调整负载均衡",
            ChangeSource::WebUI,
            None,
        ).await.unwrap();
        
        assert!(!record_id.is_empty());
        
        let query = AuditQuery {
            start_time: None,
            end_time: None,
            operator: None,
            operation_type: None,
            target_key_id: Some("key1".to_string()),
            source: None,
            limit: None,
            offset: None,
        };
        
        let records = audit_system.query_audit_records(&query).await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].target_key_id, "key1");
        assert_eq!(records[0].old_weight, 100);
        assert_eq!(records[0].new_weight, 150);
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let audit_system = WeightAuditSystem::new(AuditConfig::default());
        
        let mut weights = HashMap::new();
        weights.insert("key1".to_string(), 100);
        weights.insert("key2".to_string(), 200);
        
        let snapshot_id = audit_system.create_snapshot(
            &weights,
            "测试快照",
            "admin",
        ).await.unwrap();
        
        let snapshot = audit_system.get_snapshot(&snapshot_id).await.unwrap();
        assert_eq!(snapshot.weights.len(), 2);
        assert_eq!(snapshot.description, "测试快照");
    }

    #[tokio::test]
    async fn test_audit_statistics() {
        let audit_system = WeightAuditSystem::new(AuditConfig::default());
        
        // 记录几个变更
        audit_system.record_weight_change(
            "admin",
            OperationType::Manual,
            "key1",
            100,
            150,
            "测试",
            ChangeSource::WebUI,
            None,
        ).await.unwrap();
        
        audit_system.record_weight_change(
            "user1",
            OperationType::Intelligent,
            "key2",
            200,
            250,
            "智能优化",
            ChangeSource::Optimizer,
            None,
        ).await.unwrap();
        
        let stats = audit_system.get_audit_statistics(Some(7)).await;
        assert_eq!(stats.total_changes, 2);
        assert!(stats.changes_by_type.contains_key("Manual"));
        assert!(stats.changes_by_operator.contains_key("admin"));
    }
}