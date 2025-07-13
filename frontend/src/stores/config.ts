import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { 
  ProxyConfig, 
  ApiKey, 
  HealthStatus, 
  WeightStatsResponse,
  WeightStats,
  WeightOptimizationResponse
} from '../types'
import { configApi } from '../api/config'

export const useConfigStore = defineStore('config', () => {
  // 状态
  const config = ref<ProxyConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const healthStatus = ref<HealthStatus | null>(null)
  
  // 权重管理状态
  const weightStats = ref<WeightStatsResponse | null>(null)
  const weightDistribution = ref<WeightStats | null>(null)
  const weightOptimization = ref<WeightOptimizationResponse | null>(null)
  const weightLoading = ref(false)

  // 计算属性
  const isConfigLoaded = computed(() => config.value !== null)
  const apiKeysCount = computed(() => config.value?.gemini.api_keys.length || 0)
  const activeApiKeys = computed(() => 
    config.value?.gemini.api_keys.filter(key => key.weight > 0) || []
  )

  // 操作
  async function loadConfig() {
    loading.value = true
    error.value = null
    try {
      config.value = await configApi.getConfig()
    } catch (err) {
      error.value = err instanceof Error ? err.message : '加载配置失败'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function updateConfig(newConfig: ProxyConfig) {
    loading.value = true
    error.value = null
    try {
      await configApi.updateConfig(newConfig)
      config.value = newConfig
    } catch (err) {
      error.value = err instanceof Error ? err.message : '更新配置失败'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function addApiKey(apiKey: Omit<ApiKey, 'id'>) {
    if (!config.value) return
    
    const newApiKey: ApiKey = {
      id: `key_${Date.now()}`,
      ...apiKey
    }
    
    const updatedConfig = {
      ...config.value,
      gemini: {
        ...config.value.gemini,
        api_keys: [...config.value.gemini.api_keys, newApiKey]
      }
    }
    
    await updateConfig(updatedConfig)
  }

  async function removeApiKey(keyId: string) {
    if (!config.value) return
    
    const updatedConfig = {
      ...config.value,
      gemini: {
        ...config.value.gemini,
        api_keys: config.value.gemini.api_keys.filter(key => key.id !== keyId)
      }
    }
    
    await updateConfig(updatedConfig)
  }

  async function updateApiKey(keyId: string, updates: Partial<ApiKey>) {
    if (!config.value) return
    
    const updatedConfig = {
      ...config.value,
      gemini: {
        ...config.value.gemini,
        api_keys: config.value.gemini.api_keys.map(key =>
          key.id === keyId ? { ...key, ...updates } : key
        )
      }
    }
    
    await updateConfig(updatedConfig)
  }

  async function checkHealth() {
    try {
      healthStatus.value = await configApi.getHealthStatus()
    } catch (err) {
      console.error('健康检查失败:', err)
    }
  }

  // 权重管理方法
  async function loadWeightStats() {
    weightLoading.value = true
    try {
      const response = await configApi.getWeightStats()
      if (response.success && response.data) {
        weightStats.value = response.data
      }
    } catch (err) {
      console.error('加载权重统计失败:', err)
      throw err
    } finally {
      weightLoading.value = false
    }
  }

  async function loadWeightDistribution() {
    weightLoading.value = true
    try {
      weightDistribution.value = await configApi.getWeightDistribution()
    } catch (err) {
      console.error('加载权重分配失败:', err)
      throw err
    } finally {
      weightLoading.value = false
    }
  }

  async function updateKeyWeight(keyId: string, weight: number) {
    weightLoading.value = true
    try {
      await configApi.updateKeyWeight(keyId, weight)
      // 更新本地配置
      if (config.value) {
        const apiKey = config.value.gemini.api_keys.find(k => k.id === keyId)
        if (apiKey) {
          apiKey.weight = weight
        }
      }
      // 刷新权重统计
      await loadWeightStats()
    } catch (err) {
      console.error('更新权重失败:', err)
      throw err
    } finally {
      weightLoading.value = false
    }
  }

  async function batchUpdateWeights(updates: Array<{key_id: string, weight: number}>) {
    weightLoading.value = true
    try {
      await configApi.batchUpdateWeights({ updates })
      // 更新本地配置
      if (config.value) {
        updates.forEach(update => {
          const apiKey = config.value!.gemini.api_keys.find(k => k.id === update.key_id)
          if (apiKey) {
            apiKey.weight = update.weight
          }
        })
      }
      // 刷新权重统计
      await loadWeightStats()
    } catch (err) {
      console.error('批量更新权重失败:', err)
      throw err
    } finally {
      weightLoading.value = false
    }
  }

  async function rebalanceWeights() {
    weightLoading.value = true
    try {
      await configApi.rebalanceWeights()
      // 重新加载配置和权重统计
      await Promise.all([loadConfig(), loadWeightStats()])
    } catch (err) {
      console.error('权重重平衡失败:', err)
      throw err
    } finally {
      weightLoading.value = false
    }
  }

  async function loadWeightOptimization() {
    weightLoading.value = true
    try {
      weightOptimization.value = await configApi.getWeightOptimization()
    } catch (err) {
      console.error('加载优化建议失败:', err)
      throw err
    } finally {
      weightLoading.value = false
    }
  }

  return {
    // 状态
    config,
    loading,
    error,
    healthStatus,
    
    // 权重管理状态
    weightStats,
    weightDistribution,
    weightOptimization,
    weightLoading,
    
    // 计算属性
    isConfigLoaded,
    apiKeysCount,
    activeApiKeys,
    
    // 操作
    loadConfig,
    updateConfig,
    addApiKey,
    removeApiKey,
    updateApiKey,
    checkHealth,
    
    // 权重管理操作
    loadWeightStats,
    loadWeightDistribution,
    updateKeyWeight,
    batchUpdateWeights,
    rebalanceWeights,
    loadWeightOptimization
  }
})