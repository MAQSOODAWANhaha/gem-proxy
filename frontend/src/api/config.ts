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

// 创建 axios 实例
const api = axios.create({
  baseURL: '/api',
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json'
  }
})

// 响应拦截器
api.interceptors.response.use(
  (response) => response,
  (error) => {
    console.error('API 请求错误:', error)
    return Promise.reject(error)
  }
)

export const configApi = {
  // 获取当前配置
  async getConfig(): Promise<ProxyConfig> {
    // 由于后端还没有实现配置 API，这里返回模拟数据
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve({
          server: {
            host: "0.0.0.0",
            port: 8080,
            workers: 4,
            max_connections: 10000,
            tls: {
              enabled: false,
              cert_path: "config/cert.pem",
              key_path: "config/key.pem",
              acme: {
                enabled: true,
                domains: ["proxy.example.com"],
                email: "admin@example.com",
                directory_url: "https://acme-v02.api.letsencrypt.org/directory"
              }
            }
          },
          gemini: {
            base_url: "generativelanguage.googleapis.com:443",
            timeout_seconds: 30,
            api_keys: [
              {
                id: "key1",
                key: "AIzaSyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                weight: 100,
                max_requests_per_minute: 60
              },
              {
                id: "key2", 
                key: "AIzaSyYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY",
                weight: 100,
                max_requests_per_minute: 60
              }
            ]
          },
          auth: {
            enabled: true,
            jwt_secret: "your-super-secret-key-that-is-long-and-secure",
            rate_limit_per_minute: 100
          },
          metrics: {
            enabled: true,
            prometheus_port: 9090
          }
        })
      }, 500)
    })
  },

  // 更新配置
  async updateConfig(config: ProxyConfig): Promise<void> {
    // 模拟更新配置的 API 调用
    return new Promise((resolve) => {
      setTimeout(() => {
        console.log('配置已更新:', config)
        resolve()
      }, 500)
    })
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
      const response = await api.get('/weights/stats')
      return response.data
    } catch (error) {
      console.error('获取权重统计失败:', error)
      throw new Error('无法获取权重统计')
    }
  },

  // 获取权重分配详情
  async getWeightDistribution(): Promise<WeightStats> {
    try {
      const response = await api.get('/weights/distribution')
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
      const response = await api.put(`/weights/${keyId}`, request)
      return response.data
    } catch (error) {
      console.error('更新权重失败:', error)
      throw new Error('更新权重失败')
    }
  },

  // 批量更新权重
  async batchUpdateWeights(updates: BatchUpdateWeightRequest): Promise<void> {
    try {
      await api.post('/weights/batch', updates)
    } catch (error) {
      console.error('批量更新权重失败:', error)
      throw new Error('批量更新权重失败')
    }
  },

  // 智能权重重平衡
  async rebalanceWeights(): Promise<void> {
    try {
      await api.post('/weights/rebalance')
    } catch (error) {
      console.error('权重重平衡失败:', error)
      throw new Error('权重重平衡失败')
    }
  },

  // 获取权重优化建议
  async getWeightOptimization(): Promise<WeightOptimizationResponse> {
    try {
      const response = await api.get('/weights/optimize')
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