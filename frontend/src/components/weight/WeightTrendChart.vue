<template>
  <div class="weight-trend-chart">
    <el-card shadow="hover">
      <template #header>
        <div class="chart-header">
          <h3>权重使用趋势</h3>
          <div class="chart-controls">
            <el-select v-model="timeRange" size="small" @change="updateTimeRange">
              <el-option label="最近1小时" value="1h" />
              <el-option label="最近6小时" value="6h" />
              <el-option label="最近24小时" value="24h" />
              <el-option label="最近7天" value="7d" />
            </el-select>
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
          <!-- 趋势图表 -->
          <div ref="trendChartContainer" class="echart-container"></div>
          
          <!-- 权重变化统计 -->
          <div class="trend-stats">
            <el-row :gutter="16">
              <el-col :span="8">
                <div class="stat-item">
                  <div class="stat-label">权重变化次数</div>
                  <div class="stat-value">{{ trendData.change_count }}</div>
                </div>
              </el-col>
              <el-col :span="8">
                <div class="stat-item">
                  <div class="stat-label">平均负载均衡度</div>
                  <div class="stat-value">{{ trendData.avg_effectiveness?.toFixed(1) }}%</div>
                </div>
              </el-col>
              <el-col :span="8">
                <div class="stat-item">
                  <div class="stat-label">权重稳定性</div>
                  <div class="stat-value">
                    <el-tag :type="stabilityLevel.type" size="small">
                      {{ stabilityLevel.text }}
                    </el-tag>
                  </div>
                </div>
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
// import { getWeightStats } from '@/api/config' // 暂时注释，使用模拟数据

// 组件状态
const loading = ref(false)
const error = ref('')
const timeRange = ref('24h')
const trendChartContainer = ref<HTMLElement>()

// 图表实例
let trendChartInstance: echarts.ECharts | null = null

// 模拟趋势数据接口
interface WeightTrendData {
  timestamps: string[]
  key_trends: {
    key_id: string
    weights: number[]
    usage_rates: number[]
  }[]
  effectiveness_trend: number[]
  change_count: number
  avg_effectiveness: number
}

// 趋势数据状态
const trendData = reactive<WeightTrendData>({
  timestamps: [],
  key_trends: [],
  effectiveness_trend: [],
  change_count: 0,
  avg_effectiveness: 0
})

// 颜色配置
const trendColors = [
  '#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de',
  '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc', '#ffb980'
]

// 计算权重稳定性
const stabilityLevel = computed(() => {
  const changeCount = trendData.change_count
  const timeHours = parseInt(timeRange.value.replace(/[^\d]/g, ''))
  const changeRate = changeCount / timeHours
  
  if (changeRate < 1) {
    return { type: 'success', text: '稳定' }
  } else if (changeRate < 3) {
    return { type: 'warning', text: '中等' }
  } else {
    return { type: 'danger', text: '频繁变化' }
  }
})

// 生成模拟趋势数据
const generateMockTrendData = () => {
  const now = new Date()
  const timeHours = parseInt(timeRange.value.replace(/[^\d]/g, ''))
  const pointCount = Math.min(timeHours * 6, 144) // 每10分钟一个点，最多144个点
  
  const timestamps: string[] = []
  const effectiveness_trend: number[] = []
  
  // 生成时间戳
  for (let i = pointCount - 1; i >= 0; i--) {
    const time = new Date(now.getTime() - i * 10 * 60 * 1000) // 10分钟间隔
    timestamps.push(time.toLocaleTimeString())
  }
  
  // 模拟权重趋势数据
  const key_trends = [
    { key_id: 'key1', weights: [] as number[], usage_rates: [] as number[] },
    { key_id: 'key2', weights: [] as number[], usage_rates: [] as number[] },
    { key_id: 'key3', weights: [] as number[], usage_rates: [] as number[] }
  ]
  
  // 生成模拟数据
  let baseWeights = [100, 100, 100]
  for (let i = 0; i < pointCount; i++) {
    // 模拟权重变化
    if (Math.random() < 0.1) { // 10%概率发生权重变化
      const keyIndex = Math.floor(Math.random() * 3)
      baseWeights[keyIndex] += (Math.random() - 0.5) * 50
      baseWeights[keyIndex] = Math.max(50, Math.min(300, baseWeights[keyIndex]))
    }
    
    key_trends.forEach((trend, index) => {
      trend.weights.push(Math.round(baseWeights[index]))
      trend.usage_rates.push(Math.random() * 80 + 10) // 10-90%使用率
    })
    
    // 计算负载均衡有效性
    const totalWeight = baseWeights.reduce((sum, w) => sum + w, 0)
    const idealWeight = totalWeight / 3
    const variance = baseWeights.reduce((sum, w) => sum + Math.pow(w - idealWeight, 2), 0) / 3
    const effectiveness = Math.max(0, 100 - Math.sqrt(variance) / idealWeight * 100)
    effectiveness_trend.push(Math.round(effectiveness * 10) / 10)
  }
  
  // 更新数据
  Object.assign(trendData, {
    timestamps,
    key_trends,
    effectiveness_trend,
    change_count: Math.floor(Math.random() * timeHours * 2),
    avg_effectiveness: effectiveness_trend.reduce((sum, val) => sum + val, 0) / effectiveness_trend.length
  })
}

