<template>
  <div class="proxy-config">
    <h1>代理配置</h1>
    
    <el-card v-loading="configStore.loading">
      <el-form 
        v-if="localConfig"
        ref="formRef"
        :model="localConfig" 
        :rules="formRules"
        label-width="120px"
        class="config-form"
      >
        <!-- 服务器配置 -->
        <el-divider content-position="left">
          <h3>服务器配置</h3>
        </el-divider>
        
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="监听地址" prop="server.host">
              <el-input 
                v-model="localConfig.server.host" 
                placeholder="例: 0.0.0.0"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="监听端口" prop="server.port">
              <el-input-number 
                v-model="localConfig.server.port" 
                :min="1"
                :max="65535"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="工作进程" prop="server.workers">
              <el-input-number 
                v-model="localConfig.server.workers" 
                :min="1"
                :max="16"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最大连接数" prop="server.max_connections">
              <el-input-number 
                v-model="localConfig.server.max_connections" 
                :min="100"
                :max="100000"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>

        <!-- Gemini 配置 -->
        <el-divider content-position="left">
          <h3>Gemini API 配置</h3>
        </el-divider>
        
        <el-row :gutter="20">
          <el-col :span="16">
            <el-form-item label="API 基础地址" prop="gemini.base_url">
              <el-input 
                v-model="localConfig.gemini.base_url" 
                placeholder="例: generativelanguage.googleapis.com:443"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="超时时间(秒)" prop="gemini.timeout_seconds">
              <el-input-number 
                v-model="localConfig.gemini.timeout_seconds" 
                :min="5"
                :max="300"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>

        <!-- 认证配置 -->
        <el-divider content-position="left">
          <h3>认证配置</h3>
        </el-divider>
        
        <el-form-item label="启用认证" prop="auth.enabled">
          <el-switch v-model="localConfig.auth.enabled" />
        </el-form-item>
        
        <template v-if="localConfig.auth.enabled">
          <el-form-item label="JWT 密钥" prop="auth.jwt_secret">
            <el-input 
              v-model="localConfig.auth.jwt_secret" 
              type="password"
              show-password
              placeholder="请输入 JWT 签名密钥"
            />
          </el-form-item>
          
          <el-form-item label="速率限制" prop="auth.rate_limit_per_minute">
            <el-input-number 
              v-model="localConfig.auth.rate_limit_per_minute" 
              :min="1"
              :max="10000"
              style="width: 200px"
            />
            <span class="form-help">每分钟最大请求数</span>
          </el-form-item>
        </template>

        <!-- 监控配置 -->
        <el-divider content-position="left">
          <h3>监控配置</h3>
        </el-divider>
        
        <el-form-item label="启用监控" prop="metrics.enabled">
          <el-switch v-model="localConfig.metrics.enabled" />
        </el-form-item>
        
        <el-form-item 
          v-if="localConfig.metrics.enabled" 
          label="监控端口" 
          prop="metrics.prometheus_port"
        >
          <el-input-number 
            v-model="localConfig.metrics.prometheus_port" 
            :min="1024"
            :max="65535"
            style="width: 200px"
          />
          <span class="form-help">Prometheus 指标端口</span>
        </el-form-item>

        <!-- TLS 配置 -->
        <el-divider content-position="left">
          <h3>TLS 配置</h3>
        </el-divider>
        
        <el-form-item label="启用 TLS" prop="server.tls.enabled">
          <el-switch v-model="localConfig.server.tls.enabled" />
        </el-form-item>
        
        <template v-if="localConfig.server.tls.enabled">
          <el-row :gutter="20">
            <el-col :span="12">
              <el-form-item label="证书文件路径" prop="server.tls.cert_path">
                <el-input 
                  v-model="localConfig.server.tls.cert_path" 
                  placeholder="例: /path/to/cert.pem"
                />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="私钥文件路径" prop="server.tls.key_path">
                <el-input 
                  v-model="localConfig.server.tls.key_path" 
                  placeholder="例: /path/to/key.pem"
                />
              </el-form-item>
            </el-col>
          </el-row>
        </template>

        <!-- 操作按钮 -->
        <el-form-item class="form-actions">
          <el-button 
            type="primary" 
            @click="saveConfig"
            :loading="configStore.loading"
          >
            保存配置
          </el-button>
          <el-button @click="resetForm">重置</el-button>
          <el-button 
            type="info" 
            @click="loadConfig"
            :loading="configStore.loading"
          >
            重新加载
          </el-button>
        </el-form-item>
      </el-form>
      
      <div v-else class="loading-placeholder">
        <el-empty description="配置加载中..." />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance } from 'element-plus'
