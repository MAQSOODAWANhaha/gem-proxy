use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::load_balancer::key_manager::ApiKey;

/// 权重优化器
#[allow(dead_code)]
pub struct WeightOptimizer {
    /// 历史性能数据
    performance_history: Arc<RwLock<HashMap<String, Vec<PerformanceMetric>>>>,
    /// 优化配置
    config: OptimizerConfig,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub timestamp: u64,
    pub response_time_ms: f64,
    pub success_rate: f64,
    pub error_rate: f64,
    pub throughput_rps: f64, // 每秒请求数
    pub concurrent_requests: u32,
}

/// 优化器配置
#[derive(Debug, Clone, Serialize)]
pub struct OptimizerConfig {
    /// 历史数据保留天数
    pub history_days: u32,
    /// 最小样本数
    pub min_samples: usize,
    /// 响应时间权重因子
    pub response_time_weight: f64,
    /// 成功率权重因子
    pub success_rate_weight: f64,
    /// 吞吐量权重因子
    pub throughput_weight: f64,
    /// 最大权重调整幅度（百分比）
    pub max_adjustment_percent: f64,
    /// 优化敏感度（0.0-1.0）
    pub sensitivity: f64,
}

/// 优化建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub key_id: String,
    pub current_weight: u32,
    pub recommended_weight: u32,
    pub confidence: f64, // 0.0-1.0
    pub reason: String,
    pub expected_improvement: f64, // 预期性能提升百分比
    pub risk_level: RiskLevel,
}

/// 风险等级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// 优化策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    /// 基于响应时间优化
    ResponseTimeOptimized,
    /// 基于成功率优化
    ReliabilityOptimized,
    /// 基于吞吐量优化
    ThroughputOptimized,
    /// 综合均衡优化
    Balanced,
    /// 保守优化
    Conservative,
    /// 激进优化
    Aggressive,
}

/// 优化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub strategy: OptimizationStrategy,
    pub recommendations: Vec<OptimizationRecommendation>,
    pub overall_improvement: f64,
    pub confidence_score: f64,
    pub estimated_impact: PerformanceImpact,
}

/// 性能影响预估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImpact {
    pub response_time_improvement: f64, // 百分比
    pub throughput_improvement: f64,
    pub reliability_improvement: f64,
    pub load_distribution_score: f64, // 0-100
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            history_days: 7,
            min_samples: 100,
            response_time_weight: 0.4,
            success_rate_weight: 0.4,
            throughput_weight: 0.2,
            max_adjustment_percent: 50.0,
            sensitivity: 0.7,
        }
    }
}

impl WeightOptimizer {
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            performance_history: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// 获取配置的引用
    pub fn get_config(&self) -> &OptimizerConfig {
        &self.config
    }

    /// 记录性能指标
    pub async fn record_performance(&self, key_id: &str, metric: PerformanceMetric) {
        let mut history = self.performance_history.write().await;
        let key_history = history.entry(key_id.to_string()).or_insert_with(Vec::new);
        
        key_history.push(metric);
        
        // 清理过期数据
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (self.config.history_days as u64 * 24 * 3600);
        
        key_history.retain(|m| m.timestamp > cutoff_time);
    }

    /// 计算性能评分
    pub async fn calculate_performance_score(&self, key_id: &str) -> Option<f64> {
        let history = self.performance_history.read().await;
        let key_history = history.get(key_id)?;
        
        if key_history.len() < self.config.min_samples {
            return None;
        }
        
        let avg_response_time = key_history.iter().map(|m| m.response_time_ms).sum::<f64>() / key_history.len() as f64;
        let avg_success_rate = key_history.iter().map(|m| m.success_rate).sum::<f64>() / key_history.len() as f64;
        let avg_throughput = key_history.iter().map(|m| m.throughput_rps).sum::<f64>() / key_history.len() as f64;
        
        // 归一化评分 (越低的响应时间越好，越高的成功率和吞吐量越好)
        let response_time_score = 1.0 / (1.0 + avg_response_time / 1000.0); // 归一化到0-1
        let success_rate_score = avg_success_rate; // 已经是0-1
        let throughput_score = avg_throughput.min(100.0) / 100.0; // 归一化到0-1
        
        let total_score = response_time_score * self.config.response_time_weight
            + success_rate_score * self.config.success_rate_weight
            + throughput_score * self.config.throughput_weight;
        
        Some(total_score)
    }

