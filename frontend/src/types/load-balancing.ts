// 负载均衡相关类型定义

export interface ApiKey {
  id: string
  weight: number
  enabled: boolean
}

export interface OptimizationRecommendation {
  key_id: string
  current_weight: number
  recommended_weight: number
  expected_improvement: number
  risk_level: 'Low' | 'Medium' | 'High'
  reason: string
  confidence: number
}

export interface OptimizationResult {
  strategy: string
  confidence_score: number
  overall_improvement: number
  recommendations: OptimizationRecommendation[]
}

export interface OptimizationStrategy {
  value: string
  label: string
}

export interface AuditRecord {
  id: string
  timestamp: number
  operator: string
  operation_type: 'Manual' | 'Intelligent' | 'Batch' | 'Rollback' | 'Automatic'
  target_key_id: string
  old_weight: number
  new_weight: number
  reason: string
  source: 'WebUI' | 'API' | 'ConfigFile' | 'Optimizer' | 'Monitor'
}

export interface Snapshot {
  snapshot_id: string
  description: string
  created_by: string
  timestamp: number
}

export interface BatchEditForm {
  operation: 'set' | 'increase' | 'decrease' | 'multiply'
  value: number
  targetKeys: string[]
  reason: string
}

export interface AuditPagination {
  page: number
  size: number
  total: number
}