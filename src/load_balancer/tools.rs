use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::load_balancer::{
    key_manager::ApiKey, 
    audit::{WeightAuditSystem, OperationType, ChangeSource}
};
use crate::config::ApiKeyConfig;
// use crate::persistence::weight_presets::WeightPresetStore;

/// 权重预设模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub weights: HashMap<String, u32>,
    pub created_by: String,
    pub created_at: u64,
    pub tags: Vec<String>,
}

/// 权重分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightAnalysis {
    pub load_balance_score: f64,
    pub variance_coefficient: f64,
    pub efficiency_score: f64,
    pub recommended_adjustments: Vec<WeightRecommendation>,
    pub risk_assessment: RiskAssessment,
}

/// 权重调整建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightRecommendation {
    pub key_id: String,
    pub current_weight: u32,
    pub recommended_weight: u32,
    pub reason: String,
    pub priority: Priority,
    pub impact_score: f64,
}

/// 优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// 风险评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub mitigation_suggestions: Vec<String>,
}

/// 风险级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// 风险因素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: String,
    pub description: String,
    pub severity: RiskLevel,
    pub affected_keys: Vec<String>,
}

/// 权重管理工具集
#[allow(dead_code)]
pub struct WeightManagementToolkit {
    /// 权重预设存储（内存）
    presets: Arc<RwLock<Vec<WeightPreset>>>,
    /// 审计系统引用
    audit_system: Arc<RwLock<WeightAuditSystem>>,
    /// 工具配置
    config: ToolkitConfig,
}

/// 工具集配置
#[derive(Debug, Clone)]
pub struct ToolkitConfig {
    /// 启用自动分析
    pub auto_analysis_enabled: bool,
    /// 分析间隔（秒）
    pub analysis_interval: u64,
    /// 最大预设数量
    pub max_presets: usize,
    /// 风险阈值
    pub risk_thresholds: RiskThresholds,
}

/// 风险阈值配置
#[derive(Debug, Clone)]
pub struct RiskThresholds {
    /// 权重不平衡阈值
    pub imbalance_threshold: f64,
    /// 方差系数阈值
    pub variance_threshold: f64,
    /// 单个密钥最大权重比例
    pub max_single_key_ratio: f64,
}

impl Default for ToolkitConfig {
    fn default() -> Self {
        Self {
            auto_analysis_enabled: true,
            analysis_interval: 300, // 5分钟
            max_presets: 50,
            risk_thresholds: RiskThresholds {
                imbalance_threshold: 0.3,
                variance_threshold: 0.5,
                max_single_key_ratio: 0.7,
            },
        }
    }
}

impl WeightManagementToolkit {
    pub fn new(
        audit_system: Arc<RwLock<WeightAuditSystem>>,
        config: ToolkitConfig,
    ) -> Self {
        Self {
            presets: Arc::new(RwLock::new(Vec::new())),
            audit_system,
            config,
        }
    }

    /// 将 ApiKeyConfig 转换为 ApiKey
    fn config_to_api_key(config: &ApiKeyConfig) -> ApiKey {
        ApiKey {
            id: config.id.clone(),
            key: config.key.clone(),
            weight: config.weight,
            max_requests_per_minute: config.max_requests_per_minute,
            current_requests: 0,
            last_reset: chrono::Utc::now(),
            is_active: true,
            failure_count: 0,
        }
    }

    /// 将 ApiKey 的权重更新回 ApiKeyConfig
    fn update_config_weight(config: &mut ApiKeyConfig, api_key: &ApiKey) {
        config.weight = api_key.weight;
    }

    /// 创建权重预设
    pub async fn create_preset(
        &self,
        name: String,
        description: String,
        weights: HashMap<String, u32>,
        created_by: String,
        tags: Vec<String>,
    ) -> Result<String, String> {
        let preset_id = self.generate_preset_id();
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let preset = WeightPreset {
            id: preset_id.clone(),
            name,
            description,
            weights,
            created_by,
            created_at,
            tags,
        };

        let mut presets = self.presets.write().await;
        
        // 检查预设数量限制
        if presets.len() >= self.config.max_presets {
            return Err("预设数量已达到上限".to_string());
        }

        presets.push(preset);
        Ok(preset_id)
    }