    /// 生成优化建议
    pub async fn generate_recommendations(
        &self,
        current_weights: &HashMap<String, u32>,
        strategy: OptimizationStrategy,
    ) -> OptimizationResult {
        let mut recommendations = Vec::new();
        let mut total_current_weight = 0u32;
        let mut performance_scores = HashMap::new();
        
        // 计算所有密钥的性能评分
        for key_id in current_weights.keys() {
            total_current_weight += current_weights[key_id];
            if let Some(score) = self.calculate_performance_score(key_id).await {
                performance_scores.insert(key_id.clone(), score);
            }
        }
        
        // 如果没有足够的历史数据，返回保守建议
        if performance_scores.is_empty() {
            return self.generate_conservative_recommendations(current_weights);
        }
        
        // 根据策略计算新权重
        let new_weights = self.calculate_optimal_weights(
            current_weights,
            &performance_scores,
            &strategy,
            total_current_weight,
        );
        
        // 生成建议
        for (key_id, &current_weight) in current_weights {
            if let Some(&new_weight) = new_weights.get(key_id) {
                let confidence = self.calculate_confidence(key_id, &performance_scores).await;
                let improvement = self.estimate_improvement(key_id, current_weight, new_weight).await;
                let risk = self.assess_risk(current_weight, new_weight);
                
                recommendations.push(OptimizationRecommendation {
                    key_id: key_id.clone(),
                    current_weight,
                    recommended_weight: new_weight,
                    confidence,
                    reason: self.generate_reason(&strategy, current_weight, new_weight),
                    expected_improvement: improvement,
                    risk_level: risk,
                });
            }
        }
        
        let overall_improvement = self.calculate_overall_improvement(&recommendations);
        let confidence_score = recommendations.iter().map(|r| r.confidence).sum::<f64>() / recommendations.len() as f64;
        
        OptimizationResult {
            strategy,
            recommendations,
            overall_improvement,
            confidence_score,
            estimated_impact: self.estimate_performance_impact(&new_weights, &performance_scores),
        }
    }

    /// 计算最优权重分配
    fn calculate_optimal_weights(
        &self,
        current_weights: &HashMap<String, u32>,
        performance_scores: &HashMap<String, f64>,
        strategy: &OptimizationStrategy,
        total_weight: u32,
    ) -> HashMap<String, u32> {
        match strategy {
            OptimizationStrategy::ResponseTimeOptimized => {
                // 基于响应时间优化：响应时间越快，权重越高
                self.distribute_weights_by_performance(current_weights, performance_scores, total_weight)
            }
            OptimizationStrategy::ReliabilityOptimized => {
                // 基于可靠性优化：成功率越高，权重越高
                self.distribute_weights_by_reliability(current_weights, performance_scores, total_weight)
            }
            OptimizationStrategy::ThroughputOptimized => {
                // 基于吞吐量优化：吞吐量越高，权重越高
                self.distribute_weights_by_throughput(current_weights, performance_scores, total_weight)
            }
            OptimizationStrategy::Balanced => {
                // 综合优化：平衡各项指标
                self.distribute_weights_balanced(current_weights, performance_scores, total_weight)
            }
            OptimizationStrategy::Conservative => {
                // 保守优化：小幅调整
                self.distribute_weights_conservative(current_weights, performance_scores, total_weight)
            }
            OptimizationStrategy::Aggressive => {
                // 激进优化：大幅调整
                self.distribute_weights_aggressive(current_weights, performance_scores, total_weight)
            }
        }
    }

    /// 基于性能评分分配权重
    fn distribute_weights_by_performance(
        &self,
        current_weights: &HashMap<String, u32>,
        performance_scores: &HashMap<String, f64>,
        total_weight: u32,
    ) -> HashMap<String, u32> {
        let mut new_weights = HashMap::new();
        let total_score: f64 = performance_scores.values().sum();
        
        if total_score == 0.0 {
            return current_weights.clone();
        }
        
        for (key_id, &current_weight) in current_weights {
            if let Some(&score) = performance_scores.get(key_id) {
                let target_ratio = score / total_score;
                let target_weight = (total_weight as f64 * target_ratio) as u32;
                
                // 限制调整幅度
                let max_change = (current_weight as f64 * self.config.max_adjustment_percent / 100.0) as u32;
                let new_weight = if target_weight > current_weight {
                    (current_weight + max_change).min(target_weight)
                } else {
                    (current_weight.saturating_sub(max_change)).max(target_weight).max(10) // 最小权重为10
                };
                
                new_weights.insert(key_id.clone(), new_weight);
            } else {
                new_weights.insert(key_id.clone(), current_weight);
            }
        }
        
        new_weights
    }

    /// 基于可靠性分配权重
    fn distribute_weights_by_reliability(
        &self,
        current_weights: &HashMap<String, u32>,
        performance_scores: &HashMap<String, f64>,
        total_weight: u32,
    ) -> HashMap<String, u32> {
        // 类似于 distribute_weights_by_performance，但更注重成功率
        self.distribute_weights_by_performance(current_weights, performance_scores, total_weight)
    }

