<template>
  <AppPage 
    title="负载均衡管理" 
    description="智能权重配置与负载均衡策略管理中心"
  >
    <template #actions>
      <el-button 
        type="primary" 
        @click="refreshWeights"
        :loading="weightsLoading"
      >
        <el-icon><Refresh /></el-icon>
        刷新数据
      </el-button>
    </template>
    
    <!-- 第1行：系统状态统计概览 -->
    <ContentCard title="系统状态概览" :span="24">
      <el-row :gutter="32">
        <StatCard
          title="API Keys 总数"
          :value="totalApiKeys"
          :value-style="{ color: 'var(--color-primary)' }"
          :icon="Key"
          icon-color="var(--color-primary)"
          :span="6"
        />
        
        <StatCard
          title="活跃 Keys"
          :value="activeApiKeys"
          :value-style="{ color: 'var(--color-success)' }"
          :icon="CircleCheck"
          icon-color="var(--color-success)"
          :span="6"
        />
        
        <StatCard
          title="总权重"
          :value="totalWeight"
          :value-style="{ color: 'var(--color-warning)' }"
          :icon="ScaleToOriginal"
          icon-color="var(--color-warning)"
          :span="6"
        />
        
        <StatCard
          title="负载均衡评分"
          :value="loadBalanceScoreText + '/100'"
          :value-style="{ color: getScoreColor(loadBalanceScore) }"
          :icon="TrendCharts"
          :icon-color="getScoreColor(loadBalanceScore)"
          :span="6"
        />
      </el-row>
    </ContentCard>

    <!-- 第2行：权重配置管理 -->
    <ContentCard 
      title="权重配置管理" 
      description="手动调整API密钥权重分配"
      :span="24"
    >
          <template #actions>
            <el-button 
              type="primary" 
              size="small" 
              @click="refreshWeights"
              :loading="weightsLoading"
            >
              <el-icon><Refresh /></el-icon>
              刷新
            </el-button>
          </template>

          <div class="weight-management">
            <div v-for="apiKey in apiKeys" :key="apiKey.id" class="weight-item">
              <div class="weight-info">
                <div class="key-name">{{ apiKey.id }}</div>
                <div class="key-status">
                  <el-tag :type="apiKey.enabled ? 'success' : 'danger'" size="small">
                    {{ apiKey.enabled ? '启用' : '禁用' }}
                  </el-tag>
                </div>
              </div>
              <div class="weight-control">
                <el-slider
                  v-model="apiKey.weight"
                  :min="0"
                  :max="1000"
                  :step="10"
                  :show-tooltip="true"
                  @change="onWeightChange(apiKey)"
                  class="weight-slider"
                />
                <div class="weight-value">{{ apiKey.weight }}</div>
              </div>
            </div>
          </div>

          <div class="panel-actions">
            <el-button type="success" @click="saveWeights" :loading="savingWeights">
              <el-icon><Check /></el-icon>
              保存配置
            </el-button>
            <el-button @click="createSnapshot">
              <el-icon><Camera /></el-icon>
              创建快照
            </el-button>
            <el-button @click="openBatchEdit">
              <el-icon><Edit /></el-icon>
              批量编辑
            </el-button>
          </div>
    </ContentCard>

    <!-- 权重分析模块 - 符合UI设计标准的Element Plus原生布局 -->
    <el-row :gutter="24" class="weight-analysis-row">
      <!-- 左侧：权重分布图表 -->
      <el-col :span="12">
        <el-card class="chart-card">
          <template #header>
            <div class="card-header">
              <div class="header-main">
                <span class="card-title">权重分布图表</span>
                <p class="page-description">多维度权重分布分析</p>
              </div>
              <div class="card-actions">
                <el-button-group size="small">
                  <el-button @click="chartType = 'pie'" :type="chartType === 'pie' ? 'primary' : ''">饼图</el-button>
                  <el-button @click="chartType = 'bar'" :type="chartType === 'bar' ? 'primary' : ''">柱状图</el-button>
                  <el-button @click="chartType = 'radar'" :type="chartType === 'radar' ? 'primary' : ''">雷达图</el-button>
                </el-button-group>
              </div>
            </div>
          </template>
          
          <div ref="chartContainer" class="chart-container">
          </div>
        </el-card>
      </el-col>

      <!-- 右侧：权重趋势分析 -->
      <el-col :span="12">
        <el-card class="chart-card">
          <template #header>
            <div class="card-header">
              <div class="header-main">
                <span class="card-title">权重趋势分析</span>
                <p class="page-description">权重变化趋势与稳定性分析</p>
              </div>
              <div class="card-actions">
                <el-button-group size="small">
                  <el-button @click="trendTimeRange = '1h'" :type="trendTimeRange === '1h' ? 'primary' : ''">1小时</el-button>
                  <el-button @click="trendTimeRange = '6h'" :type="trendTimeRange === '6h' ? 'primary' : ''">6小时</el-button>
                  <el-button @click="trendTimeRange = '24h'" :type="trendTimeRange === '24h' ? 'primary' : ''">24小时</el-button>
                </el-button-group>
              </div>
            </div>
          </template>
          
          <!-- 趋势统计信息 -->
          <div class="trend-stats">
            <el-row :gutter="16">
              <el-col :span="12">
                <div class="trend-stat-item">
                  <div class="trend-label">权重波动率</div>
                  <div class="trend-value">{{ weightVolatility }}%</div>
                </div>
              </el-col>
              <el-col :span="12">
                <div class="trend-stat-item">
                  <div class="trend-label">调整频率</div>
                  <div class="trend-value">{{ adjustmentFrequency }}/小时</div>
                </div>
              </el-col>
            </el-row>
          </div>
          
          <div ref="trendChartContainer" class="trend-chart-container">
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 实时权重监控 -->
    <ContentCard 
      title="实时权重监控" 
      description="关键指标实时监控与状态展示"
      :span="24"
      class="real-time-monitoring"
    >
      <template #actions>
        <el-button 
          size="small" 
          @click="refreshRealTimeData"
          :loading="realTimeLoading"
        >
          <el-icon><Refresh /></el-icon>
          刷新
        </el-button>
      </template>

      <el-row :gutter="32">
        <StatCard 
          :span="6"
          title="在线密钥"
          :value="activeApiKeys"
          :value-style="{ color: '#52c41a' }"
          :icon="CircleCheck"
          icon-color="#52c41a"
        />
        <StatCard 
          :span="6"
          title="当前请求"
          :value="currentRequests"
          :value-style="{ color: '#1890ff' }"
          :icon="TrendCharts"
          icon-color="#1890ff"
        />
        <StatCard 
          :span="6"
          title="失败次数"
          :value="failedRequests"
          :value-style="{ color: '#f5222d' }"
          :icon="Close"
          icon-color="#f5222d"
        />
        <StatCard 
          :span="6"
          title="均衡度"
          :value="loadBalanceScoreText + '%'"
          :value-style="{ color: getScoreColor(loadBalanceScore) }"
          :icon="ScaleToOriginal"
          :icon-color="getScoreColor(loadBalanceScore)"
        />
      </el-row>
    </ContentCard>


    <!-- 智能优化建议 -->
    <ContentCard 
      title="智能优化建议" 
      description="基于性能数据的智能权重优化建议"
      :span="24"
    >
        <template #actions>
          <el-button 
            type="primary" 
            size="small" 
            @click="generateOptimization"
            :loading="optimizationLoading"
          >
            <el-icon><MagicStick /></el-icon>
            生成建议
          </el-button>
        </template>

        <div class="optimization-panel">
          <div class="strategy-selector">
            <el-select v-model="selectedStrategy" placeholder="选择优化策略" style="width: 100%">
              <el-option
                v-for="strategy in optimizationStrategies"
                :key="strategy.value"
                :label="strategy.label"
                :value="strategy.value"
              />
            </el-select>
          </div>

          <div v-if="optimizationResult" class="optimization-result">
            <div class="result-summary">
              <div class="summary-grid">
                <div class="summary-item">
                  <span class="summary-label">策略</span>
                  <span class="summary-value">{{ getStrategyLabel(optimizationResult.strategy) }}</span>
                </div>
                <div class="summary-item">
                  <span class="summary-label">置信度</span>
                  <span class="summary-value">{{ (optimizationResult.confidence_score * 100).toFixed(1) }}%</span>
                </div>
                <div class="summary-item">
                  <span class="summary-label">预期改进</span>
                  <span class="summary-value">{{ optimizationResult.overall_improvement.toFixed(1) }}%</span>
                </div>
                <div class="summary-item">
                  <span class="summary-label">建议数量</span>
                  <span class="summary-value">{{ optimizationResult.recommendations.length }}</span>
                </div>
              </div>
            </div>

            <div class="recommendations">
              <div v-for="rec in optimizationResult.recommendations" :key="rec.key_id" class="recommendation-item">
                <div class="rec-header">
                  <span class="key-id">{{ rec.key_id }}</span>
                  <el-tag :type="getRiskTagType(rec.risk_level)" size="small">
                    {{ rec.risk_level }}
                  </el-tag>
                </div>
                <div class="rec-changes">
                  <span class="weight-change">
                    {{ rec.current_weight }} → {{ rec.recommended_weight }}
                  </span>
                  <span class="improvement">+{{ rec.expected_improvement.toFixed(1) }}%</span>
                </div>
                <div class="rec-reason">{{ rec.reason }}</div>
              </div>
            </div>

            <div class="optimization-actions">
              <el-button type="primary" @click="applyOptimization">
                <el-icon><Check /></el-icon>
                应用建议
              </el-button>
              <el-button @click="previewOptimization">
                <el-icon><View /></el-icon>
                预览效果
              </el-button>
            </div>
          </div>

          <div v-else class="no-optimization">
            <el-empty description="暂无优化建议，请选择策略并点击生成建议" />
          </div>
        </div>
    </ContentCard>

    <!-- 第5行：权重变更审计 -->
    <ContentCard 
      title="权重变更审计" 
      description="完整的权重配置变更历史记录与追踪"
      :span="24"
    >
          <template #actions>
            <div class="header-actions">
              <el-button size="small" @click="openAuditQuery">
                <el-icon><Search /></el-icon>
                查询
              </el-button>
              <el-button size="small" @click="exportAuditRecords">
                <el-icon><Download /></el-icon>
                导出
              </el-button>
              <el-button size="small" @click="showSnapshots">
                <el-icon><FolderOpened /></el-icon>
                快照管理
              </el-button>
            </div>
          </template>

          <div class="audit-content">
            <el-table
              :data="auditRecords"
              v-loading="auditLoading"
              stripe
              style="width: 100%"
              :default-sort="{ prop: 'timestamp', order: 'descending' }"
            >
              <el-table-column prop="timestamp" label="时间" width="180">
                <template #default="{ row }">
                  {{ formatTimestamp(row.timestamp) }}
                </template>
              </el-table-column>
              <el-table-column prop="operator" label="操作者" width="100" />
              <el-table-column prop="operation_type" label="操作类型" width="120">
                <template #default="{ row }">
                  <el-tag :type="getOperationTagType(row.operation_type)" size="small">
                    {{ formatOperationType(row.operation_type) }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="target_key_id" label="目标Key" width="150" />
              <el-table-column label="权重变更" width="120">
                <template #default="{ row }">
                  <span class="weight-change">
                    {{ row.old_weight }} → {{ row.new_weight }}
                  </span>
                </template>
              </el-table-column>
              <el-table-column prop="reason" label="原因" show-overflow-tooltip />
              <el-table-column prop="source" label="来源" width="100">
                <template #default="{ row }">
                  <el-tag type="info" size="small">
                    {{ formatSource(row.source) }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column label="操作" width="100">
                <template #default="{ row }">
                  <el-button type="text" size="small" @click="showAuditDetail(row)">
                    详情
                  </el-button>
                </template>
              </el-table-column>
            </el-table>

            <el-pagination
              v-model:current-page="auditPagination.page"
              v-model:page-size="auditPagination.size"
              :page-sizes="[10, 20, 50, 100]"
              :total="auditPagination.total"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="handleAuditSizeChange"
              @current-change="handleAuditPageChange"
              class="pagination"
            />
          </div>
    </ContentCard>

    <!-- 批量编辑对话框 -->
    <el-dialog
      v-model="batchEditVisible"
      title="批量权重编辑"
      width="600px"
    >
      <div class="batch-edit-content">
        <el-form :model="batchEditForm" label-width="120px">
          <el-form-item label="操作类型">
            <el-radio-group v-model="batchEditForm.operation">
              <el-radio label="set">设置权重</el-radio>
              <el-radio label="increase">增加权重</el-radio>
              <el-radio label="decrease">减少权重</el-radio>
              <el-radio label="multiply">按比例调整</el-radio>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="数值">
            <el-input-number 
              v-model="batchEditForm.value" 
              :min="batchEditForm.operation === 'multiply' ? 0.1 : 0"
              :max="batchEditForm.operation === 'set' ? 1000 : undefined"
              :step="batchEditForm.operation === 'multiply' ? 0.1 : 10"
            />
            <span class="input-suffix">
              {{ batchEditForm.operation === 'multiply' ? '倍' : '' }}
            </span>
          </el-form-item>
          <el-form-item label="应用范围">
            <el-checkbox-group v-model="batchEditForm.targetKeys">
              <el-checkbox 
                v-for="apiKey in apiKeys" 
                :key="apiKey.id" 
                :label="apiKey.id"
              >
                {{ apiKey.id }} (当前: {{ apiKey.weight }})
              </el-checkbox>
            </el-checkbox-group>
          </el-form-item>
          <el-form-item label="操作原因">
            <el-input v-model="batchEditForm.reason" placeholder="请输入操作原因" />
          </el-form-item>
        </el-form>
      </div>
      <template #footer>
        <el-button @click="batchEditVisible = false">取消</el-button>
        <el-button type="primary" @click="applyBatchEdit">应用</el-button>
      </template>
    </el-dialog>

    <!-- 快照管理对话框 -->
    <el-dialog
      v-model="snapshotsVisible"
      title="快照管理"
      width="800px"
    >
      <div class="snapshots-content">
        <div class="snapshots-header">
          <el-button type="primary" @click="createSnapshot">
            <el-icon><Plus /></el-icon>
            创建新快照
          </el-button>
        </div>
        
        <el-table :data="snapshots" v-loading="snapshotsLoading">
          <el-table-column prop="snapshot_id" label="快照ID" width="200" />
          <el-table-column prop="description" label="描述" />
          <el-table-column prop="created_by" label="创建者" width="100" />
          <el-table-column prop="timestamp" label="创建时间" width="180">
            <template #default="{ row }">
              {{ formatTimestamp(row.timestamp) }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="200">
            <template #default="{ row }">
              <el-button type="text" size="small" @click="previewSnapshot(row)">预览</el-button>
              <el-button type="text" size="small" @click="rollbackToSnapshot(row)">回滚</el-button>
              <el-button type="text" size="small" @click="deleteSnapshot(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </el-dialog>
  </AppPage>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, nextTick, computed, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as echarts from 'echarts'
import type { ECharts } from 'echarts'
import AppPage from '../components/layout/AppPage.vue'
import ContentCard from '../components/layout/ContentCard.vue'
import StatCard from '../components/layout/StatCard.vue'
import {
  Key, CircleCheck, TrendCharts, ScaleToOriginal, Refresh, Check, Camera, Edit,
  MagicStick, View, Search, Download, FolderOpened, Plus, Close
} from '@element-plus/icons-vue'
import type {
  ApiKey, OptimizationResult, OptimizationStrategy, AuditRecord,
  Snapshot, BatchEditForm, AuditPagination
} from '../types/load-balancing'

// 数据定义
const apiKeys = ref<ApiKey[]>([])
const totalApiKeys = computed(() => apiKeys.value.length)
const activeApiKeys = computed(() => apiKeys.value.filter(key => key.enabled).length)
const totalWeight = computed(() => apiKeys.value.reduce((sum, key) => sum + key.weight, 0))
const loadBalanceScore = computed(() => {
  if (apiKeys.value.length === 0) return 0
  const weights = apiKeys.value.map(key => key.weight)
  const mean = weights.reduce((sum, w) => sum + w, 0) / weights.length
  const variance = weights.reduce((sum, w) => sum + Math.pow(w - mean, 2), 0) / weights.length
  return Math.max(0, 100 - variance / 10)
})

const loadBalanceScoreText = computed(() => loadBalanceScore.value.toFixed(0))

// 根据评分获取颜色
const getScoreColor = (score: number) => {
  if (score >= 80) return 'var(--color-success)'
  if (score >= 60) return 'var(--color-warning)'
  return 'var(--color-danger)'
}

// 加载状态
const weightsLoading = ref(false)
const savingWeights = ref(false)
const optimizationLoading = ref(false)
const auditLoading = ref(false)
const snapshotsLoading = ref(false)
const realTimeLoading = ref(false)

// 实时监控数据
const currentRequests = ref(0)
const failedRequests = ref(0)

// 智能优化相关
const selectedStrategy = ref('Balanced')
const optimizationStrategies = ref<OptimizationStrategy[]>([
  { value: 'ResponseTimeOptimized', label: '响应时间优化' },
  { value: 'ReliabilityOptimized', label: '可靠性优化' },
  { value: 'ThroughputOptimized', label: '吞吐量优化' },
  { value: 'Balanced', label: '综合均衡' },
  { value: 'Conservative', label: '保守优化' },
  { value: 'Aggressive', label: '激进优化' }
])
const optimizationResult = ref<OptimizationResult | null>(null)

// 图表相关
const chartType = ref('pie')
const chartContainer = ref(null)
let chart: ECharts | null = null

// 权重趋势分析相关
const trendTimeRange = ref('24h')
const trendChartContainer = ref(null)
let trendChart: ECharts | null = null
const weightVolatility = ref(8.5)
const adjustmentFrequency = ref(2.3)

// 审计相关
const auditRecords = ref<AuditRecord[]>([])
const auditPagination = reactive<AuditPagination>({
  page: 1,
  size: 20,
  total: 0
})

// 快照相关
const snapshots = ref<Snapshot[]>([])
const snapshotsVisible = ref(false)

// 批量编辑
const batchEditVisible = ref(false)
const batchEditForm = reactive<BatchEditForm>({
  operation: 'set',
  value: 100,
  targetKeys: [],
  reason: ''
})

// 方法定义
const refreshWeights = async () => {
  weightsLoading.value = true
  try {
    // 模拟API调用
    await new Promise(resolve => setTimeout(resolve, 1000))
    // 这里应该调用实际的API
    ElMessage.success('权重数据已刷新')
  } catch (error) {
    ElMessage.error('刷新权重数据失败')
  } finally {
    weightsLoading.value = false
  }
}

const refreshRealTimeData = async () => {
  realTimeLoading.value = true
  try {
    // 模拟API调用获取实时数据
    await new Promise(resolve => setTimeout(resolve, 800))
    
    // 模拟实时数据更新
    currentRequests.value = Math.floor(Math.random() * 100)
    failedRequests.value = Math.floor(Math.random() * 10)
    
    ElMessage.success('实时数据已刷新')
  } catch (error) {
    ElMessage.error('刷新实时数据失败')
  } finally {
    realTimeLoading.value = false
  }
}

const onWeightChange = (apiKey: ApiKey) => {
  console.log(`Weight changed for ${apiKey.id}: ${apiKey.weight}`)
}

const saveWeights = async () => {
  savingWeights.value = true
  try {
    // 模拟API调用
    await new Promise(resolve => setTimeout(resolve, 1500))
    ElMessage.success('权重配置已保存')
  } catch (error) {
    ElMessage.error('保存权重配置失败')
  } finally {
    savingWeights.value = false
  }
}

const generateOptimization = async () => {
  optimizationLoading.value = true
  try {
    // 模拟API调用
    await new Promise(resolve => setTimeout(resolve, 2000))
    
    // 模拟优化结果
    optimizationResult.value = {
      strategy: selectedStrategy.value,
      confidence_score: 0.85,
      overall_improvement: 12.5,
      recommendations: [
        {
          key_id: 'key-001',
          current_weight: 100,
          recommended_weight: 150,
          expected_improvement: 15.2,
          risk_level: 'Low',
          reason: '基于响应时间分析，建议增加权重以提升性能',
          confidence: 0.9
        },
        {
          key_id: 'key-002',
          current_weight: 200,
          recommended_weight: 180,
          expected_improvement: 8.7,
          risk_level: 'Medium',
          reason: '当前权重过高，建议适当降低以平衡负载',
          confidence: 0.8
        }
      ]
    }
    
    ElMessage.success('优化建议已生成')
  } catch (error) {
    ElMessage.error('生成优化建议失败')
  } finally {
    optimizationLoading.value = false
  }
}

const createSnapshot = async () => {
  try {
    await ElMessageBox.prompt('请输入快照描述', '创建快照', {
      confirmButtonText: '创建',
      cancelButtonText: '取消',
      inputPlaceholder: '快照描述'
    })
    
    // 模拟创建快照API调用
    await new Promise(resolve => setTimeout(resolve, 1000))
    ElMessage.success('快照创建成功')
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error('创建快照失败')
    }
  }
}

const openBatchEdit = () => {
  batchEditForm.targetKeys = []
  batchEditForm.operation = 'set'
  batchEditForm.value = 100
  batchEditForm.reason = ''
  batchEditVisible.value = true
}

const applyBatchEdit = async () => {
  if (batchEditForm.targetKeys.length === 0) {
    ElMessage.warning('请选择要编辑的API Key')
    return
  }
  
  try {
    // 模拟API调用
    await new Promise(resolve => setTimeout(resolve, 1000))
    ElMessage.success('批量编辑已应用')
    batchEditVisible.value = false
  } catch (error) {
    ElMessage.error('批量编辑失败')
  }
}

const initChart = () => {
  if (!chartContainer.value) return
  
  // 确保容器尺寸正确
  const container = chartContainer.value as HTMLElement
  container.style.width = '100%'
  container.style.height = '100%'
  
  chart = echarts.init(container)
  updateChart()
  
  // 自动调整图表大小
  const resizeObserver = new ResizeObserver(() => {
    if (chart) {
      chart.resize()
    }
  })
  resizeObserver.observe(container)
}

const updateChart = () => {
  if (!chart) return
  
  const data = apiKeys.value.map(key => ({
    name: key.id,
    value: key.weight
  }))
  
  let option = {}
  
  switch (chartType.value) {
    case 'pie':
      option = {
        tooltip: {
          trigger: 'item',
          formatter: '{a} <br/>{b}: {c} ({d}%)',
          backgroundColor: 'rgba(50, 50, 93, 0.9)',
          borderColor: '#409EFF',
          borderWidth: 1,
          textStyle: {
            color: '#fff',
            fontSize: 14
          }
        },
        legend: {
          orient: 'vertical',
          left: 'left',
          top: 'center',
          textStyle: {
            color: '#606266',
            fontSize: 14
          }
        },
        series: [{
          name: '权重分布',
          type: 'pie',
          radius: ['45%', '75%'],
          center: ['60%', '50%'],
          avoidLabelOverlap: false,
          label: {
            show: true,
            position: 'outside',
            formatter: '{b}: {c}',
            fontSize: 12,
            color: '#606266'
          },
          labelLine: {
            show: true,
            length: 15,
            length2: 10
          },
          emphasis: {
            itemStyle: {
              shadowBlur: 10,
              shadowOffsetX: 0,
              shadowColor: 'rgba(0, 0, 0, 0.5)'
            }
          },
          data,
          itemStyle: {
            borderRadius: 8,
            borderColor: '#fff',
            borderWidth: 2
          }
        }]
      }
      break
      
    case 'bar':
      option = {
        tooltip: {
          trigger: 'axis',
          backgroundColor: 'rgba(50, 50, 93, 0.9)',
          borderColor: '#409EFF',
          borderWidth: 1,
          textStyle: {
            color: '#fff',
            fontSize: 14
          }
        },
        grid: {
          left: '3%',
          right: '4%',
          bottom: '8%',
          top: '10%',
          containLabel: true
        },
        xAxis: {
          type: 'category',
          data: data.map(item => item.name),
          axisLine: {
            lineStyle: {
              color: '#E4E7ED'
            }
          },
          axisLabel: {
            color: '#606266',
            fontSize: 12
          }
        },
        yAxis: {
          type: 'value',
          name: '权重',
          nameTextStyle: {
            color: '#606266',
            fontSize: 14
          },
          axisLine: {
            lineStyle: {
              color: '#E4E7ED'
            }
          },
          axisLabel: {
            color: '#606266',
            fontSize: 12
          },
          splitLine: {
            lineStyle: {
              color: '#F2F6FC',
              type: 'dashed'
            }
          }
        },
        series: [{
          name: '权重',
          type: 'bar',
          data: data.map(item => item.value),
          itemStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: '#409EFF' },
              { offset: 1, color: '#66B3FF' }
            ]),
            borderRadius: [4, 4, 0, 0]
          },
          emphasis: {
            itemStyle: {
              color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                { offset: 0, color: '#337ECC' },
                { offset: 1, color: '#409EFF' }
              ])
            }
          }
        }]
      }
      break
      
    case 'radar':
      option = {
        tooltip: {},
        radar: {
          indicator: data.map(item => ({
            name: item.name,
            max: Math.max(...data.map(d => d.value)) * 1.2
          }))
        },
        series: [{
          name: '权重分布',
          type: 'radar',
          data: [{
            value: data.map(item => item.value),
            name: '当前权重'
          }]
        }]
      }
      break
  }
  
  chart.setOption(option)
}