    /// 获取所有预设
    pub async fn get_presets(&self) -> Vec<WeightPreset> {
        let presets = self.presets.read().await;
        presets.clone()
    }

    /// 根据ID获取预设
    pub async fn get_preset(&self, preset_id: &str) -> Option<WeightPreset> {
        let presets = self.presets.read().await;
        presets.iter().find(|p| p.id == preset_id).cloned()
    }

    /// 应用权重预设（使用 ApiKeyConfig）
    pub async fn apply_preset_config(
        &self,
        preset_id: &str,
        operator: &str,
        api_key_configs: &mut Vec<ApiKeyConfig>,
    ) -> Result<(), String> {
        let preset = self.get_preset(preset_id).await
            .ok_or_else(|| format!("预设 {} 不存在", preset_id))?;

        // 记录权重变更到审计系统
        let audit_system = self.audit_system.read().await;
        
        for config in api_key_configs.iter_mut() {
            if let Some(&new_weight) = preset.weights.get(&config.id) {
                let old_weight = config.weight;
                config.weight = new_weight;
                
                // 记录变更
                let mut metadata = HashMap::new();
                metadata.insert("preset_id".to_string(), preset_id.to_string());
                metadata.insert("preset_name".to_string(), preset.name.clone());
                
                audit_system.record_weight_change(
                    operator,
                    OperationType::Batch,
                    &config.id,
                    old_weight,
                    new_weight,
                    &format!("应用预设: {}", preset.name),
                    ChangeSource::WebUI,
                    Some(metadata),
                ).await?;
            }
        }

        Ok(())
    }

    /// 应用权重预设
    pub async fn apply_preset(
        &self,
        preset_id: &str,
        operator: &str,
        api_keys: &mut Vec<ApiKey>,
    ) -> Result<(), String> {
        let preset = self.get_preset(preset_id).await
            .ok_or_else(|| format!("预设 {} 不存在", preset_id))?;

        // 记录权重变更到审计系统
        let audit_system = self.audit_system.read().await;
        
        for api_key in api_keys.iter_mut() {
            if let Some(&new_weight) = preset.weights.get(&api_key.id) {
                let old_weight = api_key.weight;
                api_key.weight = new_weight;
                
                // 记录变更
                let mut metadata = HashMap::new();
                metadata.insert("preset_id".to_string(), preset_id.to_string());
                metadata.insert("preset_name".to_string(), preset.name.clone());
                
                audit_system.record_weight_change(
                    operator,
                    OperationType::Batch,
                    &api_key.id,
                    old_weight,
                    new_weight,
                    &format!("应用预设: {}", preset.name),
                    ChangeSource::WebUI,
                    Some(metadata),
                ).await?;
            }
        }

        Ok(())
    }

    /// 删除预设
    pub async fn delete_preset(&self, preset_id: &str) -> Result<(), String> {
        let mut presets = self.presets.write().await;
        let initial_len = presets.len();
        presets.retain(|p| p.id != preset_id);
        
        if presets.len() == initial_len {
            return Err(format!("预设 {} 不存在", preset_id));
        }
        
        Ok(())
    }

    /// 分析当前权重配置（使用 ApiKeyConfig）
    pub async fn analyze_weights_config(&self, api_key_configs: &[ApiKeyConfig]) -> WeightAnalysis {
        let api_keys: Vec<ApiKey> = api_key_configs.iter().map(Self::config_to_api_key).collect();
        self.analyze_weights(&api_keys).await
    }

