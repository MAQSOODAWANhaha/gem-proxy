<template>
  <div class="dashboard">
    <h1>控制台</h1>
    
    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-cards">
      <el-col :span="6">
        <el-card class="stat-card">
          <div class="stat-content">
            <div class="stat-icon">
              <el-icon color="#1890ff" size="32"><Key /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ apiKeysCount }}</div>
              <div class="stat-label">API 密钥</div>
            </div>
          </div>
        </el-card>
      </el-col>
      
      <el-col :span="6">
        <el-card class="stat-card">
          <div class="stat-content">
            <div class="stat-icon">
              <el-icon color="#52c41a" size="32"><CircleCheckFilled /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ activeKeysCount }}</div>
              <div class="stat-label">活跃密钥</div>
            </div>
          </div>
        </el-card>
      </el-col>
      
      <el-col :span="6">
        <el-card class="stat-card">
          <div class="stat-content">
            <div class="stat-icon">
              <el-icon color="#722ed1" size="32"><Monitor /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ serverPort }}</div>
              <div class="stat-label">服务端口</div>
            </div>
          </div>
        </el-card>
      </el-col>
      
      <el-col :span="6">
        <el-card class="stat-card">
          <div class="stat-content">
            <div class="stat-icon">
              <el-icon :color="healthStatusColor" size="32">
                <CircleCheckFilled v-if="isHealthy" />
                <CircleCloseFilled v-else />
              </el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ healthStatusText }}</div>
              <div class="stat-label">服务状态</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 健康检查详情 -->
    <el-row :gutter="20" class="health-section">
      <el-col :span="12">
        <el-card title="健康检查">
          <template #header>
            <div class="card-header">
              <span>健康检查</span>
              <el-button 
                type="primary" 
                size="small" 
                @click="refreshHealth"
                :loading="configStore.loading"
              >
                刷新
              </el-button>
            </div>
          </template>
          
          <div v-if="configStore.healthStatus" class="health-checks">
            <div 
              v-for="(check, name) in configStore.healthStatus.checks" 
              :key="name"
              class="health-check-item"
            >
              <div class="check-name">
                <el-icon :color="getCheckStatusColor(check.status)">
                  <CircleCheckFilled v-if="check.status === 'healthy'" />
                  <WarningFilled v-else-if="check.status === 'degraded'" />
                  <CircleCloseFilled v-else />
                </el-icon>
                {{ getCheckDisplayName(name) }}
              </div>
              <div class="check-details">
                <div class="check-message">{{ check.message }}</div>
                <div class="check-duration">{{ check.duration_ms }}ms</div>
              </div>
            </div>
          </div>
          
          <div v-else class="no-health-data">
            <el-empty description="暂无健康检查数据" />
          </div>
        </el-card>
      </el-col>
      
      <el-col :span="12">
        <el-card title="系统信息">
          <template #header>
            <div class="card-header">
              <span>系统信息</span>
            </div>
          </template>
          
          <div v-if="configStore.config" class="system-info">
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
          
          <div v-else class="no-config-data">
            <el-empty description="暂无配置数据" />
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useConfigStore } from '../stores/config'
import { 
  Key, 
  CircleCheckFilled, 
  CircleCloseFilled,
  Monitor,
  WarningFilled
} from '@element-plus/icons-vue'

const configStore = useConfigStore()

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

// 方法
function refreshHealth() {
  configStore.checkHealth()
}

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
.dashboard {
  max-width: 1200px;
}

.stats-cards {
  margin-bottom: 24px;
}

.stat-card {
  cursor: pointer;
  transition: all 0.3s;
}

.stat-card:hover {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat-icon {
  flex-shrink: 0;
}

.stat-info {
  flex: 1;
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: #1f2937;
  margin-bottom: 4px;
}

.stat-label {
  font-size: 14px;
  color: #6b7280;
}

.health-section {
  margin-bottom: 24px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.health-checks {
  space-y: 12px;
}

.health-check-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid #f0f0f0;
}

.health-check-item:last-child {
  border-bottom: none;
}

.check-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
}

.check-details {
  text-align: right;
}

.check-message {
  font-size: 14px;
  color: #6b7280;
}

.check-duration {
  font-size: 12px;
  color: #9ca3af;
  margin-top: 2px;
}

.system-info {
  margin-top: 16px;
}

.no-health-data,
.no-config-data {
  text-align: center;
  padding: 40px 0;
}
</style>