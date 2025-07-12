import axios from 'axios'
import type { ProxyConfig, HealthStatus } from '../types'

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
  }
}