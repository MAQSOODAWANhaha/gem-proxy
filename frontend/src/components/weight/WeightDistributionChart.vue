<template>
  <div class="weight-distribution-chart">
    <el-card shadow="hover">
      <template #header>
        <div class="chart-header">
          <h3>权重分布图表</h3>
          <div class="chart-controls">
            <el-radio-group v-model="chartType" size="small" @change="updateChart">
              <el-radio-button value="pie">饼图</el-radio-button>
              <el-radio-button value="bar">柱状图</el-radio-button>
              <el-radio-button value="comparison">对比图</el-radio-button>
            </el-radio-group>
            <el-button size="small" @click="refreshData" :loading="loading">
              <el-icon><Refresh /></el-icon>
              刷新
            </el-button>
          </div>
        </div>
      </template>

      <div class="chart-container">
        <div v-if="loading" class="loading-container">
          <el-skeleton :rows="5" animated />
        </div>
        <div v-else-if="error" class="error-container">
          <el-alert :title="error" type="error" show-icon />
        </div>
        <div v-else>
          <!-- 图表容器 -->
          <div ref="chartContainer" class="echart-container"></div>
          
          <!-- 权重统计信息 -->
          <div class="weight-stats">
            <el-row :gutter="16">
              <el-col :span="6">
                <el-statistic title="总权重" :value="statsData.total_weight">
                  <template #suffix>
                    <el-tag size="small" type="primary">权重单位</el-tag>
                  </template>
                </el-statistic>
              </el-col>
              <el-col :span="6">
                <el-statistic title="活跃密钥" :value="statsData.active_keys_count">
                  <template #suffix>
                    <span>/ {{ statsData.total_keys_count }}</span>
                  </template>
                </el-statistic>
              </el-col>
              <el-col :span="6">
                <el-statistic title="负载均衡有效性" :value="statsData.load_balance_effectiveness" precision="1">
                  <template #suffix>%</template>
                </el-statistic>
              </el-col>
              <el-col :span="6">
                <el-statistic title="最后更新" :value="lastUpdateTime">
                  <template #suffix>
                    <el-tag size="small" :type="isRecentUpdate ? 'success' : 'info'">
                      {{ isRecentUpdate ? '最新' : '较早' }}
                    </el-tag>
                  </template>
                </el-statistic>
              </el-col>
            </el-row>
          </div>
        </div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onBeforeUnmount, computed, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import type { EChartsOption } from 'echarts'
import { getWeightStats } from '@/api/config'
import type { WeightStatsResponse } from '@/types'

// 组件状态
const loading = ref(false)
const error = ref('')
const chartType = ref<'pie' | 'bar' | 'comparison'>('pie')
const chartContainer = ref<HTMLElement>()
const lastUpdateTime = ref('')

// 图表实例
let chartInstance: echarts.ECharts | null = null

// 数据状态
const statsData = reactive<WeightStatsResponse>({
  total_weight: 0,
  active_keys_count: 0,
  total_keys_count: 0,
  distributions: [],
  load_balance_effectiveness: 0
})

// 计算属性
const isRecentUpdate = computed(() => {
  if (!lastUpdateTime.value) return false
  const updateTime = new Date(lastUpdateTime.value)
  const now = new Date()
  return (now.getTime() - updateTime.getTime()) < 60000 // 1分钟内
})

// 颜色配置
const chartColors = [
  '#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de',
  '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc', '#ffb980'
]

// 获取权重统计数据
const loadWeightStats = async () => {
  try {
    loading.value = true
    error.value = ''
    
    const response = await getWeightStats()
    if (response.success && response.data) {
      Object.assign(statsData, response.data)
      lastUpdateTime.value = new Date().toLocaleString()
      await nextTick()
      updateChart()
    } else {
      throw new Error(response.message || '获取权重统计失败')
    }
  } catch (err) {
    console.error('加载权重统计失败:', err)
    error.value = err instanceof Error ? err.message : '加载权重统计失败'
    ElMessage.error(error.value)
  } finally {
    loading.value = false
  }
}

// 刷新数据
const refreshData = () => {
  loadWeightStats()
}

// 生成饼图配置
const generatePieChartOption = (): EChartsOption => {
  const data = statsData.distributions.map((item, index) => ({
    name: item.key_id,
    value: item.weight,
    itemStyle: {
      color: chartColors[index % chartColors.length]
    },
    label: {
      formatter: (params: any) => {
        return `${params.name}\n权重: ${params.value}\n占比: ${params.percent}%`
      }
    }
  }))

  return {
    title: {
      text: 'API密钥权重分布',
      subtext: `总权重: ${statsData.total_weight}`,
      left: 'center'
    },
    tooltip: {
      trigger: 'item',
      formatter: (params: any) => {
        const item = statsData.distributions.find(d => d.key_id === params.name)
        return `
          <div style="padding: 8px;">
            <strong>${params.name}</strong><br/>
            权重: ${params.value}<br/>
            占比: ${params.percent}%<br/>
            状态: ${item?.is_active ? '活跃' : '非活跃'}<br/>
            当前请求: ${item?.current_requests || 0}<br/>
            失败次数: ${item?.failure_count || 0}
          </div>
        `
      }
    },
    legend: {
      orient: 'vertical',
      left: 'left',
      data: data.map(item => item.name)
    },
    series: [
      {
        name: '权重分布',
        type: 'pie',
        radius: ['40%', '70%'],
        center: ['60%', '60%'],
        avoidLabelOverlap: false,
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowOffsetX: 0,
            shadowColor: 'rgba(0, 0, 0, 0.5)'
          }
        },
        data
      }
    ]
  }
}