// 权重趋势分析方法
const initTrendChart = () => {
  if (!trendChartContainer.value) return
  
  const container = trendChartContainer.value as HTMLElement
  container.style.width = '100%'
  container.style.height = '100%'
  
  trendChart = echarts.init(container)
  updateTrendChart()
  
  // 自动调整图表大小
  const resizeObserver = new ResizeObserver(() => {
    if (trendChart) {
      trendChart.resize()
    }
  })
  resizeObserver.observe(container)
}

const generateTrendData = () => {
  const now = Date.now()
  const timePoints = []
  const keyData: Record<string, number[]> = {}
  
  // 根据时间范围生成不同的数据点
  const ranges = {
    '1h': { points: 12, interval: 5 * 60 * 1000 }, // 5分钟间隔
    '6h': { points: 24, interval: 15 * 60 * 1000 }, // 15分钟间隔  
    '24h': { points: 24, interval: 60 * 60 * 1000 } // 1小时间隔
  }
  
  const config = ranges[trendTimeRange.value as keyof typeof ranges]
  
  // 生成时间点
  for (let i = config.points; i >= 0; i--) {
    timePoints.push(new Date(now - i * config.interval))
  }
  
  // 为每个API密钥生成权重变化数据
  apiKeys.value.forEach(key => {
    keyData[key.id] = []
    let baseWeight = key.weight
    
    timePoints.forEach((_, index) => {
      // 模拟权重波动，增加一些随机变化
      const variation = (Math.random() - 0.5) * 20 // ±10的随机变化
      const newWeight = Math.max(0, baseWeight + variation)
      keyData[key.id].push(Math.round(newWeight))
      baseWeight = newWeight * 0.8 + key.weight * 0.2 // 逐渐回归原始权重
    })
  })
  
  return { timePoints, keyData }
}