// 生成趋势图表配置
const generateTrendChartOption = (): EChartsOption => {
  const series: any[] = []
  
  // 权重趋势线
  trendData.key_trends.forEach((trend, index) => {
    series.push({
      name: `${trend.key_id} 权重`,
      type: 'line',
      yAxisIndex: 0,
      data: trend.weights,
      lineStyle: {
        color: trendColors[index],
        width: 2
      },
      symbol: 'circle',
      symbolSize: 4,
      smooth: true
    })
  })
  
  // 使用率趋势线
  trendData.key_trends.forEach((trend, index) => {
    series.push({
      name: `${trend.key_id} 使用率`,
      type: 'line',
      yAxisIndex: 1,
      data: trend.usage_rates,
      lineStyle: {
        color: trendColors[index],
        width: 1,
        type: 'dashed'
      },
      symbol: 'none',
      smooth: true
    })
  })
  
  // 负载均衡有效性
  series.push({
    name: '负载均衡有效性',
    type: 'line',
    yAxisIndex: 1,
    data: trendData.effectiveness_trend,
    lineStyle: {
      color: '#ff6b6b',
      width: 3
    },
    areaStyle: {
      color: {
        type: 'linear',
        x: 0, y: 0, x2: 0, y2: 1,
        colorStops: [
          { offset: 0, color: 'rgba(255, 107, 107, 0.3)' },
          { offset: 1, color: 'rgba(255, 107, 107, 0.1)' }
        ]
      }
    },
    symbol: 'none',
    smooth: true
  })
  
  return {
    title: {
      text: '权重使用趋势分析',
      subtext: `时间范围: ${timeRange.value}`,
      left: 'center'
    },
    tooltip: {
      trigger: 'axis',
      axisPointer: {
        type: 'cross'
      },
      formatter: (params: any) => {
        const time = params[0].axisValue
        let content = `<div style="padding: 8px;"><strong>${time}</strong><br/>`
        params.forEach((param: any) => {
          content += `${param.seriesName}: ${param.value}<br/>`
        })
        content += '</div>'
        return content
      }
    },
    legend: {
      data: series.map(s => s.name),
      bottom: 10,
      type: 'scroll'
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '15%',
      containLabel: true
    },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: trendData.timestamps,
      axisLabel: {
        rotate: 45,
        formatter: (value: string) => {
          // 只显示时:分
          return value.split(':').slice(0, 2).join(':')
        }
      }
    },
    yAxis: [
      {
        type: 'value',
        name: '权重值',
        position: 'left',
        splitLine: {
          show: true,
          lineStyle: {
            type: 'dashed'
          }
        }
      },
      {
        type: 'value',
        name: '百分比 (%)',
        position: 'right',
        max: 100,
        splitLine: {
          show: false
        }
      }
    ],
    dataZoom: [
      {
        type: 'inside',
        start: 0,
        end: 100
      },
      {
        start: 0,
        end: 100,
        height: 30,
        bottom: 60
      }
    ],
    series
  }
}

// 更新图表
const updateChart = async () => {
  if (!trendChartContainer.value) return
  
  // 销毁现有图表
  if (trendChartInstance) {
    trendChartInstance.dispose()
  }
  
  // 创建新图表实例
  trendChartInstance = echarts.init(trendChartContainer.value)
  
  // 设置图表配置
  const option = generateTrendChartOption()
  trendChartInstance.setOption(option)
  
  // 监听窗口大小变化
  const handleResize = () => {
    if (trendChartInstance) {
      trendChartInstance.resize()
    }
  }
  window.addEventListener('resize', handleResize)
}

// 加载趋势数据
const loadTrendData = async () => {
  try {
    loading.value = true
    error.value = ''
    
    // 暂时使用模拟数据，后续可替换为真实API
    generateMockTrendData()
    
    await nextTick()
    updateChart()
  } catch (err) {
    console.error('加载趋势数据失败:', err)
    error.value = err instanceof Error ? err.message : '加载趋势数据失败'
    ElMessage.error(error.value)
  } finally {
    loading.value = false
  }
}

// 更新时间范围
const updateTimeRange = () => {
  loadTrendData()
}

// 刷新数据
const refreshData = () => {
  loadTrendData()
}

// 生命周期钩子
onMounted(() => {
  loadTrendData()
})

onBeforeUnmount(() => {
  if (trendChartInstance) {
    trendChartInstance.dispose()
  }
  window.removeEventListener('resize', () => {})
})
</script>

<style scoped>
.weight-trend-chart {
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
  min-height: 450px;
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

.trend-stats {
  padding: 20px;
  background: #f8f9fa;
  border-radius: 8px;
  border: 1px solid #e9ecef;
}

.stat-item {
  text-align: center;
  padding: 16px;
  background: white;
  border-radius: 6px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.stat-label {
  font-size: 14px;
  color: #606266;
  margin-bottom: 8px;
}

.stat-value {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
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