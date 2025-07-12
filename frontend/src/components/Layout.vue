<template>
  <el-container class="layout-container">
    <!-- 侧边栏 -->
    <el-aside width="250px" class="sidebar">
      <div class="logo">
        <h2>Gemini 代理管理</h2>
      </div>
      
      <el-menu
        :default-active="currentPath"
        class="sidebar-menu"
        background-color="#001529"
        text-color="#ffffff"
        active-text-color="#1890ff"
        router
      >
        <el-menu-item 
          v-for="item in menuItems" 
          :key="item.path"
          :index="item.path"
        >
          <el-icon><component :is="item.icon" /></el-icon>
          <span>{{ item.title }}</span>
        </el-menu-item>
      </el-menu>
    </el-aside>

    <!-- 主要内容区域 -->
    <el-container>
      <!-- 顶部导航栏 -->
      <el-header class="header">
        <div class="header-left">
          <el-breadcrumb separator="/">
            <el-breadcrumb-item>首页</el-breadcrumb-item>
            <el-breadcrumb-item>{{ currentPageTitle }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>
        
        <div class="header-right">
          <!-- 健康状态指示器 -->
          <el-tooltip :content="healthStatusText" placement="bottom">
            <el-badge 
              :value="healthStatusBadge" 
              :type="healthStatusType"
              class="health-indicator"
            >
              <el-icon :color="healthStatusColor">
                <CircleCheckFilled v-if="isHealthy" />
                <CircleCloseFilled v-else />
              </el-icon>
            </el-badge>
          </el-tooltip>
          
          <!-- 刷新按钮 -->
          <el-button 
            type="text" 
            size="large"
            @click="refreshData"
            :loading="loading"
          >
            <el-icon><Refresh /></el-icon>
          </el-button>
        </div>
      </el-header>

      <!-- 主要内容 -->
      <el-main class="main-content">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { useConfigStore } from '../stores/config'
import { 
  CircleCheckFilled,
  CircleCloseFilled,
  Refresh
} from '@element-plus/icons-vue'

const route = useRoute()
const configStore = useConfigStore()

// 菜单项配置
const menuItems = [
  { path: '/dashboard', title: '控制台', icon: 'Monitor' },
  { path: '/config', title: '代理配置', icon: 'Setting' },
  { path: '/api-keys', title: 'API 密钥', icon: 'Key' },
  { path: '/tls-config', title: 'TLS 配置', icon: 'Lock' },
  { path: '/monitoring', title: '监控指标', icon: 'DataAnalysis' }
]

// 计算属性
const currentPath = computed(() => route.path)
const currentPageTitle = computed(() => {
  const currentItem = menuItems.find(item => item.path === route.path)
  return currentItem?.title || '未知页面'
})

const loading = computed(() => configStore.loading)

// 健康状态相关
const isHealthy = computed(() => {
  return configStore.healthStatus?.status === 'healthy'
})

const healthStatusText = computed(() => {
  if (!configStore.healthStatus) return '未知状态'
  const status = configStore.healthStatus.status
  const timestamp = new Date(configStore.healthStatus.timestamp * 1000).toLocaleString()
  return `状态: ${status} (更新时间: ${timestamp})`
})

const healthStatusBadge = computed(() => {
  if (!configStore.healthStatus) return '?'
  return configStore.healthStatus.status === 'healthy' ? '' : '!'
})

const healthStatusType = computed(() => {
  if (!configStore.healthStatus) return 'info'
  const status = configStore.healthStatus.status
  if (status === 'healthy') return 'success'
  if (status === 'degraded') return 'warning'
  return 'danger'
})

const healthStatusColor = computed(() => {
  if (!configStore.healthStatus) return '#909399'
  const status = configStore.healthStatus.status
  if (status === 'healthy') return '#67c23a'
  if (status === 'degraded') return '#e6a23c'
  return '#f56c6c'
})

// 刷新数据
async function refreshData() {
  try {
    await configStore.checkHealth()
    await configStore.loadConfig()
  } catch (error) {
    console.error('刷新数据失败:', error)
  }
}

// 定时健康检查
let healthCheckInterval: number | null = null

onMounted(() => {
  // 初始加载
  refreshData()
  
  // 设置定时健康检查 (每30秒)
  healthCheckInterval = window.setInterval(() => {
    configStore.checkHealth()
  }, 30000)
})

onUnmounted(() => {
  if (healthCheckInterval) {
    clearInterval(healthCheckInterval)
  }
})
</script>

<style scoped>
.layout-container {
  height: 100vh;
}

.sidebar {
  background-color: #001529;
}

.logo {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-bottom: 1px solid #002140;
}

.logo h2 {
  color: #ffffff;
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.sidebar-menu {
  border-right: none;
}

.header {
  background-color: #ffffff;
  border-bottom: 1px solid #f0f0f0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
}

.header-left {
  flex: 1;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

.health-indicator {
  cursor: pointer;
}

.main-content {
  padding: 24px;
  background-color: #f5f5f5;
  overflow-y: auto;
}
</style>