const updateTrendChart = () => {
  if (!trendChart) return
  
  const { timePoints, keyData } = generateTrendData()
  
  const series = Object.keys(keyData).map((keyId, index) => ({
    name: keyId,
    type: 'line',
    data: keyData[keyId],
    smooth: true,
    symbol: 'circle',
    symbolSize: 6,
    lineStyle: {
      width: 3
    },
    itemStyle: {
      color: ['#409EFF', '#67C23A', '#E6A23C', '#F56C6C', '#909399'][index % 5]
    }
  }))
  
  const option = {
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(50, 50, 93, 0.9)',
      borderColor: '#409EFF',
      borderWidth: 1,
      textStyle: {
        color: '#fff',
        fontSize: 12
      },
      formatter: (params: any) => {
        let result = `<div style="margin-bottom: 4px;">${params[0].axisValueLabel}</div>`
        params.forEach((item: any) => {
          result += `<div style="margin: 2px 0;">
            <span style="color: ${item.color};">●</span> 
            ${item.seriesName}: ${item.value}
          </div>`
        })
        return result
      }
    },
    legend: {
      data: Object.keys(keyData),
      textStyle: {
        color: '#606266',
        fontSize: 12
      },
      bottom: 0
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '15%',
      top: '10%',
      containLabel: true
    },
    xAxis: {
      type: 'category',
      data: timePoints.map(time => time.toLocaleTimeString('zh-CN', { 
        hour: '2-digit', 
        minute: '2-digit' 
      })),
      axisLine: {
        lineStyle: {
          color: '#E4E7ED'
        }
      },
      axisLabel: {
        color: '#606266',
        fontSize: 11
      }
    },
    yAxis: {
      type: 'value',
      name: '权重',
      nameTextStyle: {
        color: '#606266',
        fontSize: 12
      },
      axisLine: {
        lineStyle: {
          color: '#E4E7ED'
        }
      },
      axisLabel: {
        color: '#606266',
        fontSize: 11
      },
      splitLine: {
        lineStyle: {
          color: '#F2F6FC',
          type: 'dashed'
        }
      }
    },
    series
  }
  
  trendChart.setOption(option)
}

