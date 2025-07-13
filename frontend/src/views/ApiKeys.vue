<template>
  <div class="api-keys">
    <div class="page-header">
      <h1>API 密钥管理</h1>
      <el-button 
        type="primary" 
        @click="showAddDialog = true"
        :icon="Plus"
      >
        添加密钥
      </el-button>
    </div>

    <!-- 统计信息 -->
    <el-row :gutter="16" class="stats-row">
      <el-col :span="6">
        <el-statistic title="总密钥数" :value="apiKeys.length" />
      </el-col>
      <el-col :span="6">
        <el-statistic title="活跃密钥" :value="activeKeysCount" />
      </el-col>
      <el-col :span="6">
        <el-statistic title="总权重" :value="totalWeight" />
      </el-col>
      <el-col :span="6">
        <el-statistic title="每分钟限额" :value="totalRateLimit" />
      </el-col>
    </el-row>

    <!-- 权重管理工具栏 -->
    <el-card class="weight-management-card">
      <template #header>
        <div class="card-header">
          <span>权重管理</span>
          <div class="header-actions">
            <el-button 
              size="small" 
              @click="loadWeightOptimization"
              :loading="configStore.weightLoading"
            >
              <template #icon>
                <el-icon><View /></el-icon>
              </template>
              优化建议
            </el-button>
            <el-button 
              size="small" 
              type="primary"
              @click="rebalanceWeights"
              :loading="configStore.weightLoading"
            >
              <template #icon>
                <el-icon><Connection /></el-icon>
              </template>
              智能重平衡
            </el-button>
          </div>
        </div>
      </template>

      <!-- 权重分配图表 -->
      <el-row :gutter="20">
        <el-col :span="12">
          <div class="chart-container">
            <h4>权重分配比例</h4>
            <div id="weight-pie-chart" style="height: 300px;"></div>
          </div>
        </el-col>
        <el-col :span="12">
          <div class="chart-container">
            <h4>负载均衡效果评分</h4>
            <div class="effectiveness-score">
              <el-progress 
                type="circle" 
                :percentage="loadBalanceEffectiveness" 
                :color="getEffectivenessColor(loadBalanceEffectiveness)"
                :width="120"
              />
              <div class="score-description">
                <p>{{ getEffectivenessDescription(loadBalanceEffectiveness) }}</p>
              </div>
            </div>
          </div>
        </el-col>
      </el-row>

      <!-- 优化建议 -->
      <div v-if="configStore.weightOptimization && configStore.weightOptimization.optimization_needed" class="optimization-suggestions">
        <h4>优化建议</h4>
        <el-alert
          title="发现权重优化机会"
          type="info"
          :description="`总体评分: ${configStore.weightOptimization.overall_score.toFixed(1)}/100`"
          show-icon
          :closable="false"
        />
        <div class="suggestions-list">
          <el-card 
            v-for="suggestion in configStore.weightOptimization.suggestions" 
            :key="suggestion.key_id"
            class="suggestion-card"
          >
            <div class="suggestion-content">
              <div class="suggestion-header">
                <strong>{{ suggestion.key_id }}</strong>
                <el-tag type="warning">{{ suggestion.current_weight }} → {{ suggestion.suggested_weight }}</el-tag>
              </div>
              <p class="suggestion-reason">{{ suggestion.reason }}</p>
              <p class="suggestion-impact">{{ suggestion.impact }}</p>
              <el-button 
                size="small" 
                type="primary"
                @click="applyOptimizationSuggestion(suggestion)"
                :loading="configStore.weightLoading"
              >
                应用建议
              </el-button>
            </div>
          </el-card>
        </div>
      </div>
    </el-card>

    <!-- 密钥列表 -->
    <el-card v-loading="configStore.loading">
      <el-table 
        :data="apiKeys" 
        style="width: 100%"
        :row-key="(row: ApiKey) => row.id"
      >
        <el-table-column prop="id" label="密钥 ID" width="120" />
        
        <el-table-column label="API 密钥" min-width="200">
          <template #default="{ row }">
            <div class="api-key-cell">
              <span class="masked-key">{{ maskApiKey(row.key) }}</span>
              <el-button
                type="text"
                size="small"
                @click="toggleKeyVisibility(row.id)"
                class="toggle-btn"
              >
                <el-icon>
                  <View v-if="!visibleKeys.has(row.id)" />
                  <Hide v-else />
                </el-icon>
              </el-button>
              <el-button
                type="text"
                size="small"
                @click="copyApiKey(row.key)"
                class="copy-btn"
              >
                <el-icon><CopyDocument /></el-icon>
              </el-button>
            </div>
            <div v-if="visibleKeys.has(row.id)" class="full-key">
              {{ row.key }}
            </div>
          </template>
        </el-table-column>
        
        <el-table-column label="权重" width="120">
          <template #default="{ row }">
            <div class="weight-cell">
              <el-input-number
                v-if="editingWeight === row.id"
                v-model="tempWeight"
                :min="0"
                :max="1000"
                size="small"
                @blur="saveWeight(row.id)"
                @keyup.enter="saveWeight(row.id)"
                style="width: 80px;"
              />
              <div v-else class="weight-display" @click="startEditWeight(row.id, row.weight)">
                <el-tag :type="row.weight > 0 ? 'success' : 'info'">
                  {{ row.weight }}
                </el-tag>
                <el-icon class="edit-icon"><Edit /></el-icon>
              </div>
            </div>
          </template>
        </el-table-column>
        
        <el-table-column prop="max_requests_per_minute" label="每分钟限额" width="120" />
        
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="row.weight > 0 ? 'success' : 'info'">
              {{ row.weight > 0 ? '活跃' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        
        <el-table-column label="操作" width="200">
          <template #default="{ row }">
            <el-button 
              size="small" 
              @click="editApiKey(row)"
              :icon="Edit"
            >
              编辑
            </el-button>
            <el-button 
              size="small" 
              type="success" 
              @click="testApiKey(row)"
              :loading="testingKeys.has(row.id)"
              :icon="Connection"
            >
              测试
            </el-button>
            <el-popconfirm
              title="确定删除这个 API 密钥吗？"
              @confirm="removeApiKey(row.id)"
            >
              <template #reference>
                <el-button 
                  size="small" 
                  type="danger"
                  :icon="Delete"
                >
                  删除
                </el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 添加/编辑密钥对话框 -->
    <el-dialog
      v-model="showAddDialog"
      :title="editingKey ? '编辑 API 密钥' : '添加 API 密钥'"
      width="500px"
    >
      <el-form 
        ref="keyFormRef"
        :model="keyForm" 
        :rules="keyFormRules"
        label-width="120px"
      >
        <el-form-item label="密钥 ID" prop="id">
          <el-input 
            v-model="keyForm.id" 
            placeholder="请输入密钥 ID"
            :disabled="!!editingKey"
          />
        </el-form-item>
        
        <el-form-item label="API 密钥" prop="key">
          <el-input 
            v-model="keyForm.key" 
            type="textarea"
            :rows="3"
            placeholder="请输入 Gemini API 密钥"
          />
        </el-form-item>
        
        <el-form-item label="权重" prop="weight">
          <el-input-number 
            v-model="keyForm.weight" 
            :min="0"
            :max="1000"
            style="width: 100%"
          />
          <div class="form-help">设置为 0 表示停用此密钥</div>
        </el-form-item>
        
        <el-form-item label="每分钟限额" prop="max_requests_per_minute">
          <el-input-number 
            v-model="keyForm.max_requests_per_minute" 
            :min="1"
            :max="10000"
            style="width: 100%"
          />
          <div class="form-help">此密钥每分钟最大请求数</div>
        </el-form-item>
      </el-form>
      
      <template #footer>
        <el-button @click="closeKeyDialog">取消</el-button>
        <el-button 
          type="primary" 
          @click="saveApiKey"
          :loading="configStore.loading"
        >
          {{ editingKey ? '更新' : '添加' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { ElMessage, type FormInstance } from 'element-plus'
import { 
  Plus, 
  Edit, 
  Delete, 
  View, 
  Hide, 
  CopyDocument, 
  Connection
} from '@element-plus/icons-vue'
import { useConfigStore } from '../stores/config'
import { configApi } from '../api/config'
import type { ApiKey, WeightOptimizationSuggestion } from '../types'
import * as echarts from 'echarts'

const configStore = useConfigStore()
const keyFormRef = ref<FormInstance>()

// 状态
const showAddDialog = ref(false)
const editingKey = ref<ApiKey | null>(null)
const visibleKeys = ref(new Set<string>())
const testingKeys = ref(new Set<string>())

// 权重编辑状态
const editingWeight = ref<string | null>(null)
const tempWeight = ref(0)

// 图表实例
let pieChart: echarts.ECharts | null = null

// 表单数据
const keyForm = ref({
  id: '',
  key: '',
  weight: 100,
  max_requests_per_minute: 60
})

// 表单验证规则
const keyFormRules = {
  id: [
    { required: true, message: '请输入密钥 ID', trigger: 'blur' },
    { min: 2, max: 50, message: 'ID 长度在 2 到 50 个字符', trigger: 'blur' }
  ],
  key: [
    { required: true, message: '请输入 API 密钥', trigger: 'blur' },
    { min: 20, message: 'API 密钥长度至少 20 个字符', trigger: 'blur' }
  ],
  weight: [
    { required: true, message: '请输入权重', trigger: 'blur' },
    { type: 'number', min: 0, max: 1000, message: '权重范围: 0-1000', trigger: 'blur' }
  ],
  max_requests_per_minute: [
    { required: true, message: '请输入每分钟限额', trigger: 'blur' },
    { type: 'number', min: 1, max: 10000, message: '限额范围: 1-10000', trigger: 'blur' }
  ]
}

// 计算属性
const apiKeys = computed(() => configStore.config?.gemini.api_keys || [])
const activeKeysCount = computed(() => apiKeys.value.filter(key => key.weight > 0).length)
const totalWeight = computed(() => apiKeys.value.reduce((sum, key) => sum + key.weight, 0))
const totalRateLimit = computed(() => 
  apiKeys.value.reduce((sum, key) => sum + key.max_requests_per_minute, 0)
)

// 权重管理相关计算属性
const loadBalanceEffectiveness = computed(() => {
  return configStore.weightStats?.load_balance_effectiveness || 0
})

const weightChartData = computed(() => {
  const activeKeys = apiKeys.value.filter(key => key.weight > 0)
  return activeKeys.map(key => ({
    name: key.id,
    value: key.weight
  }))
})

// 方法
function maskApiKey(key: string): string {
  if (key.length <= 8) return key
  return key.slice(0, 4) + '*'.repeat(key.length - 8) + key.slice(-4)
}

function toggleKeyVisibility(keyId: string) {
  if (visibleKeys.value.has(keyId)) {
    visibleKeys.value.delete(keyId)
  } else {
    visibleKeys.value.add(keyId)
  }
}

async function copyApiKey(key: string) {
  try {
    await navigator.clipboard.writeText(key)
    ElMessage.success('API 密钥已复制到剪贴板')
  } catch (error) {
    ElMessage.error('复制失败，请手动复制')
  }
}

function editApiKey(key: ApiKey) {
  editingKey.value = key
  keyForm.value = { ...key }
  showAddDialog.value = true
}

async function removeApiKey(keyId: string) {
  try {
    await configStore.removeApiKey(keyId)
    ElMessage.success('API 密钥删除成功')
  } catch (error) {
    console.error('删除密钥失败:', error)
    ElMessage.error('删除密钥失败')
  }
}

async function testApiKey(key: ApiKey) {
  testingKeys.value.add(key.id)
  try {
    const isValid = await configApi.testApiKey(key.key)
    if (isValid) {
      ElMessage.success(`密钥 ${key.id} 测试通过`)
    } else {
      ElMessage.error(`密钥 ${key.id} 测试失败`)
    }
  } catch (error) {
    console.error('测试密钥失败:', error)
    ElMessage.error('测试密钥失败')
  } finally {
    testingKeys.value.delete(key.id)
  }
}

async function saveApiKey() {
  if (!keyFormRef.value) return
  
  try {
    await keyFormRef.value.validate()
    
    if (editingKey.value) {
      // 更新现有密钥
      await configStore.updateApiKey(editingKey.value.id, keyForm.value)
      ElMessage.success('API 密钥更新成功')
    } else {
      // 检查 ID 是否已存在
      const existingKey = apiKeys.value.find(key => key.id === keyForm.value.id)
      if (existingKey) {
        ElMessage.error('密钥 ID 已存在')
        return
      }
      
      // 添加新密钥
      await configStore.addApiKey({
        key: keyForm.value.key,
        weight: keyForm.value.weight,
        max_requests_per_minute: keyForm.value.max_requests_per_minute
      })
      ElMessage.success('API 密钥添加成功')
    }
    
    closeKeyDialog()
  } catch (error) {
    console.error('保存密钥失败:', error)
    ElMessage.error('保存密钥失败')
  }
}

function closeKeyDialog() {
  showAddDialog.value = false
  editingKey.value = null
  keyForm.value = {
    id: '',
    key: '',
    weight: 100,
    max_requests_per_minute: 60
  }
  keyFormRef.value?.clearValidate()
}

// 权重管理相关方法
function startEditWeight(keyId: string, currentWeight: number) {
  editingWeight.value = keyId
  tempWeight.value = currentWeight
  nextTick(() => {
    // 自动聚焦到输入框
    const input = document.querySelector('.el-input-number__input') as HTMLInputElement
    if (input) {
      input.focus()
      input.select()
    }
  })
}

async function saveWeight(keyId: string) {
  if (editingWeight.value !== keyId) return
  
  try {
    await configStore.updateKeyWeight(keyId, tempWeight.value)
    ElMessage.success('权重更新成功')
    // 更新图表
    updateWeightChart()
  } catch (error) {
    console.error('更新权重失败:', error)
    ElMessage.error('更新权重失败')
  } finally {
    editingWeight.value = null
  }
}

async function loadWeightOptimization() {
  try {
    await configStore.loadWeightOptimization()
    ElMessage.success('优化建议加载成功')
  } catch (error) {
    console.error('加载优化建议失败:', error)
    ElMessage.error('加载优化建议失败')
  }
}

async function rebalanceWeights() {
  try {
    await configStore.rebalanceWeights()
    ElMessage.success('智能重平衡完成')
    // 更新图表
    updateWeightChart()
  } catch (error) {
    console.error('重平衡失败:', error)
    ElMessage.error('重平衡失败')
  }
}

async function applyOptimizationSuggestion(suggestion: WeightOptimizationSuggestion) {
  try {
    await configStore.updateKeyWeight(suggestion.key_id, suggestion.suggested_weight)
    ElMessage.success(`已将 ${suggestion.key_id} 权重调整为 ${suggestion.suggested_weight}`)
    // 重新加载优化建议
    await configStore.loadWeightOptimization()
    // 更新图表
    updateWeightChart()
  } catch (error) {
    console.error('应用优化建议失败:', error)
    ElMessage.error('应用优化建议失败')
  }
}

function getEffectivenessColor(score: number) {
  if (score >= 80) return '#67c23a'
  if (score >= 60) return '#e6a23c'
  return '#f56c6c'
}

function getEffectivenessDescription(score: number) {
  if (score >= 80) return '负载均衡效果优秀'
  if (score >= 60) return '负载均衡效果良好'
  if (score >= 40) return '负载均衡效果一般'
  return '建议优化权重分配'
}

function initWeightChart() {
  const chartDom = document.getElementById('weight-pie-chart')
  if (!chartDom) return
  
  pieChart = echarts.init(chartDom)
  updateWeightChart()
}

function updateWeightChart() {
  if (!pieChart) return
  
  const option = {
    title: {
      text: `总权重: ${totalWeight.value}`,
      left: 'center',
      textStyle: {
        fontSize: 14,
        color: '#666'
      }
    },
    tooltip: {
      trigger: 'item',
      formatter: '{a} <br/>{b}: {c} ({d}%)'
    },
    series: [
      {
        name: '权重分配',
        type: 'pie',
        radius: ['40%', '70%'],
        center: ['50%', '60%'],
        data: weightChartData.value,
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowOffsetX: 0,
            shadowColor: 'rgba(0, 0, 0, 0.5)'
          }
        },
        label: {
          show: true,
          formatter: '{b}\n{c} ({d}%)'
        }
      }
    ]
  }
  
  pieChart.setOption(option)
}

// 初始化
onMounted(async () => {
  if (!configStore.config) {
    await configStore.loadConfig()
  }
  
  // 加载权重统计
  try {
    await configStore.loadWeightStats()
  } catch (error) {
    console.error('加载权重统计失败:', error)
  }
  
  // 初始化图表
  nextTick(() => {
    initWeightChart()
  })
})
</script>

<style scoped>
.api-keys {
  max-width: 1200px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.stats-row {
  margin-bottom: 24px;
}

.api-key-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.masked-key {
  font-family: 'Courier New', monospace;
  flex: 1;
}

.toggle-btn,
.copy-btn {
  padding: 4px;
  margin-left: 4px;
}

.full-key {
  font-family: 'Courier New', monospace;
  font-size: 12px;
  color: #666;
  background-color: #f5f5f5;
  padding: 4px 8px;
  border-radius: 4px;
  margin-top: 8px;
  word-break: break-all;
}

.form-help {
  font-size: 12px;
  color: #6b7280;
  margin-top: 4px;
}

:deep(.el-statistic__content) {
  font-size: 24px;
  font-weight: 600;
}

:deep(.el-statistic__head) {
  color: #6b7280;
  margin-bottom: 8px;
}

/* 权重管理样式 */
.weight-management-card {
  margin-bottom: 24px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.chart-container {
  text-align: center;
}

.chart-container h4 {
  margin-bottom: 16px;
  color: #1f2937;
}

.effectiveness-score {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.score-description p {
  margin: 0;
  font-size: 14px;
  color: #6b7280;
}

.optimization-suggestions {
  margin-top: 24px;
}

.optimization-suggestions h4 {
  margin-bottom: 16px;
  color: #1f2937;
}

.suggestions-list {
  display: grid;
  gap: 12px;
  margin-top: 16px;
}

.suggestion-card {
  border-left: 4px solid #e6a23c;
}

.suggestion-content {
  padding: 16px;
}

.suggestion-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.suggestion-reason {
  margin: 8px 0;
  color: #374151;
  font-size: 14px;
}

.suggestion-impact {
  margin: 8px 0 12px;
  color: #6b7280;
  font-size: 12px;
}

/* 权重编辑样式 */
.weight-cell {
  display: flex;
  align-items: center;
  gap: 4px;
}

.weight-display {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  transition: opacity 0.2s;
}

.weight-display:hover {
  opacity: 0.7;
}

.edit-icon {
  opacity: 0;
  transition: opacity 0.2s;
  font-size: 12px;
  color: #6b7280;
}

.weight-display:hover .edit-icon {
  opacity: 1;
}
</style>