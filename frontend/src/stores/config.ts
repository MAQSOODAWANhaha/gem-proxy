import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { ProxyConfig, ApiKey, HealthStatus } from '../types'
import { configApi } from '../api/config'

export const useConfigStore = defineStore('config', () => {
  // 状态
  const config = ref<ProxyConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const healthStatus = ref<HealthStatus | null>(null)

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

  return {
    // 状态
    config,
    loading,
    error,
    healthStatus,
    
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
    checkHealth
  }
})