// 工具方法
const getStrategyLabel = (strategy: string) => {
  const strategyMap: Record<string, string> = {
    'ResponseTimeOptimized': '响应时间优化',
    'ReliabilityOptimized': '可靠性优化',
    'ThroughputOptimized': '吞吐量优化',
    'Balanced': '综合均衡',
    'Conservative': '保守优化',
    'Aggressive': '激进优化'
  }
  return strategyMap[strategy] || strategy
}

const getRiskTagType = (riskLevel: string) => {
  const riskMap: Record<string, string> = {
    'Low': 'success',
    'Medium': 'warning',
    'High': 'danger'
  }
  return riskMap[riskLevel] || 'info'
}

const getLoadPercentage = (apiKey: ApiKey) => {
  if (totalWeight.value === 0) return 0
  return Math.round((apiKey.weight / totalWeight.value) * 100)
}

const getProgressColor = (percentage: number) => {
  if (percentage < 20) return '#67C23A'
  if (percentage < 40) return '#E6A23C'
  return '#F56C6C'
}

const getRequestCount = (_apiKey: ApiKey) => {
  // 模拟请求数据
  return Math.floor(Math.random() * 1000)
}

const formatTimestamp = (timestamp: number) => {
  return new Date(timestamp * 1000).toLocaleString()
}

