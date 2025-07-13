<template>
  <div class="weight-realtime-monitor">
    <el-card shadow="hover">
      <template #header>
        <div class="monitor-header">
          <h3>实时权重监控</h3>
          <div class="monitor-controls">
            <el-switch
              v-model="autoRefresh"
              @change="toggleAutoRefresh"
              active-text="自动刷新"
              inactive-text="手动模式"
              size="small"
            />
            <el-select v-model="refreshInterval" size="small" :disabled="!autoRefresh">
              <el-option label="1秒" :value="1000" />
              <el-option label="5秒" :value="5000" />
              <el-option label="10秒" :value="10000" />
              <el-option label="30秒" :value="30000" />
            </el-select>
            <el-button size="small" @click="refreshData" :loading="loading">
              <el-icon><Refresh /></el-icon>
            </el-button>
          </div>
        </div>
      </template>

      <div class="monitor-content">
        <!-- 实时状态指示器 -->
        <div class="status-indicators">
          <div class="status-item">
            <el-badge :value="onlineKeysCount" :type="onlineKeysCount > 0 ? 'success' : 'danger'">
              <el-icon size="24" color="#67c23a"><Connection /></el-icon>
            </el-badge>
            <span class="status-label">在线密钥</span>
          </div>
          <div class="status-item">
            <el-badge :value="currentRequests" type="primary">
              <el-icon size="24" color="#409eff"><DataAnalysis /></el-icon>
            </el-badge>
            <span class="status-label">当前请求</span>
          </div>
          <div class="status-item">
            <el-badge :value="failureCount" :type="failureCount > 0 ? 'warning' : 'success'">
              <el-icon size="24" color="#e6a23c"><Warning /></el-icon>
            </el-badge>
            <span class="status-label">失败次数</span>
          </div>
          <div class="status-item">
            <div class="effectiveness-indicator">
              <el-progress
                :percentage="Math.round(currentEffectiveness)"
                :color="effectivenessColor"
                :stroke-width="8"
                :show-text="false"
              />
              <span class="effectiveness-value">{{ Math.round(currentEffectiveness) }}%</span>
            </div>
            <span class="status-label">均衡度</span>
          </div>
        </div>

        <!-- 实时权重表格 -->
        <div class="realtime-table">
          <el-table
            :data="realtimeData"
            style="width: 100%"
            :max-height="300"
            stripe
            border
          >
            <el-table-column prop="key_id" label="API密钥" width="120">
              <template #default="scope">
                <el-tag :type="scope.row.is_active ? 'success' : 'danger'" size="small">
                  {{ scope.row.key_id }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="weight" label="权重" width="80" align="center">
              <template #default="scope">
                <strong>{{ scope.row.weight }}</strong>
              </template>
            </el-table-column>
            <el-table-column prop="percentage" label="占比" width="80" align="center">
              <template #default="scope">
                {{ scope.row.percentage.toFixed(1) }}%
              </template>
            </el-table-column>
            <el-table-column prop="current_requests" label="当前请求" width="100" align="center">
              <template #default="scope">
                <el-tag :type="scope.row.current_requests > 0 ? 'primary' : 'info'" size="small">
                  {{ scope.row.current_requests }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="max_requests_per_minute" label="限流/分钟" width="100" align="center">
              <template #default="scope">
                {{ scope.row.max_requests_per_minute }}
              </template>
            </el-table-column>
            <el-table-column prop="failure_count" label="失败次数" width="100" align="center">
              <template #default="scope">
                <el-tag :type="scope.row.failure_count > 0 ? 'danger' : 'success'" size="small">
                  {{ scope.row.failure_count }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="负载状态" width="120" align="center">
              <template #default="scope">
                <div class="load-indicator">
                  <el-progress
                    :percentage="calculateLoadPercentage(scope.row)"
                    :color="getLoadColor(calculateLoadPercentage(scope.row))"
                    :stroke-width="6"
                    :show-text="false"
                  />
                  <span class="load-text">{{ getLoadStatus(calculateLoadPercentage(scope.row)) }}</span>
                </div>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="120" align="center">
              <template #default="scope">
                <el-button-group size="small">
                  <el-button @click="adjustWeight(scope.row.key_id, -10)" :disabled="scope.row.weight <= 10">
                    -10
                  </el-button>
                  <el-button @click="adjustWeight(scope.row.key_id, 10)">
                    +10
                  </el-button>
                </el-button-group>
              </template>
            </el-table-column>
          </el-table>
        </div>

        <!-- 最近活动日志 -->
        <div class="activity-log">
          <h4>最近活动</h4>
          <div class="log-container">
            <div v-for="(log, index) in activityLogs" :key="index" class="log-item">
              <el-tag :type="log.type" size="small">{{ log.time }}</el-tag>
              <span class="log-message">{{ log.message }}</span>
            </div>
            <div v-if="activityLogs.length === 0" class="empty-log">
              <el-empty description="暂无活动记录" :image-size="60" />
            </div>
          </div>
        </div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { ElMessage } from 'element-plus'
import { Refresh, Connection, DataAnalysis, Warning } from '@element-plus/icons-vue'
import { getWeightStats, updateKeyWeight } from '@/api/config'
import type { WeightDistribution } from '@/types'

// 组件状态
const loading = ref(false)
const autoRefresh = ref(true)
const refreshInterval = ref(5000)
const realtimeData = ref<WeightDistribution[]>([])

// 自动刷新定时器
let refreshTimer: number | null = null

// 活动日志
interface ActivityLog {
  time: string
  message: string
  type: 'success' | 'warning' | 'danger' | 'info'
}

const activityLogs = ref<ActivityLog[]>([])

// 计算属性
const onlineKeysCount = computed(() => 
  realtimeData.value.filter(item => item.is_active).length
)

const currentRequests = computed(() => 
  realtimeData.value.reduce((sum, item) => sum + item.current_requests, 0)
)

const failureCount = computed(() => 
  realtimeData.value.reduce((sum, item) => sum + item.failure_count, 0)
)

const currentEffectiveness = computed(() => {
  if (realtimeData.value.length === 0) return 0
  
  const totalWeight = realtimeData.value.reduce((sum, item) => sum + item.weight, 0)
  const idealWeight = totalWeight / realtimeData.value.length
  const variance = realtimeData.value.reduce((sum, item) => 
    sum + Math.pow(item.weight - idealWeight, 2), 0
  ) / realtimeData.value.length
  
  return Math.max(0, 100 - Math.sqrt(variance) / idealWeight * 100)
})

const effectivenessColor = computed(() => {
  const effectiveness = currentEffectiveness.value
  if (effectiveness >= 90) return '#67c23a'
  if (effectiveness >= 70) return '#e6a23c'
  return '#f56c6c'
})

// 计算负载百分比
const calculateLoadPercentage = (row: WeightDistribution) => {
  if (row.max_requests_per_minute === 0) return 0
  return Math.min(100, (row.current_requests / row.max_requests_per_minute) * 100)
}

// 获取负载颜色
const getLoadColor = (percentage: number) => {
  if (percentage < 50) return '#67c23a'
  if (percentage < 80) return '#e6a23c'
  return '#f56c6c'
}

// 获取负载状态
const getLoadStatus = (percentage: number) => {
  if (percentage < 30) return '空闲'
  if (percentage < 70) return '正常'
  if (percentage < 90) return '繁忙'
  return '满载'
}

// 添加活动日志
const addActivityLog = (message: string, type: ActivityLog['type'] = 'info') => {
  const log: ActivityLog = {
    time: new Date().toLocaleTimeString(),
    message,
    type
  }
  
  activityLogs.value.unshift(log)
  
  // 保持最多20条记录
  if (activityLogs.value.length > 20) {
    activityLogs.value = activityLogs.value.slice(0, 20)
  }
}

// 调整权重
const adjustWeight = async (keyId: string, delta: number) => {
  try {
    const currentItem = realtimeData.value.find(item => item.key_id === keyId)
    if (!currentItem) return
    
    const newWeight = Math.max(10, currentItem.weight + delta)
    
    const response = await updateKeyWeight(keyId, newWeight)
    if (response.success) {
      addActivityLog(`${keyId} 权重调整为 ${newWeight}`, 'success')
      await refreshData()
      ElMessage.success(`权重调整成功`)
    } else {
      throw new Error(response.message || '权重调整失败')
    }
  } catch (error) {
    console.error('调整权重失败:', error)
    const message = error instanceof Error ? error.message : '权重调整失败'
    addActivityLog(`权重调整失败: ${message}`, 'danger')
    ElMessage.error(message)
  }
}

// 刷新数据
const refreshData = async () => {
  try {
    loading.value = true
    
    const response = await getWeightStats()
    if (response.success && response.data) {
      const newData = response.data.distributions
      
      // 检测权重变化
      if (realtimeData.value.length > 0) {
        newData.forEach((newItem: WeightDistribution) => {
          const oldItem = realtimeData.value.find(item => item.key_id === newItem.key_id)
          if (oldItem && oldItem.weight !== newItem.weight) {
            addActivityLog(
              `${newItem.key_id} 权重从 ${oldItem.weight} 变更为 ${newItem.weight}`,
              'warning'
            )
          }
        })
      }
      
      realtimeData.value = newData
      
      // 检查异常状态
      newData.forEach((item: WeightDistribution) => {
        if (!item.is_active) {
          addActivityLog(`${item.key_id} 离线`, 'danger')
        }
        if (item.failure_count > 0) {
          addActivityLog(`${item.key_id} 发生 ${item.failure_count} 次失败`, 'warning')
        }
      })
      
    } else {
      throw new Error(response.message || '获取权重数据失败')
    }
  } catch (error) {
    console.error('刷新数据失败:', error)
    const message = error instanceof Error ? error.message : '刷新数据失败'
    addActivityLog(`数据刷新失败: ${message}`, 'danger')
    if (!autoRefresh.value) {
      ElMessage.error(message)
    }
  } finally {
    loading.value = false
  }
}

// 切换自动刷新
const toggleAutoRefresh = (enabled: boolean) => {
  if (enabled) {
    startAutoRefresh()
    addActivityLog('启用自动刷新', 'info')
  } else {
    stopAutoRefresh()
    addActivityLog('禁用自动刷新', 'info')
  }
}

// 开始自动刷新
const startAutoRefresh = () => {
  stopAutoRefresh()
  refreshTimer = setInterval(() => {
    refreshData()
  }, refreshInterval.value)
}

// 停止自动刷新
const stopAutoRefresh = () => {
  if (refreshTimer) {
    clearInterval(refreshTimer)
    refreshTimer = null
  }
}

// 监听刷新间隔变化 (暂时未使用)
// const onRefreshIntervalChange = () => {
//   if (autoRefresh.value) {
//     startAutoRefresh()
//     addActivityLog(`刷新间隔调整为 ${refreshInterval.value / 1000} 秒`, 'info')
//   }
// }

// 生命周期钩子
onMounted(() => {
  refreshData()
  if (autoRefresh.value) {
    startAutoRefresh()
  }
  addActivityLog('实时监控已启动', 'success')
})

onBeforeUnmount(() => {
  stopAutoRefresh()
})
</script>

<style scoped>
.weight-realtime-monitor {
  width: 100%;
}

.monitor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.monitor-header h3 {
  margin: 0;
  color: #303133;
  font-size: 18px;
  font-weight: 600;
}

.monitor-controls {
  display: flex;
  align-items: center;
  gap: 12px;
}

.monitor-content {
  min-height: 400px;
}

.status-indicators {
  display: flex;
  justify-content: space-around;
  padding: 20px 0;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 8px;
  margin-bottom: 24px;
}

.status-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: white;
}

.status-label {
  font-size: 14px;
  font-weight: 500;
}

.effectiveness-indicator {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.effectiveness-value {
  font-size: 16px;
  font-weight: 600;
  color: white;
}

.realtime-table {
  margin-bottom: 24px;
}

.load-indicator {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.load-text {
  font-size: 12px;
  color: #606266;
}

.activity-log {
  margin-top: 24px;
}

.activity-log h4 {
  margin: 0 0 16px 0;
  color: #303133;
  font-size: 16px;
  font-weight: 600;
}

.log-container {
  max-height: 200px;
  overflow-y: auto;
  background: #f8f9fa;
  border-radius: 6px;
  padding: 12px;
}

.log-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid #e4e7ed;
}

.log-item:last-child {
  border-bottom: none;
}

.log-message {
  font-size: 14px;
  color: #606266;
}

.empty-log {
  text-align: center;
  padding: 20px;
}

:deep(.el-progress-bar__outer) {
  height: 8px !important;
}

:deep(.el-table .el-table__cell) {
  padding: 8px 0;
}

@media (max-width: 768px) {
  .monitor-header {
    flex-direction: column;
    gap: 12px;
    align-items: stretch;
  }
  
  .monitor-controls {
    justify-content: center;
  }
  
  .status-indicators {
    flex-wrap: wrap;
    gap: 16px;
  }
  
  .realtime-table {
    overflow-x: auto;
  }
}
</style>