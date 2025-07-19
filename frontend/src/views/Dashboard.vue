<template>
  <AppPage title="控制台" description="系统监控与权重管理中心">
    <template #actions>
      <el-button type="primary" @click="refreshHealth" :loading="configStore.loading">
        <el-icon><Refresh /></el-icon>
        刷新数据
      </el-button>
    </template>
    
    <!-- 权重状态概览 -->
    <ContentCard title="权重状态概览" :span="24">
      <el-row :gutter="32">
        <StatCard 
          :span="6"
          title="总权重"
          :value="weightStats.total_weight || 0"
          :value-style="{ color: '#409eff' }"
          :icon="ScaleToOriginal"
          icon-color="#409eff"
        />
        <StatCard 
          :span="6"
          title="活跃密钥"
          :value="weightStats.active_keys_count || 0"
          :value-style="{ color: '#67c23a' }"
          :icon="Connection"
          icon-color="#67c23a"
        />
        <StatCard 
          :span="6"
          title="均衡度"
          :value="`${Math.round(weightStats.load_balance_effectiveness || 0)}%`"
          :value-style="{ color: '#e6a23c' }"
          :icon="TrendCharts"
          icon-color="#e6a23c"
        />
        <StatCard 
          :span="6"
          title="权重状态"
          :value="weightHealthText"
          :value-style="{ color: weightHealthColor }"
          :icon="isWeightHealthy ? CircleCheckFilled : WarningFilled"
          :icon-color="weightHealthColor"
        />
      </el-row>
    </ContentCard>
    
    <!-- 权重监控图表 -->
    <ContentCard title="权重分布图表" :span="12">
      <div style="height: 300px; width: 100%;">
        <WeightDistributionChart />
      </div>
    </ContentCard>
    
    <ContentCard title="实时权重监控" :span="12">
      <div style="height: 300px; width: 100%;">
        <WeightRealTimeMonitor />
      </div>
    </ContentCard>
    
    <!-- 权重趋势分析 -->
    <ContentCard title="权重趋势分析" :span="24">
      <div style="height: 400px; width: 100%;">
        <WeightTrendChart />
      </div>
    </ContentCard>
    
    <!-- 系统状态统计 -->
    <ContentCard title="系统状态统计" :span="24">
      <el-row :gutter="32">
        <StatCard 
          :span="6"
          title="API 密钥"
          :value="apiKeysCount"
          :value-style="{ color: '#1890ff' }"
          :icon="Key"
          icon-color="#1890ff"
        />
        <StatCard 
          :span="6"
          title="活跃密钥"
          :value="activeKeysCount"
          :value-style="{ color: '#52c41a' }"
          :icon="CircleCheckFilled"
          icon-color="#52c41a"
        />
        <StatCard 
          :span="6"
          title="服务端口"
          :value="serverPort"
          :value-style="{ color: '#722ed1' }"
          :icon="Monitor"
          icon-color="#722ed1"
        />
        <StatCard 
          :span="6"
          title="服务状态"
          :value="healthStatusText"
          :value-style="{ color: healthStatusColor }"
          :icon="isHealthy ? CircleCheckFilled : CircleCloseFilled"
          :icon-color="healthStatusColor"
        />
      </el-row>
    </ContentCard>
    
    <!-- 系统详情 -->
    <ContentCard title="健康检查" :span="12">
      <template #actions>
        <el-button 
          type="primary" 
          size="small" 
          @click="refreshHealth"
          :loading="configStore.loading"
        >
          <el-icon><Refresh /></el-icon>
          刷新
        </el-button>
      </template>
      
      <div v-if="configStore.healthStatus">
        <div 
          v-for="(check, name) in configStore.healthStatus.checks" 
          :key="name"
          class="flex justify-between items-center mb-medium"
        >
          <div class="flex items-center gap-small">
            <el-icon :color="getCheckStatusColor(check.status)">
              <CircleCheckFilled v-if="check.status === 'healthy'" />
              <WarningFilled v-else-if="check.status === 'degraded'" />
              <CircleCloseFilled v-else />
            </el-icon>
            <span class="font-medium">{{ getCheckDisplayName(name) }}</span>
          </div>
          <div class="text-right">
            <div class="text-small text-secondary">{{ check.message }}</div>
            <div class="text-small text-secondary">{{ check.duration_ms }}ms</div>
          </div>
        </div>
      </div>
      
      <el-empty v-else description="暂无健康检查数据" />
    </ContentCard>
    
    <ContentCard title="系统信息" :span="12">
      <div v-if="configStore.config">
        <el-descriptions :column="1" border>
          <el-descriptions-item label="代理地址">
            {{ configStore.config.server.host }}:{{ configStore.config.server.port }}
          </el-descriptions-item>
          <el-descriptions-item label="工作进程">
            {{ configStore.config.server.workers }}
          </el-descriptions-item>
          <el-descriptions-item label="最大连接数">
            {{ configStore.config.server.max_connections }}
          </el-descriptions-item>
          <el-descriptions-item label="TLS 状态">
            <el-tag :type="configStore.config.server.tls.enabled ? 'success' : 'info'">
              {{ configStore.config.server.tls.enabled ? '已启用' : '未启用' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="认证状态">
            <el-tag :type="configStore.config.auth.enabled ? 'success' : 'info'">
              {{ configStore.config.auth.enabled ? '已启用' : '未启用' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="监控端口">
            {{ configStore.config.metrics.prometheus_port }}
          </el-descriptions-item>
        </el-descriptions>
      </div>
      
      <el-empty v-else description="暂无配置数据" />
    </ContentCard>
  </AppPage>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive } from 'vue'
import { useConfigStore } from '../stores/config'
import { getWeightStats } from '../api/config'
import type { WeightStatsResponse } from '../types'
import { WeightDistributionChart, WeightRealTimeMonitor, WeightTrendChart } from '../components/weight'
import AppPage from '../components/layout/AppPage.vue'
import ContentCard from '../components/layout/ContentCard.vue'
import StatCard from '../components/layout/StatCard.vue'
import { 
  Key, 
  CircleCheckFilled, 
  CircleCloseFilled,
  Monitor,
  WarningFilled,
  ScaleToOriginal,
  Connection,
  TrendCharts,
  Refresh
} from '@element-plus/icons-vue'

const configStore = useConfigStore()

// 权重状态管理
const weightStats = reactive<Partial<WeightStatsResponse>>({
  total_weight: 0,
  active_keys_count: 0,
  total_keys_count: 0,
  distributions: [],
  load_balance_effectiveness: 0
})

// 计算属性
const apiKeysCount = computed(() => configStore.apiKeysCount)
const activeKeysCount = computed(() => configStore.activeApiKeys.length)
const serverPort = computed(() => configStore.config?.server.port || 0)

const isHealthy = computed(() => configStore.healthStatus?.status === 'healthy')
const healthStatusText = computed(() => {
  if (!configStore.healthStatus) return '未知'
  return configStore.healthStatus.status === 'healthy' ? '健康' : '异常'
})
const healthStatusColor = computed(() => {
  if (!configStore.healthStatus) return '#909399'
  return configStore.healthStatus.status === 'healthy' ? '#67c23a' : '#f56c6c'
})

// 权重相关计算属性
const isWeightHealthy = computed(() => {
  const effectiveness = weightStats.load_balance_effectiveness || 0
  return effectiveness >= 70 && weightStats.active_keys_count === weightStats.total_keys_count
})

const weightHealthText = computed(() => {
  const effectiveness = weightStats.load_balance_effectiveness || 0
  if (effectiveness >= 90) return '优秀'
  if (effectiveness >= 70) return '良好'
  if (effectiveness >= 50) return '一般'
  return '待优化'
})

const weightHealthColor = computed(() => {
  const effectiveness = weightStats.load_balance_effectiveness || 0
  if (effectiveness >= 90) return '#67c23a'
  if (effectiveness >= 70) return '#e6a23c'
  return '#f56c6c'
})

// 方法
function refreshHealth() {
  configStore.checkHealth()
}

// 加载权重统计数据
async function loadWeightStats() {
  try {
    const response = await getWeightStats()
    if (response.success && response.data) {
      Object.assign(weightStats, response.data)
    }
  } catch (error) {
    console.error('加载权重统计失败:', error)
  }
}

// 初始化
onMounted(() => {
  loadWeightStats()
  // 定期刷新权重数据
  setInterval(loadWeightStats, 30000) // 每30秒刷新一次
})

function getCheckStatusColor(status: string) {
  switch (status) {
    case 'healthy': return '#67c23a'
    case 'degraded': return '#e6a23c'
    default: return '#f56c6c'
  }
}

function getCheckDisplayName(name: string) {
  const nameMap: Record<string, string> = {
    'system': '系统检查',
    'configuration': '配置检查',
    'api_keys': 'API 密钥检查'
  }
  return nameMap[name] || name
}
</script>

<style scoped>
/* 使用全局样式和工具类，无需自定义样式 */
</style>