const formatOperationType = (type: string) => {
  const typeMap: Record<string, string> = {
    'Manual': '手动',
    'Intelligent': '智能',
    'Batch': '批量',
    'Rollback': '回滚',
    'Automatic': '自动'
  }
  return typeMap[type] || type
}

const getOperationTagType = (type: string) => {
  const typeMap: Record<string, string> = {
    'Manual': 'primary',
    'Intelligent': 'success',
    'Batch': 'warning',
    'Rollback': 'danger',
    'Automatic': 'info'
  }
  return typeMap[type] || 'info'
}

const formatSource = (source: string) => {
  const sourceMap: Record<string, string> = {
    'WebUI': '网页',
    'API': 'API',
    'ConfigFile': '配置',
    'Optimizer': '优化器',
    'Monitor': '监控'
  }
  return sourceMap[source] || source
}

// 审计相关方法
const openAuditQuery = () => {
  ElMessage.info('审计查询功能开发中...')
}

const exportAuditRecords = () => {
  ElMessage.info('导出功能开发中...')
}

const showSnapshots = () => {
  snapshotsVisible.value = true
  // 加载快照数据
  snapshots.value = [
    {
      snapshot_id: 'snap_001',
      description: '优化前备份',
      created_by: 'admin',
      timestamp: Date.now() / 1000 - 86400
    },
    {
      snapshot_id: 'snap_002', 
      description: '手动调整后',
      created_by: 'user1',
      timestamp: Date.now() / 1000 - 3600
    }
  ]
}

