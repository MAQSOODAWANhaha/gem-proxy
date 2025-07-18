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
          :value-style="{ color: 'var(--color-primary)', fontSize: '24px', fontWeight: '600' }"
          :icon="Key"
          icon-color="var(--color-primary)"
          :span="6"
        />
        
        <StatCard
          title="活跃 Keys"
          :value="activeApiKeys"
          :value-style="{ color: 'var(--color-success)', fontSize: '24px', fontWeight: '600' }"
          :icon="CircleCheck"
          icon-color="var(--color-success)"
          :span="6"
        />
        
        <StatCard
          title="总权重"
          :value="totalWeight"
          :value-style="{ color: 'var(--color-warning)', fontSize: '24px', fontWeight: '600' }"
          :icon="ScaleToOriginal"
          icon-color="var(--color-warning)"
          :span="6"
        />
        
        <StatCard
          title="负载均衡评分"
          :value="loadBalanceScoreText + '/100'"
          :value-style="{ color: getScoreColor(loadBalanceScore), fontSize: '24px', fontWeight: '600' }"
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

    <!-- 第3行：实时负载状态监控 -->
    <ContentCard 
      title="实时负载状态" 
      description="监控API密钥的实时使用情况"
      :span="24"
    >
      <div class="load-status">
        <el-row :gutter="24">
          <el-col v-for="apiKey in apiKeys" :key="apiKey.id" :span="12" class="load-item-col">
            <div class="load-item">
              <div class="load-header">
                <span class="key-name">{{ apiKey.id }}</span>
                <span class="load-percentage">{{ getLoadPercentage(apiKey) }}%</span>
              </div>
              <el-progress 
                :percentage="getLoadPercentage(apiKey)" 
                :color="getProgressColor(getLoadPercentage(apiKey))"
                :stroke-width="8"
              />
              <div class="load-stats">
                <span>权重: {{ apiKey.weight }}</span>
                <span>请求数: {{ getRequestCount(apiKey) }}</span>
              </div>
            </div>
          </el-col>
        </el-row>
      </div>
    </ContentCard>

    <!-- 第4行：可视化分析 + 智能优化建议 -->
    <el-row :gutter="32">
      <!-- 左侧：权重分布可视化 -->
      <ContentCard 
        title="权重分布可视化" 
        description="多维度权重分布分析图表"
        :span="12"
      >
          <template #actions>
            <el-button-group size="small">
              <el-button @click="chartType = 'pie'" :type="chartType === 'pie' ? 'primary' : ''">饼图</el-button>
              <el-button @click="chartType = 'bar'" :type="chartType === 'bar' ? 'primary' : ''">柱状图</el-button>
              <el-button @click="chartType = 'radar'" :type="chartType === 'radar' ? 'primary' : ''">雷达图</el-button>
            </el-button-group>
          </template>

          <div class="chart-container">
            <div ref="chartContainer" style="width: 100%; height: 400px;"></div>
          </div>
        </ContentCard>

      <!-- 右侧：智能优化建议 -->
      <ContentCard 
        title="智能优化建议" 
        description="基于性能数据的智能权重优化建议"
        :span="12"
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
    </el-row>

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
  MagicStick, View, Search, Download, FolderOpened, Plus
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
  
  chart = echarts.init(chartContainer.value)
  updateChart()
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
          formatter: '{a} <br/>{b}: {c} ({d}%)'
        },
        series: [{
          name: '权重分布',
          type: 'pie',
          radius: ['40%', '70%'],
          avoidLabelOverlap: false,
          label: {
            show: false,
            position: 'center'
          },
          emphasis: {
            label: {
              show: true,
              fontSize: '18',
              fontWeight: 'bold'
            }
          },
          labelLine: {
            show: false
          },
          data
        }]
      }
      break
      
    case 'bar':
      option = {
        tooltip: {
          trigger: 'axis'
        },
        xAxis: {
          type: 'category',
          data: data.map(item => item.name)
        },
        yAxis: {
          type: 'value',
          name: '权重'
        },
        series: [{
          name: '权重',
          type: 'bar',
          data: data.map(item => item.value),
          itemStyle: {
            color: '#409EFF'
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
  
  // 初始化图表
  await nextTick()
  initChart()
  
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
</script>

<style scoped>
/* 页面整体布局 */
.overview-stats {
  margin-bottom: var(--spacing-extra-large);
}

.config-monitoring-section {
  margin-bottom: var(--spacing-extra-large);
}

.analysis-optimization-section {
  margin-bottom: var(--spacing-extra-large);
}

.audit-section {
  margin-bottom: var(--spacing-extra-large);
}

/* 统计卡片网格 */
.overview-stats .el-col {
  margin-bottom: var(--spacing-medium);
}

/* 权重配置管理区域 */

.weight-management {
  margin-bottom: var(--spacing-large);
}

.weight-item {
  display: flex;
  align-items: center;
  padding: var(--spacing-medium) 0;
  border-bottom: 1px solid var(--border-color-lighter);
}

.weight-item:last-child {
  border-bottom: none;
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

/* 可视化图表容器 */

.chart-container {
  padding: var(--spacing-large) 0;
}

.load-status {
  padding: var(--spacing-small) 0;
}

.load-item {
  margin-bottom: var(--spacing-large);
}

.load-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-small);
}

.load-stats {
  display: flex;
  justify-content: space-between;
  font-size: var(--font-size-extra-small);
  color: var(--text-color-secondary);
  margin-top: var(--spacing-mini);
}

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

/* 响应式布局优化 */
@media (max-width: 1200px) {
  .overview-stats .el-col {
    margin-bottom: var(--spacing-large);
  }
  
  .config-monitoring-section .el-col,
  .analysis-optimization-section .el-col {
    margin-bottom: var(--spacing-large);
  }
  
  .overview-stats .el-col {
    flex: 0 0 50%;
    max-width: 50%;
  }
}

@media (max-width: 768px) {
  .overview-stats .el-col {
    flex: 0 0 100%;
    max-width: 100%;
  }
  
  .config-monitoring-section .el-col,
  .analysis-optimization-section .el-col {
    flex: 0 0 100%;
    max-width: 100%;
  }
  
  .header-actions {
    flex-direction: column;
    gap: var(--spacing-mini);
  }
  
  .summary-grid {
    grid-template-columns: 1fr;
    gap: var(--spacing-small);
    padding: var(--spacing-small);
  }
  
  .weight-info {
    flex: 0 0 auto;
    margin-bottom: var(--spacing-small);
  }
  
  .weight-control {
    margin-left: 0;
  }
  
  .weight-item {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>