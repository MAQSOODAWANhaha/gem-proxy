import axios from 'axios'
import type { 
  ProxyConfig, 
  HealthStatus, 
  WeightStatsResponse,
  WeightStats,
  WeightOptimizationResponse,
  UpdateWeightRequest,
  BatchUpdateWeightRequest,
  ApiResponse
} from '../types'

// 使用全局axios实例，不需要单独创建

export const configApi = {
  // 获取当前配置
  async getConfig(): Promise<ProxyConfig> {
    try {
      const response = await axios.get('/api/config')
      return response.data.data || response.data
    } catch (error) {
      console.error('获取配置失败:', error)
      throw new Error('无法获取配置')
    }
  },

  // 更新配置
  async updateConfig(config: ProxyConfig): Promise<void> {
    try {
      await axios.put('/api/config', config)
    } catch (error) {
      console.error('更新配置失败:', error)
      throw new Error('更新配置失败')
    }
  },

  // 获取健康状态
  async getHealthStatus(): Promise<HealthStatus> {
    try {
      const response = await axios.get('/health')
      return response.data
    } catch (error) {
      throw new Error('无法获取健康状态')
    }
  },

  // 获取监控指标
  async getMetrics(): Promise<string> {
    try {
      const response = await axios.get('/metrics', {
        responseType: 'text'
      })
      return response.data
    } catch (error) {
      throw new Error('无法获取监控指标')
    }
  },

  // 测试 API 密钥
  async testApiKey(apiKey: string): Promise<boolean> {
    // 模拟测试 API 密钥
    return new Promise((resolve) => {
      setTimeout(() => {
        // 简单的模拟逻辑
        resolve(apiKey.startsWith('AIza') && apiKey.length > 20)
      }, 1000)
    })
  },

  // 权重管理 API
  
  // 获取权重统计
  async getWeightStats(): Promise<ApiResponse<WeightStatsResponse>> {
    try {
      const response = await axios.get('/api/weights/stats')
      return response.data
    } catch (error) {
      console.error('获取权重统计失败:', error)
      throw new Error('无法获取权重统计')
    }
  },

  // 获取权重分配详情
  async getWeightDistribution(): Promise<WeightStats> {
    try {
      const response = await axios.get('/api/weights/distribution')
      return response.data.data || response.data
    } catch (error) {
      console.error('获取权重分配失败:', error)
      throw new Error('无法获取权重分配')
    }
  },

  // 更新单个密钥权重
  async updateKeyWeight(keyId: string, weight: number): Promise<ApiResponse<void>> {
    try {
      const request: UpdateWeightRequest = { weight }
      const response = await axios.put(`/api/weights/${keyId}`, request)
      return response.data
    } catch (error) {
      console.error('更新权重失败:', error)
      throw new Error('更新权重失败')
    }
  },

  // 批量更新权重
  async batchUpdateWeights(updates: BatchUpdateWeightRequest): Promise<void> {
    try {
      await axios.post('/api/weights/batch', updates)
    } catch (error) {
      console.error('批量更新权重失败:', error)
      throw new Error('批量更新权重失败')
    }
  },

  // 智能权重重平衡
  async rebalanceWeights(): Promise<void> {
    try {
      await axios.post('/api/weights/rebalance')
    } catch (error) {
      console.error('权重重平衡失败:', error)
      throw new Error('权重重平衡失败')
    }
  },

  // 获取权重优化建议
  async getWeightOptimization(): Promise<WeightOptimizationResponse> {
    try {
      const response = await axios.get('/api/weights/optimize')
      return response.data.data || response.data
    } catch (error) {
      console.error('获取优化建议失败:', error)
      throw new Error('无法获取优化建议')
    }
  }
}

// 导出权重管理相关函数
export const getWeightStats = configApi.getWeightStats
export const updateKeyWeight = configApi.updateKeyWeight
export const batchUpdateWeights = configApi.batchUpdateWeights
export const getOptimizationSuggestions = configApi.getWeightOptimization
export const rebalanceWeights = configApi.rebalanceWeights
export const getWeightDistribution = configApi.getWeightDistribution