    /// 分析当前权重配置
    pub async fn analyze_weights(&self, api_keys: &[ApiKey]) -> WeightAnalysis {
        let weights: Vec<u32> = api_keys.iter().map(|k| k.weight).collect();
        let total_weight: u32 = weights.iter().sum();
        
        if weights.is_empty() || total_weight == 0 {
            return WeightAnalysis {
                load_balance_score: 0.0,
                variance_coefficient: 0.0,
                efficiency_score: 0.0,
                recommended_adjustments: vec![],
                risk_assessment: RiskAssessment {
                    overall_risk: RiskLevel::High,
                    risk_factors: vec![RiskFactor {
                        factor_type: "NoWeights".to_string(),
                        description: "没有可用的权重配置".to_string(),
                        severity: RiskLevel::Critical,
                        affected_keys: vec![],
                    }],
                    mitigation_suggestions: vec!["请配置API密钥权重".to_string()],
                },
            };
        }

        // 计算负载均衡评分
        let load_balance_score = self.calculate_balance_score(&weights);
        
        // 计算方差系数
        let variance_coefficient = self.calculate_variance_coefficient(&weights);
        
        // 计算效率评分
        let efficiency_score = self.calculate_efficiency_score(api_keys);
        
        // 生成调整建议
        let recommended_adjustments = self.generate_weight_recommendations(api_keys);
        
        // 风险评估
        let risk_assessment = self.assess_risks(api_keys, &weights, total_weight);

        WeightAnalysis {
            load_balance_score,
            variance_coefficient,
            efficiency_score,
            recommended_adjustments,
            risk_assessment,
        }
    }

    /// 权重标准化（使用 ApiKeyConfig）
    pub async fn normalize_weights_config(
        &self,
        api_key_configs: &mut Vec<ApiKeyConfig>,
        target_total: u32,
        operator: &str,
    ) -> Result<(), String> {
        let current_total: u32 = api_key_configs.iter().map(|k| k.weight).sum();
        
        if current_total == 0 {
            return Err("当前总权重为0，无法标准化".to_string());
        }

        let audit_system = self.audit_system.read().await;
        
        for config in api_key_configs.iter_mut() {
            let old_weight = config.weight;
            let new_weight = ((config.weight as f64 / current_total as f64) * target_total as f64) as u32;
            config.weight = new_weight;
            
            // 记录变更
            audit_system.record_weight_change(
                operator,
                OperationType::Automatic,
                &config.id,
                old_weight,
                new_weight,
                &format!("权重标准化 (目标总重: {})", target_total),
                ChangeSource::WebUI,
                None,
            ).await?;
        }

        Ok(())
    }

    /// 权重标准化
    pub async fn normalize_weights(
        &self,
        api_keys: &mut Vec<ApiKey>,
        target_total: u32,
        operator: &str,
    ) -> Result<(), String> {
        let current_total: u32 = api_keys.iter().map(|k| k.weight).sum();
        
        if current_total == 0 {
            return Err("当前总权重为0，无法标准化".to_string());
        }

        let audit_system = self.audit_system.read().await;
        
        for api_key in api_keys.iter_mut() {
            let old_weight = api_key.weight;
            let new_weight = ((api_key.weight as f64 / current_total as f64) * target_total as f64) as u32;
            api_key.weight = new_weight;
            
            // 记录变更
            audit_system.record_weight_change(
                operator,
                OperationType::Automatic,
                &api_key.id,
                old_weight,
                new_weight,
                &format!("权重标准化 (目标总重: {})", target_total),
                ChangeSource::WebUI,
                None,
            ).await?;
        }

        Ok(())
    }