// 生成柱状图配置
const generateBarChartOption = (): EChartsOption => {
  const data = statsData.distributions
  const xAxisData = data.map(item => item.key_id)
  const weightData = data.map(item => item.weight)
  const percentageData = data.map(item => item.percentage)

  return {
    title: {
      text: 'API密钥权重对比',
      subtext: `负载均衡有效性: ${statsData.load_balance_effectiveness.toFixed(1)}%`,
      left: 'center'
    },
    tooltip: {
      trigger: 'axis',
      axisPointer: {
        type: 'shadow'
      },
      formatter: (params: any) => {
        const index = params[0].dataIndex
        const item = data[index]
        return `
          <div style="padding: 8px;">
            <strong>${item.key_id}</strong><br/>
            权重: ${item.weight}<br/>
            占比: ${item.percentage.toFixed(2)}%<br/>
            状态: ${item.is_active ? '活跃' : '非活跃'}<br/>
            限流: ${item.max_requests_per_minute}/分钟
          </div>
        `
      }
    },
    legend: {
      data: ['权重值', '百分比']
    },
    xAxis: {
      type: 'category',
      data: xAxisData,
      axisLabel: {
        rotate: 45
      }
    },
    yAxis: [
      {
        type: 'value',
        name: '权重值',
        position: 'left'
      },
      {
        type: 'value',
        name: '百分比 (%)',
        position: 'right',
        max: 100
      }
    ],
    series: [
      {
        name: '权重值',
        type: 'bar',
        data: weightData.map((value, index) => ({
          value,
          itemStyle: {
            color: chartColors[index % chartColors.length]
          }
        })),
        itemStyle: {
          borderRadius: [4, 4, 0, 0]
        }
      },
      {
        name: '百分比',
        type: 'line',
        yAxisIndex: 1,
        data: percentageData,
        lineStyle: {
          color: '#ff6b6b',
          width: 3
        },
        symbol: 'circle',
        symbolSize: 8
      }
    ]
  }
}

// 生成对比图配置
const generateComparisonChartOption = (): EChartsOption => {
  const data = statsData.distributions
  
  const seriesData = data.map((item, index) => ({
    name: item.key_id,
    type: 'radar' as const,
    data: [
      {
        value: [
          item.weight,
          item.percentage,
          item.current_requests,
          item.failure_count
        ],
        itemStyle: {
          color: chartColors[index % chartColors.length]
        }
      }
    ]
  }))

  return {
    title: {
      text: 'API密钥多维度对比',
      subtext: '雷达图综合分析',
      left: 'center'
    },
    tooltip: {
      trigger: 'item'
    },
    legend: {
      data: data.map(item => item.key_id),
      bottom: 10
    },
    radar: {
      indicator: [
        { name: '权重', max: Math.max(...data.map(d => d.weight)) * 1.2 },
        { name: '占比 (%)', max: 100 },
        { name: '当前请求', max: Math.max(...data.map(d => d.current_requests), 10) },
        { name: '失败次数', max: Math.max(...data.map(d => d.failure_count), 5) }
      ],
      center: ['50%', '55%'],
      radius: 120
    },
    series: seriesData
  }
}

// 更新图表
const updateChart = async () => {
  if (!chartContainer.value || statsData.distributions.length === 0) return

  // 销毁现有图表
  if (chartInstance) {
    chartInstance.dispose()
  }

  // 创建新图表实例
  chartInstance = echarts.init(chartContainer.value)

  // 根据图表类型生成配置
  let option: EChartsOption
  switch (chartType.value) {
    case 'pie':
      option = generatePieChartOption()
      break
    case 'bar':
      option = generateBarChartOption()
      break
    case 'comparison':
      option = generateComparisonChartOption()
      break
    default:
      option = generatePieChartOption()
  }

  // 设置图表配置
  chartInstance.setOption(option)

  // 监听窗口大小变化
  const handleResize = () => {
    if (chartInstance) {
      chartInstance.resize()
    }
  }
  window.addEventListener('resize', handleResize)
}

// 生命周期钩子
onMounted(() => {
  loadWeightStats()
})

onBeforeUnmount(() => {
  if (chartInstance) {
    chartInstance.dispose()
  }
  window.removeEventListener('resize', () => {})
})
</script>

<style scoped>
.weight-distribution-chart {
  width: 100%;
}

.chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.chart-header h3 {
  margin: 0;
  color: #303133;
  font-size: 18px;
  font-weight: 600;
}

.chart-controls {
  display: flex;
  align-items: center;
  gap: 12px;
}

.chart-container {
  min-height: 400px;
}

.loading-container,
.error-container {
  padding: 40px;
  text-align: center;
}

.echart-container {
  width: 100%;
  height: 400px;
  margin-bottom: 24px;
}

.weight-stats {
  padding: 20px;
  background: #f8f9fa;
  border-radius: 8px;
  border: 1px solid #e9ecef;
}

:deep(.el-statistic__content) {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

:deep(.el-statistic__title) {
  color: #606266;
  font-size: 14px;
  margin-bottom: 8px;
}

:deep(.el-radio-button__inner) {
  padding: 8px 15px;
}

@media (max-width: 768px) {
  .chart-header {
    flex-direction: column;
    gap: 12px;
    align-items: stretch;
  }
  
  .chart-controls {
    justify-content: center;
  }
  
  .echart-container {
    height: 300px;
  }
}
</style>