const showAuditDetail = (row: AuditRecord) => {
  ElMessage.info(`查看审计详情: ${row.id}`)
}

const handleAuditSizeChange = (size: number) => {
  auditPagination.size = size
  auditPagination.page = 1
}

const handleAuditPageChange = (page: number) => {
  auditPagination.page = page
}

const applyOptimization = () => {
  ElMessage.success('优化建议已应用')
}

const previewOptimization = () => {
  ElMessage.info('预览优化效果功能开发中...')
}

const previewSnapshot = (snapshot: Snapshot) => {
  ElMessage.info(`预览快照: ${snapshot.snapshot_id}`)
}

const rollbackToSnapshot = async (snapshot: Snapshot) => {
  try {
    await ElMessageBox.confirm(
      `确定要回滚到快照 "${snapshot.description}" 吗？这将覆盖当前权重配置。`,
      '确认回滚',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    
    ElMessage.success('回滚成功')
  } catch (error) {
    // 用户取消
  }
}

const deleteSnapshot = async (snapshot: Snapshot) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除快照 "${snapshot.description}" 吗？`,
      '确认删除',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    
    ElMessage.success('快照已删除')
  } catch (error) {
    // 用户取消
  }
}

// 生命周期
onMounted(async () => {
  // 初始化数据
  apiKeys.value = [
    { id: 'key-001', weight: 100, enabled: true },
    { id: 'key-002', weight: 200, enabled: true },
    { id: 'key-003', weight: 150, enabled: false },
    { id: 'key-004', weight: 120, enabled: true }
  ]
  
  // 初始化实时监控数据
  currentRequests.value = 0
  failedRequests.value = 0
  
  // 初始化图表
  await nextTick()
  setTimeout(() => {
    initChart()
    initTrendChart()
  }, 100)
  
  // 加载审计记录
  auditRecords.value = [
    {
      id: 'rec_001',
      timestamp: Date.now() / 1000 - 3600,
      operator: 'admin',
      operation_type: 'Manual',
      target_key_id: 'key-001',
      old_weight: 80,
      new_weight: 100,
      reason: '手动调整权重',
      source: 'WebUI'
    },
    {
      id: 'rec_002',
      timestamp: Date.now() / 1000 - 7200,
      operator: 'system',
      operation_type: 'Intelligent',
      target_key_id: 'key-002',
      old_weight: 180,
      new_weight: 200,
      reason: '智能优化建议',
      source: 'Optimizer'
    }
  ]
  auditPagination.total = auditRecords.value.length
})

// 监听图表类型变化
watch(chartType, () => {
  updateChart()
})

// 监听趋势图表时间范围变化
watch(trendTimeRange, () => {
  updateTrendChart()
})
</script>

<style scoped>
/* 页面整体布局优化 - 专业级设计系统 */
.overview-stats,
.config-monitoring-section,
.analysis-optimization-section,
.audit-section {
  margin-bottom: var(--spacing-extra-large);
}

/* 页面级统一间距和视觉层次 */
.app-page > * {
  margin-bottom: var(--spacing-extra-large);
}

.app-page > *:last-child {
  margin-bottom: 0;
}

/* 统计卡片网格 - 确保一致的间距 */
.overview-stats .el-col {
  margin-bottom: var(--spacing-medium);
}

/* 统一卡片容器样式 */
.el-card {
  border-radius: var(--border-radius-base);
  border: 1px solid var(--border-color-light);
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
  transition: all var(--transition-duration-base) var(--transition-function);
}

.el-card:hover {
  box-shadow: 0 6px 16px 0 rgba(0, 0, 0, 0.12);
  transform: translateY(-2px);
}

/* 统一卡片头部样式 */
.el-card__header {
  background-color: var(--bg-color);
  border-bottom: 1px solid var(--border-color-lighter);
  border-radius: var(--border-radius-base) var(--border-radius-base) 0 0;
}

/* 统一按钮组样式 */
.el-button-group {
  border-radius: var(--border-radius-base);
  overflow: hidden;
  box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
}

.el-button-group .el-button {
  border-radius: 0;
  border-left: none;
}

.el-button-group .el-button:first-child {
  border-left: 1px solid var(--border-color);
  border-radius: var(--border-radius-base) 0 0 var(--border-radius-base);
}

.el-button-group .el-button:last-child {
  border-radius: 0 var(--border-radius-base) var(--border-radius-base) 0;
}

/* 实时监控卡片样式 - 专业级视觉优化 */
.real-time-monitoring {
  margin-bottom: var(--spacing-extra-large);
  position: relative;
}

.real-time-monitoring::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 4px;
  background: linear-gradient(90deg, 
    var(--color-primary) 0%, 
    var(--color-success) 25%, 
    var(--color-warning) 50%, 
    var(--color-danger) 75%, 
    var(--color-info) 100%);
  border-radius: var(--border-radius-base) var(--border-radius-base) 0 0;
  z-index: 1;
}

.real-time-monitoring .el-row {
  margin: 0 -16px; /* 确保gutter正确应用 */
}

.real-time-monitoring .el-col {
  padding: 0 16px; /* 与gutter="32"配合 */
  margin-bottom: 0; /* 移除默认底部间距 */
}

/* StatCard 悬停增强效果 */
.real-time-monitoring .stat-card:hover {
  transform: translateY(-4px) scale(1.02);
  box-shadow: 0 8px 20px 0 rgba(0, 0, 0, 0.15);
}

/* 权重分析模块 - 符合Element Plus设计标准的等高布局 */
.weight-analysis-row {
  margin-bottom: var(--spacing-extra-large);
}

.weight-analysis-row .el-col {
  display: flex;
  align-items: stretch;
}

.chart-card {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 600px;
}

.chart-card .el-card__header {
  flex-shrink: 0;
  min-height: 80px;
}

.chart-card .el-card__body {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: var(--spacing-large);
}

/* 权重分布图表容器 */
.chart-container {
  width: 100%;
  flex: 1;
  min-height: 480px;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 权重趋势分析容器 */
.trend-stats {
  flex-shrink: 0;
  margin-bottom: var(--spacing-medium);
  padding: var(--spacing-medium);
  background-color: var(--bg-color-page);
  border-radius: var(--border-radius-base);
  border: 1px solid var(--border-color-lighter);
}

.trend-stat-item {
  text-align: center;
}

.trend-label {
  font-size: var(--font-size-small);
  color: var(--text-color-secondary);
  margin-bottom: var(--spacing-mini);
  font-weight: var(--font-weight-medium);
}

.trend-value {
  font-size: var(--font-size-medium);
  color: var(--text-color-primary);
  font-weight: var(--font-weight-semibold);
}

.trend-chart-container {
  width: 100%;
  flex: 1;
  min-height: 380px;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 图表加载状态 */
.chart-loading {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 300px;
}

/* 权重配置管理区域 - Element Plus设计优化 */
.weight-management {
  margin-bottom: var(--spacing-large);
}

.weight-item {
  display: flex;
  align-items: center;
  padding: var(--spacing-medium) 0;
  border-bottom: 1px solid var(--border-color-lighter);
  transition: background-color var(--transition-duration-base) var(--transition-function);
}

.weight-item:last-child {
  border-bottom: none;
}

.weight-item:hover {
  background-color: var(--bg-color-page);
  border-radius: var(--border-radius-base);
  margin: 0 calc(-1 * var(--spacing-small));
  padding-left: var(--spacing-small);
  padding-right: var(--spacing-small);
}

.weight-info {
  flex: 0 0 200px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.key-name {
  font-weight: var(--font-weight-medium);
  color: var(--text-color-primary);
  font-size: var(--font-size-base);
}

.weight-control {
  flex: 1;
  display: flex;
  align-items: center;
  margin-left: var(--spacing-large);
}

.weight-slider {
  flex: 1;
  margin-right: var(--spacing-medium);
}

.weight-value {
  flex: 0 0 50px;
  text-align: center;
  font-weight: var(--font-weight-semibold);
  color: var(--color-primary);
}

.panel-actions {
  padding-top: var(--spacing-medium);
  border-top: 1px solid var(--border-color-lighter);
}

.optimization-panel {
  min-height: 400px;
}

.strategy-selector {
  margin-bottom: var(--spacing-large);
}

.result-summary {
  margin-bottom: var(--spacing-large);
}

.summary-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--spacing-medium);
  padding: var(--spacing-medium);
  background-color: var(--bg-color-overlay);
  border-radius: var(--border-radius-base);
  border: 1px solid var(--border-color-lighter);
}

.summary-item {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-mini);
}

.summary-label {
  font-size: var(--font-size-extra-small);
  color: var(--text-color-secondary);
  font-weight: var(--font-weight-medium);
}

.summary-value {
  font-size: var(--font-size-small);
  color: var(--text-color-primary);
  font-weight: var(--font-weight-semibold);
}

.recommendations {
  margin-bottom: var(--spacing-large);
  max-height: 300px;
  overflow-y: auto;
}

.recommendation-item {
  padding: var(--spacing-medium);
  border: 1px solid var(--border-color-lighter);
  border-radius: var(--border-radius-base);
  margin-bottom: var(--spacing-small);
  background-color: var(--bg-color-page);
}

.rec-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-small);
}

.key-id {
  font-weight: var(--font-weight-medium);
  color: var(--text-color-primary);
}

.rec-changes {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-small);
}

.weight-change {
  color: var(--color-primary);
  font-weight: var(--font-weight-medium);
}

.improvement {
  color: var(--color-success);
  font-weight: var(--font-weight-medium);
}

.rec-reason {
  font-size: var(--font-size-extra-small);
  color: var(--text-color-secondary);
}

.no-optimization {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 300px;
}

/* 图表容器 - 符合Element Plus标准的简化样式 */


/* 审计表格区域 */
.audit-content {
  padding: 0;
}

.audit-content .el-table {
  border-radius: var(--border-radius-base);
}

.header-actions {
  display: flex;
  gap: var(--spacing-small);
  align-items: center;
}

.pagination {
  margin-top: var(--spacing-large);
  display: flex;
  justify-content: center;
}

.batch-edit-content {
  padding: var(--spacing-large) 0;
}

.input-suffix {
  margin-left: var(--spacing-small);
  color: var(--text-color-secondary);
}

.snapshots-content {
  padding: var(--spacing-small) 0;
}

.snapshots-header {
  margin-bottom: var(--spacing-large);
  text-align: right;
}

/* 桌面端专用样式 - 符合UI设计标准 */

/* Element Plus 标准交互效果 - 保持简洁 */
</style>