import { useConfigStore } from '../stores/config'
import type { ProxyConfig } from '../types'

const configStore = useConfigStore()
const formRef = ref<FormInstance>()

// 本地配置副本
const localConfig = ref<ProxyConfig | null>(null)

// 表单验证规则
const formRules = {
  'server.host': [
    { required: true, message: '请输入监听地址', trigger: 'blur' }
  ],
  'server.port': [
    { required: true, message: '请输入监听端口', trigger: 'blur' },
    { type: 'number', min: 1, max: 65535, message: '端口范围: 1-65535', trigger: 'blur' }
  ],
  'server.workers': [
    { required: true, message: '请输入工作进程数', trigger: 'blur' },
    { type: 'number', min: 1, max: 16, message: '工作进程数范围: 1-16', trigger: 'blur' }
  ],
  'server.max_connections': [
    { required: true, message: '请输入最大连接数', trigger: 'blur' },
    { type: 'number', min: 100, max: 100000, message: '最大连接数范围: 100-100000', trigger: 'blur' }
  ],
  'gemini.base_url': [
    { required: true, message: '请输入 Gemini API 基础地址', trigger: 'blur' }
  ],
  'gemini.timeout_seconds': [
    { required: true, message: '请输入超时时间', trigger: 'blur' },
    { type: 'number', min: 5, max: 300, message: '超时时间范围: 5-300秒', trigger: 'blur' }
  ],
  'auth.jwt_secret': [
    { 
      validator: (_rule: any, value: string, callback: any) => {
        if (localConfig.value?.auth.enabled && (!value || value.length < 16)) {
          callback(new Error('JWT 密钥长度至少 16 位'))
        } else {
          callback()
        }
      }, 
      trigger: 'blur' 
    }
  ],
  'auth.rate_limit_per_minute': [
    { type: 'number', min: 1, max: 10000, message: '速率限制范围: 1-10000', trigger: 'blur' }
  ],
  'metrics.prometheus_port': [
    { type: 'number', min: 1024, max: 65535, message: '端口范围: 1024-65535', trigger: 'blur' }
  ]
}

// 监听配置变化，同步到本地副本
watch(() => configStore.config, (newConfig) => {
  if (newConfig) {
    localConfig.value = JSON.parse(JSON.stringify(newConfig))
  }
}, { immediate: true, deep: true })

// 方法
async function saveConfig() {
  if (!localConfig.value || !formRef.value) return
  
  try {
    // 表单验证
    await formRef.value.validate()
    
    // 确认保存
    await ElMessageBox.confirm(
      '确定要保存配置吗？这将会重新启动代理服务。',
      '确认保存',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    
    // 保存配置
    await configStore.updateConfig(localConfig.value)
    ElMessage.success('配置保存成功')
    
  } catch (error) {
    if (error !== 'cancel') {
      console.error('保存配置失败:', error)
      ElMessage.error('保存配置失败')
    }
  }
}

function resetForm() {
  if (configStore.config) {
    localConfig.value = JSON.parse(JSON.stringify(configStore.config))
    ElMessage.info('已重置为原始配置')
  }
}

async function loadConfig() {
  try {
    await configStore.loadConfig()
    ElMessage.success('配置重新加载成功')
  } catch (error) {
    console.error('加载配置失败:', error)
    ElMessage.error('加载配置失败')
  }
}

// 初始化
onMounted(() => {
  if (!configStore.config) {
    loadConfig()
  }
})
</script>

<style scoped>
.proxy-config {
  max-width: 800px;
}

.config-form {
  max-width: 100%;
}

.form-help {
  margin-left: 8px;
  font-size: 12px;
  color: #6b7280;
}

.form-actions {
  margin-top: 32px;
  padding-top: 24px;
  border-top: 1px solid #f0f0f0;
}

.loading-placeholder {
  text-align: center;
  padding: 60px 0;
}

:deep(.el-divider__text) {
  background-color: #ffffff;
  padding: 0 20px;
}

:deep(.el-divider__text h3) {
  margin: 0;
  font-size: 16px;
  color: #1f2937;
  font-weight: 600;
}

.el-form-item {
  margin-bottom: 20px;
}
</style>