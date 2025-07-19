<template>
  <AppPage title="监控指标" description="实时查看系统状态和 Prometheus 指标数据">
    <template #actions>
      <el-button 
        type="primary" 
        @click="refreshMetrics"
        :loading="loading"
      >
        刷新指标
      </el-button>
    </template>
    
    <!-- 实时状态概览 -->
    <ContentCard title="服务状态概览" :span="24">
      <el-row :gutter="24">
        <StatCard 
          :span="8"
          title="服务状态"
          :value="healthStatus?.status || 'unknown'"
          :value-style="{ color: healthStatusColor }"
          :icon="Monitor"
          icon-color="#409eff"
        />
        <StatCard 
          :span="8"
          title="最后检查"
          :value="lastCheckTime"
          :value-style="{ fontSize: '16px' }"
          :icon="Clock"
          icon-color="#67c23a"
        />
        <StatCard 
          :span="8"
          title="检查项目"
          :value="healthCheckData.length"
          :icon="List"
          icon-color="#e6a23c"
        />
      </el-row>
    </ContentCard>

    <!-- 健康检查详情 -->
    <ContentCard title="健康检查详情" :span="24">
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
    </ContentCard>

    <!-- Prometheus 指标 -->
    <ContentCard title="Prometheus 指标" :span="24">
      <template #actions>
        <el-button 
          size="small" 
          @click="copyMetrics"
          :disabled="!metricsData"
        >
          复制指标
        </el-button>
      </template>
      
      <el-input
        v-if="metricsData"
        v-model="metricsData"
        type="textarea"
        :rows="20"
        readonly
        class="metrics-textarea"
      />
      
      <el-empty v-else description="暂无指标数据" />
    </ContentCard>

    <!-- 访问链接 -->
    <ContentCard title="监控链接" :span="24">
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
    </ContentCard>
  </AppPage>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Monitor, Clock, List } from '@element-plus/icons-vue'
import { useConfigStore } from '../stores/config'
import { configApi } from '../api/config'
import type { HealthStatus } from '../types'
import AppPage from '../components/layout/AppPage.vue'
import ContentCard from '../components/layout/ContentCard.vue'
import StatCard from '../components/layout/StatCard.vue'

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
/* 使用全局样式和工具类，大幅简化自定义样式 */

.metrics-textarea {
  font-family: 'Courier New', monospace;
  font-size: var(--font-size-extra-small);
}

/* 统计卡片和卡片样式已由组件统一管理 */
</style>