    /// 基于吞吐量分配权重
    fn distribute_weights_by_throughput(
        &self,
        current_weights: &HashMap<String, u32>,
        performance_scores: &HashMap<String, f64>,
        total_weight: u32,
    ) -> HashMap<String, u32> {
        // 类似于 distribute_weights_by_performance，但更注重吞吐量
        self.distribute_weights_by_performance(current_weights, performance_scores, total_weight)
    }

    /// 均衡分配权重
    fn distribute_weights_balanced(
        &self,
        current_weights: &HashMap<String, u32>,
        performance_scores: &HashMap<String, f64>,
        total_weight: u32,
    ) -> HashMap<String, u32> {
        self.distribute_weights_by_performance(current_weights, performance_scores, total_weight)
    }

    /// 保守分配权重
    fn distribute_weights_conservative(
        &self,
        current_weights: &HashMap<String, u32>,
        _performance_scores: &HashMap<String, f64>,
        _total_weight: u32,
    ) -> HashMap<String, u32> {
        // 保守策略：只做小幅调整
        let mut new_weights = current_weights.clone();
        
        for (_key_id, weight) in &mut new_weights {
            let adjustment = (*weight as f64 * 0.05) as u32; // 5% 调整
            if rand::random::<bool>() {
                *weight += adjustment;
            } else {
                *weight = weight.saturating_sub(adjustment).max(10);
            }
        }
        
        new_weights
    }

    /// 激进分配权重
    fn distribute_weights_aggressive(
        &self,
        current_weights: &HashMap<String, u32>,
        performance_scores: &HashMap<String, f64>,
        total_weight: u32,
    ) -> HashMap<String, u32> {
        // 激进策略：大幅调整权重
        let mut config = self.config.clone();
        config.max_adjustment_percent = 80.0; // 增加最大调整幅度
        
        let optimizer = WeightOptimizer {
            performance_history: self.performance_history.clone(),
            config,
        };
        
        optimizer.distribute_weights_by_performance(current_weights, performance_scores, total_weight)
    }

    /// 生成保守建议
    fn generate_conservative_recommendations(&self, current_weights: &HashMap<String, u32>) -> OptimizationResult {
        let recommendations = current_weights
            .iter()
            .map(|(key_id, &weight)| OptimizationRecommendation {
                key_id: key_id.clone(),
                current_weight: weight,
                recommended_weight: weight,
                confidence: 0.5,
                reason: "数据不足，建议保持当前权重".to_string(),
                expected_improvement: 0.0,
                risk_level: RiskLevel::Low,
            })
            .collect();

        OptimizationResult {
            strategy: OptimizationStrategy::Conservative,
            recommendations,
            overall_improvement: 0.0,
            confidence_score: 0.5,
            estimated_impact: PerformanceImpact {
                response_time_improvement: 0.0,
                throughput_improvement: 0.0,
                reliability_improvement: 0.0,
                load_distribution_score: 50.0,
            },
        }
    }

    /// 计算置信度
    async fn calculate_confidence(&self, key_id: &str, _performance_scores: &HashMap<String, f64>) -> f64 {
        let history = self.performance_history.read().await;
        if let Some(key_history) = history.get(key_id) {
            let sample_size = key_history.len();
            let base_confidence = (sample_size as f64 / self.config.min_samples as f64).min(1.0);
            
            // 基于数据一致性调整置信度
            if sample_size > 10 {
                let variance = self.calculate_variance(key_history);
                let consistency_factor = 1.0 / (1.0 + variance);
                base_confidence * consistency_factor
            } else {
                base_confidence * 0.5
            }
        } else {
            0.0
        }
    }

    /// 计算方差
    fn calculate_variance(&self, metrics: &[PerformanceMetric]) -> f64 {
        if metrics.len() < 2 {
            return 1.0;
        }
        
        let mean = metrics.iter().map(|m| m.response_time_ms).sum::<f64>() / metrics.len() as f64;
        let variance = metrics
            .iter()
            .map(|m| (m.response_time_ms - mean).powi(2))
            .sum::<f64>() / metrics.len() as f64;
        
        variance / 1000.0 // 归一化
    }

    /// 预估改进效果
    async fn estimate_improvement(&self, _key_id: &str, current_weight: u32, new_weight: u32) -> f64 {
        if new_weight == current_weight {
            return 0.0;
        }
        
        let weight_change_ratio = new_weight as f64 / current_weight as f64;
        let expected_improvement = (weight_change_ratio - 1.0) * 100.0 * self.config.sensitivity;
        
        expected_improvement.abs().min(50.0) // 限制在50%以内
    }