    /// 权重均分（使用 ApiKeyConfig）
    pub async fn distribute_weights_evenly_config(
        &self,
        api_key_configs: &mut Vec<ApiKeyConfig>,
        total_weight: u32,
        operator: &str,
    ) -> Result<(), String> {
        if api_key_configs.is_empty() {
            return Err("没有API密钥可供分配".to_string());
        }

        let weight_per_key = total_weight / api_key_configs.len() as u32;
        let remainder = total_weight % api_key_configs.len() as u32;
        
        let audit_system = self.audit_system.read().await;

        for (index, config) in api_key_configs.iter_mut().enumerate() {
            let old_weight = config.weight;
            let new_weight = if index < remainder as usize {
                weight_per_key + 1
            } else {
                weight_per_key
            };
            config.weight = new_weight;
            
            // 记录变更
            audit_system.record_weight_change(
                operator,
                OperationType::Batch,
                &config.id,
                old_weight,
                new_weight,
                "权重均分分配",
                ChangeSource::WebUI,
                None,
            ).await?;
        }

        Ok(())
    }

    /// 权重均分
    pub async fn distribute_weights_evenly(
        &self,
        api_keys: &mut Vec<ApiKey>,
        total_weight: u32,
        operator: &str,
    ) -> Result<(), String> {
        if api_keys.is_empty() {
            return Err("没有API密钥可供分配".to_string());
        }

        let weight_per_key = total_weight / api_keys.len() as u32;
        let remainder = total_weight % api_keys.len() as u32;
        
        let audit_system = self.audit_system.read().await;

        for (index, api_key) in api_keys.iter_mut().enumerate() {
            let old_weight = api_key.weight;
            let new_weight = if index < remainder as usize {
                weight_per_key + 1
            } else {
                weight_per_key
            };
            api_key.weight = new_weight;
            
            // 记录变更
            audit_system.record_weight_change(
                operator,
                OperationType::Batch,
                &api_key.id,
                old_weight,
                new_weight,
                "权重均分分配",
                ChangeSource::WebUI,
                None,
            ).await?;
        }

        Ok(())
    }

    /// 根据性能自动调整权重（使用 ApiKeyConfig）
    pub async fn auto_adjust_weights_config(
        &self,
        api_key_configs: &mut Vec<ApiKeyConfig>,
        performance_data: &HashMap<String, PerformanceMetrics>,
        operator: &str,
    ) -> Result<(), String> {
        let audit_system = self.audit_system.read().await;
        
        for config in api_key_configs.iter_mut() {
            if let Some(perf) = performance_data.get(&config.id) {
                let old_weight = config.weight;
                let new_weight = self.calculate_performance_based_weight(perf, old_weight);
                
                if new_weight != old_weight {
                    config.weight = new_weight;
                    
                    // 记录变更
                    audit_system.record_weight_change(
                        operator,
                        OperationType::Automatic,
                        &config.id,
                        old_weight,
                        new_weight,
                        &format!("基于性能自动调整 (响应时间: {}ms, 成功率: {:.1}%)", 
                                perf.avg_response_time, perf.success_rate * 100.0),
                        ChangeSource::Monitor,
                        None,
                    ).await?;
                }
            }
        }

        Ok(())
    }

    /// 根据性能自动调整权重
    pub async fn auto_adjust_weights(
        &self,
        api_keys: &mut Vec<ApiKey>,
        performance_data: &HashMap<String, PerformanceMetrics>,
        operator: &str,
    ) -> Result<(), String> {
        let audit_system = self.audit_system.read().await;
        
        for api_key in api_keys.iter_mut() {
            if let Some(perf) = performance_data.get(&api_key.id) {
                let old_weight = api_key.weight;
                let new_weight = self.calculate_performance_based_weight(perf, old_weight);
                
                if new_weight != old_weight {
                    api_key.weight = new_weight;
                    
                    // 记录变更
                    audit_system.record_weight_change(
                        operator,
                        OperationType::Automatic,
                        &api_key.id,
                        old_weight,
                        new_weight,
                        &format!("基于性能自动调整 (响应时间: {}ms, 成功率: {:.1}%)", 
                                perf.avg_response_time, perf.success_rate * 100.0),
                        ChangeSource::Monitor,
                        None,
                    ).await?;
                }
            }
        }

        Ok(())
    }

