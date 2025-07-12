<template>
  <div class="monitoring">
    <h1>监控指标</h1>
    
    <!-- 实时状态 -->
    <el-row :gutter="20" class="status-row">
      <el-col :span="8">
        <el-card class="status-card">
          <el-statistic 
            title="服务状态" 
            :value="healthStatus?.status || 'unknown'"
            :value-style="{ color: healthStatusColor }"
          />
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card class="status-card">
          <el-statistic 
            title="最后检查" 
            :value="lastCheckTime"
            :value-style="{ fontSize: '16px' }"
          />
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card class="status-card">
          <el-button 
            type="primary" 
            @click="refreshMetrics"
            :loading="loading"
            style="width: 100%"
          >
            刷新指标
          </el-button>
        </el-card>
      </el-col>
    </el-row>

    <!-- 健康检查详情 -->
    <el-card title="健康检查详情" class="health-card">
      <el-table 
        v-if="healthStatus?.checks"
        :data="healthCheckData" 
        style="width: 100%"
      >
        <el-table-column prop="name" label="检查项" width="150" />
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">
              {{ getStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="message" label="消息" />
        <el-table-column prop="duration_ms" label="耗时(ms)" width="100" />
      </el-table>
      
      <el-empty v-else description="暂无健康检查数据" />
    </el-card>

    <!-- Prometheus 指标 -->
    <el-card title="Prometheus 指标" class="metrics-card">
      <div class="metrics-header">
        <span>原始指标数据</span>
        <el-button 
          size="small" 
          @click="copyMetrics"
          :disabled="!metricsData"
        >
          复制指标
        </el-button>
      </div>
      
      <el-input
        v-if="metricsData"
        v-model="metricsData"
        type="textarea"
        :rows="20"
        readonly
        class="metrics-textarea"
      />
      
      <el-empty v-else description="暂无指标数据" />
    </el-card>

    <!-- 访问链接 -->
    <el-card title="监控链接" class="links-card">
      <el-descriptions :column="1" border>
        <el-descriptions-item label="健康检查">
          <el-link 
            :href="healthUrl" 
            target="_blank" 
            type="primary"
          >
            {{ healthUrl }}
          </el-link>
        </el-descriptions-item>
        <el-descriptions-item label="Prometheus 指标">
          <el-link 
            :href="metricsUrl" 
            target="_blank" 
            type="primary"
          >
            {{ metricsUrl }}
          </el-link>
        </el-descriptions-item>
      </el-descriptions>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessage } from 'element-plus'
import { useConfigStore } from '../stores/config'
import { configApi } from '../api/config'
import type { HealthStatus } from '../types'

const configStore = useConfigStore()

// 状态
const loading = ref(false)
const healthStatus = ref<HealthStatus | null>(null)
const metricsData = ref<string>('')

// 计算属性
const healthStatusColor = computed(() => {
  if (!healthStatus.value) return '#909399'
  const status = healthStatus.value.status
  if (status === 'healthy') return '#67c23a'
  if (status === 'degraded') return '#e6a23c'
  return '#f56c6c'
})

const lastCheckTime = computed(() => {
  if (!healthStatus.value) return '未知'
  return new Date(healthStatus.value.timestamp * 1000).toLocaleString()
})

const healthCheckData = computed(() => {
  if (!healthStatus.value?.checks) return []
  return Object.entries(healthStatus.value.checks).map(([name, check]) => ({
    name: getCheckDisplayName(name),
    ...check
  }))
})

const healthUrl = computed(() => {
  const port = configStore.config?.metrics.prometheus_port || 9090
  return `http://localhost:${port}/health`
})

const metricsUrl = computed(() => {
  const port = configStore.config?.metrics.prometheus_port || 9090
  return `http://localhost:${port}/metrics`
})

// 方法
function getCheckDisplayName(name: string): string {
  const nameMap: Record<string, string> = {
    'system': '系统检查',
    'configuration': '配置检查',
    'api_keys': 'API 密钥检查'
  }
  return nameMap[name] || name
}

function getStatusType(status: string) {
  switch (status) {
    case 'healthy': return 'success'
    case 'degraded': return 'warning'
    default: return 'danger'
  }
}

function getStatusText(status: string) {
  switch (status) {
    case 'healthy': return '健康'
    case 'degraded': return '降级'
    default: return '异常'
  }
}

async function refreshMetrics() {
  loading.value = true
  try {
    // 获取健康状态
    healthStatus.value = await configApi.getHealthStatus()
    
    // 获取指标数据
    metricsData.value = await configApi.getMetrics()
    
    ElMessage.success('指标数据刷新成功')
  } catch (error) {
    console.error('刷新指标失败:', error)
    ElMessage.error('刷新指标失败')
  } finally {
    loading.value = false
  }
}

async function copyMetrics() {
  if (!metricsData.value) return
  
  try {
    await navigator.clipboard.writeText(metricsData.value)
    ElMessage.success('指标数据已复制到剪贴板')
  } catch (error) {
    ElMessage.error('复制失败，请手动复制')
  }
}

// 定时刷新
let refreshInterval: number | null = null

onMounted(() => {
  refreshMetrics()
  // 每30秒刷新一次
  refreshInterval = window.setInterval(refreshMetrics, 30000)
})

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
  }
})
</script>

<style scoped>
.monitoring {
  max-width: 1000px;
}

.status-row {
  margin-bottom: 24px;
}

.status-card {
  text-align: center;
}

.health-card,
.metrics-card,
.links-card {
  margin-bottom: 24px;
}

.metrics-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.metrics-textarea {
  font-family: 'Courier New', monospace;
  font-size: 12px;
}

:deep(.el-statistic__content) {
  font-size: 20px;
  font-weight: 600;
}

:deep(.el-card__header) {
  font-weight: 600;
  color: #1f2937;
}
</style>