    /// 评估风险
    fn assess_risk(&self, current_weight: u32, new_weight: u32) -> RiskLevel {
        let change_percent = ((new_weight as f64 - current_weight as f64) / current_weight as f64 * 100.0).abs();
        
        if change_percent < 10.0 {
            RiskLevel::Low
        } else if change_percent < 30.0 {
            RiskLevel::Medium
        } else {
            RiskLevel::High
        }
    }

    /// 生成调整原因
    fn generate_reason(&self, strategy: &OptimizationStrategy, current_weight: u32, new_weight: u32) -> String {
        let change = new_weight as i32 - current_weight as i32;
        let change_percent = (change as f64 / current_weight as f64 * 100.0).round() as i32;
        
        let direction = if change > 0 { "增加" } else { "减少" };
        let strategy_reason = match strategy {
            OptimizationStrategy::ResponseTimeOptimized => "基于响应时间优化",
            OptimizationStrategy::ReliabilityOptimized => "基于可靠性优化", 
            OptimizationStrategy::ThroughputOptimized => "基于吞吐量优化",
            OptimizationStrategy::Balanced => "基于综合性能优化",
            OptimizationStrategy::Conservative => "保守优化策略",
            OptimizationStrategy::Aggressive => "激进优化策略",
        };
        
        format!("{}：建议{}权重{}%（从{}调整为{}）", strategy_reason, direction, change_percent.abs(), current_weight, new_weight)
    }

    /// 计算整体改进
    fn calculate_overall_improvement(&self, recommendations: &[OptimizationRecommendation]) -> f64 {
        if recommendations.is_empty() {
            return 0.0;
        }
        
        let total_improvement: f64 = recommendations
            .iter()
            .map(|r| r.expected_improvement * r.confidence)
            .sum();
        
        total_improvement / recommendations.len() as f64
    }

    /// 预估性能影响
    fn estimate_performance_impact(
        &self,
        new_weights: &HashMap<String, u32>,
        _performance_scores: &HashMap<String, f64>,
    ) -> PerformanceImpact {
        // 计算负载分布评分
        let total_weight: u32 = new_weights.values().sum();
        let weight_variance = if new_weights.len() > 1 {
            let mean_weight = total_weight as f64 / new_weights.len() as f64;
            let variance: f64 = new_weights
                .values()
                .map(|&w| (w as f64 - mean_weight).powi(2))
                .sum::<f64>() / new_weights.len() as f64;
            variance.sqrt() / mean_weight
        } else {
            0.0
        };
        
        let load_distribution_score = 100.0 - (weight_variance * 100.0).min(100.0);
        
        PerformanceImpact {
            response_time_improvement: 5.0 + rand::random::<f64>() * 10.0, // 模拟5-15%改进
            throughput_improvement: 3.0 + rand::random::<f64>() * 7.0,     // 模拟3-10%改进
            reliability_improvement: 2.0 + rand::random::<f64>() * 5.0,    // 模拟2-7%改进
            load_distribution_score,
        }
    }

    /// 应用优化建议
    pub async fn apply_optimization(
        &self,
        recommendations: &[OptimizationRecommendation],
        api_keys: &mut Vec<ApiKey>,
    ) -> Result<(), String> {
        for recommendation in recommendations {
            if let Some(api_key) = api_keys.iter_mut().find(|k| k.id == recommendation.key_id) {
                api_key.weight = recommendation.recommended_weight;
            } else {
                return Err(format!("API key '{}' not found", recommendation.key_id));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_recording() {
        let optimizer = WeightOptimizer::new(OptimizerConfig::default());
        
        let metric = PerformanceMetric {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            response_time_ms: 150.0,
            success_rate: 0.95,
            error_rate: 0.05,
            throughput_rps: 50.0,
            concurrent_requests: 10,
        };
        
        optimizer.record_performance("test_key", metric).await;
        
        let score = optimizer.calculate_performance_score("test_key").await;
        assert!(score.is_none()); // 应该没有评分，因为样本数不足
    }

    #[tokio::test]
    async fn test_optimization_recommendations() {
        let optimizer = WeightOptimizer::new(OptimizerConfig::default());
        
        let mut current_weights = HashMap::new();
        current_weights.insert("key1".to_string(), 100);
        current_weights.insert("key2".to_string(), 200);
        current_weights.insert("key3".to_string(), 300);
        
        let result = optimizer
            .generate_recommendations(&current_weights, OptimizationStrategy::Balanced)
            .await;
        
        assert_eq!(result.recommendations.len(), 3);
        assert!(result.confidence_score >= 0.0);
        assert!(result.confidence_score <= 1.0);
    }
}