    /// 权重健康检查（使用 ApiKeyConfig）
    pub async fn health_check_config(&self, api_key_configs: &[ApiKeyConfig]) -> HealthCheckResult {
        let api_keys: Vec<ApiKey> = api_key_configs.iter().map(Self::config_to_api_key).collect();
        self.health_check(&api_keys).await
    }

    /// 权重健康检查
    pub async fn health_check(&self, api_keys: &[ApiKey]) -> HealthCheckResult {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        let mut recommendations = Vec::new();

        let total_weight: u32 = api_keys.iter().map(|k| k.weight).sum();
        let active_keys: Vec<&ApiKey> = api_keys.iter().filter(|k| k.is_active).collect();

        // 检查总权重
        if total_weight == 0 {
            issues.push("总权重为0，负载均衡无法正常工作".to_string());
        }

        // 检查启用的密钥数量
        if active_keys.is_empty() {
            issues.push("没有启用的API密钥".to_string());
        } else if active_keys.len() == 1 {
            warnings.push("只有一个启用的API密钥，没有负载均衡效果".to_string());
        }

        // 检查权重分布
        if !active_keys.is_empty() {
            let weights: Vec<u32> = active_keys.iter().map(|k| k.weight).collect();
            let max_weight = *weights.iter().max().unwrap();
            let min_weight = *weights.iter().min().unwrap();
            
            if min_weight == 0 {
                warnings.push("存在权重为0的启用密钥".to_string());
            }
            
            if max_weight > 0 && (max_weight as f64 / total_weight as f64) > self.config.risk_thresholds.max_single_key_ratio {
                warnings.push(format!("单个密钥权重占比过高 ({:.1}%)", 
                    max_weight as f64 / total_weight as f64 * 100.0));
                recommendations.push("建议调整权重分布，避免单点依赖".to_string());
            }
        }

        // 检查权重变异系数
        let analysis = self.analyze_weights(api_keys).await;
        if analysis.variance_coefficient > self.config.risk_thresholds.variance_threshold {
            warnings.push(format!("权重分布不均衡，变异系数: {:.3}", analysis.variance_coefficient));
            recommendations.push("建议使用权重标准化工具平衡分布".to_string());
        }

        let status = if !issues.is_empty() {
            HealthStatus::Critical
        } else if !warnings.is_empty() {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        HealthCheckResult {
            status,
            issues,
            warnings,
            recommendations,
            score: analysis.load_balance_score,
        }
    }

    // 私有辅助方法

    fn generate_preset_id(&self) -> String {
        format!("preset_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos())
    }

    fn calculate_balance_score(&self, weights: &[u32]) -> f64 {
        if weights.is_empty() {
            return 0.0;
        }

        let mean = weights.iter().sum::<u32>() as f64 / weights.len() as f64;
        let variance = weights.iter()
            .map(|&w| (w as f64 - mean).powi(2))
            .sum::<f64>() / weights.len() as f64;
        
        let cv = variance.sqrt() / mean;
        (100.0 * (1.0 - cv.min(1.0))).max(0.0)
    }

    fn calculate_variance_coefficient(&self, weights: &[u32]) -> f64 {
        if weights.is_empty() {
            return 0.0;
        }

        let mean = weights.iter().sum::<u32>() as f64 / weights.len() as f64;
        let variance = weights.iter()
            .map(|&w| (w as f64 - mean).powi(2))
            .sum::<f64>() / weights.len() as f64;
        
        if mean == 0.0 {
            0.0
        } else {
            variance.sqrt() / mean
        }
    }

    fn calculate_efficiency_score(&self, api_keys: &[ApiKey]) -> f64 {
        // 简化的效率计算，实际应该基于性能指标
        let enabled_ratio = api_keys.iter().filter(|k| k.is_active).count() as f64 / api_keys.len() as f64;
        enabled_ratio * 100.0
    }

    fn generate_weight_recommendations(&self, api_keys: &[ApiKey]) -> Vec<WeightRecommendation> {
        let mut recommendations = Vec::new();
        let total_weight: u32 = api_keys.iter().map(|k| k.weight).sum();
        
        if total_weight == 0 {
            return recommendations;
        }

        for api_key in api_keys {
            let weight_ratio = api_key.weight as f64 / total_weight as f64;
            
            // 检查权重过高
            if weight_ratio > self.config.risk_thresholds.max_single_key_ratio {
                recommendations.push(WeightRecommendation {
                    key_id: api_key.id.clone(),
                    current_weight: api_key.weight,
                    recommended_weight: (total_weight as f64 * 0.5) as u32,
                    reason: "权重占比过高，建议降低以提高负载均衡效果".to_string(),
                    priority: Priority::High,
                    impact_score: 8.0,
                });
            }
            
            // 检查权重过低
            if api_key.is_active && api_key.weight == 0 {
                recommendations.push(WeightRecommendation {
                    key_id: api_key.id.clone(),
                    current_weight: api_key.weight,
                    recommended_weight: total_weight / api_keys.len() as u32,
                    reason: "启用的密钥权重为0，无法接收请求".to_string(),
                    priority: Priority::Critical,
                    impact_score: 10.0,
                });
            }
        }

        recommendations
    }

    fn assess_risks(&self, api_keys: &[ApiKey], weights: &[u32], total_weight: u32) -> RiskAssessment {
        let mut risk_factors = Vec::new();
        let mut mitigation_suggestions = Vec::new();

        // 检查权重分布风险
        if !weights.is_empty() {
            let max_weight = *weights.iter().max().unwrap();
            if max_weight as f64 / total_weight as f64 > self.config.risk_thresholds.max_single_key_ratio {
                risk_factors.push(RiskFactor {
                    factor_type: "SinglePointFailure".to_string(),
                    description: "单个密钥权重占比过高，存在单点故障风险".to_string(),
                    severity: RiskLevel::High,
                    affected_keys: api_keys.iter()
                        .filter(|k| k.weight == max_weight)
                        .map(|k| k.id.clone())
                        .collect(),
                });
                mitigation_suggestions.push("分散权重分布，避免单点依赖".to_string());
            }
        }

        // 检查禁用密钥风险
        let disabled_keys: Vec<&ApiKey> = api_keys.iter().filter(|k| !k.is_active).collect();
        if !disabled_keys.is_empty() {
            risk_factors.push(RiskFactor {
                factor_type: "DisabledKeys".to_string(),
                description: format!("存在{}个禁用的密钥，可能影响可用性", disabled_keys.len()),
                severity: RiskLevel::Medium,
                affected_keys: disabled_keys.iter().map(|k| k.id.clone()).collect(),
            });
            mitigation_suggestions.push("检查禁用密钥的状态，必要时重新启用".to_string());
        }

        // 评估整体风险级别
        let overall_risk = if risk_factors.iter().any(|f| matches!(f.severity, RiskLevel::Critical)) {
            RiskLevel::Critical
        } else if risk_factors.iter().any(|f| matches!(f.severity, RiskLevel::High)) {
            RiskLevel::High
        } else if risk_factors.iter().any(|f| matches!(f.severity, RiskLevel::Medium)) {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        RiskAssessment {
            overall_risk,
            risk_factors,
            mitigation_suggestions,
        }
    }

    fn calculate_performance_based_weight(&self, perf: &PerformanceMetrics, current_weight: u32) -> u32 {
        // 基于性能指标计算新权重
        let response_time_factor = 1.0 / (perf.avg_response_time / 1000.0 + 1.0); // 响应时间越短越好
        let success_rate_factor = perf.success_rate; // 成功率越高越好
        let performance_score = response_time_factor * 0.6 + success_rate_factor * 0.4;
        
        let adjustment_factor = performance_score * 1.2; // 最多增加20%
        let new_weight = (current_weight as f64 * adjustment_factor) as u32;
        
        new_weight.max(10).min(current_weight * 2) // 限制调整范围
    }
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub avg_response_time: f64, // 毫秒
    pub success_rate: f64,      // 0.0-1.0
    pub error_rate: f64,        // 0.0-1.0
    pub throughput: f64,        // 请求/秒
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    pub score: f64,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_balancer::AuditConfig;

    #[tokio::test]
    async fn test_create_preset() {
        let audit_system = Arc::new(RwLock::new(
            WeightAuditSystem::new(AuditConfig::default())
        ));
        let toolkit = WeightManagementToolkit::new(audit_system, ToolkitConfig::default());
        
        let mut weights = HashMap::new();
        weights.insert("key1".to_string(), 100);
        weights.insert("key2".to_string(), 200);
        
        let preset_id = toolkit.create_preset(
            "测试预设".to_string(),
            "用于测试的权重预设".to_string(),
            weights,
            "admin".to_string(),
            vec!["test".to_string()],
        ).await.unwrap();
        
        assert!(!preset_id.is_empty());
        
        let preset = toolkit.get_preset(&preset_id).await.unwrap();
        assert_eq!(preset.name, "测试预设");
        assert_eq!(preset.weights.len(), 2);
    }

    #[tokio::test]
    async fn test_weight_analysis() {
        let audit_system = Arc::new(RwLock::new(
            WeightAuditSystem::new(AuditConfig::default())
        ));
        let toolkit = WeightManagementToolkit::new(audit_system, ToolkitConfig::default());
        
        let api_keys = vec![
            ApiKey { 
                id: "key1".to_string(), 
                key: "test-key-1".to_string(),
                weight: 100, 
                max_requests_per_minute: 60,
                current_requests: 0,
                last_reset: chrono::Utc::now(),
                is_active: true,
                failure_count: 0,
            },
            ApiKey { 
                id: "key2".to_string(), 
                key: "test-key-2".to_string(),
                weight: 200, 
                max_requests_per_minute: 60,
                current_requests: 0,
                last_reset: chrono::Utc::now(),
                is_active: true,
                failure_count: 0,
            },
            ApiKey { 
                id: "key3".to_string(), 
                key: "test-key-3".to_string(),
                weight: 50, 
                max_requests_per_minute: 60,
                current_requests: 0,
                last_reset: chrono::Utc::now(),
                is_active: false,
                failure_count: 0,
            },
        ];
        
        let analysis = toolkit.analyze_weights(&api_keys).await;
        
        assert!(analysis.load_balance_score >= 0.0);
        assert!(analysis.load_balance_score <= 100.0);
        assert!(!analysis.recommended_adjustments.is_empty());
    }

    #[tokio::test]
    async fn test_normalize_weights() {
        let audit_system = Arc::new(RwLock::new(
            WeightAuditSystem::new(AuditConfig::default())
        ));
        let toolkit = WeightManagementToolkit::new(audit_system, ToolkitConfig::default());
        
        let mut api_keys = vec![
            ApiKey { 
                id: "key1".to_string(), 
                key: "test-key-1".to_string(),
                weight: 150, 
                max_requests_per_minute: 60,
                current_requests: 0,
                last_reset: chrono::Utc::now(),
                is_active: true,
                failure_count: 0,
            },
            ApiKey { 
                id: "key2".to_string(), 
                key: "test-key-2".to_string(),
                weight: 300, 
                max_requests_per_minute: 60,
                current_requests: 0,
                last_reset: chrono::Utc::now(),
                is_active: true,
                failure_count: 0,
            },
        ];
        
        toolkit.normalize_weights(&mut api_keys, 1000, "admin").await.unwrap();
        
        let total_weight: u32 = api_keys.iter().map(|k| k.weight).sum();
        assert_eq!(total_weight, 1000);
        
        // 检查比例是否保持
        assert_eq!(api_keys[0].weight * 2, api_keys[1].weight);
    }
}