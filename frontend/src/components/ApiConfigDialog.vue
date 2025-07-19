<template>
  <el-dialog
    v-model="visible"
    title="API 服务器配置"
    width="500px"
    :close-on-click-modal="false"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="120px">
      <el-form-item label="API 地址" prop="baseUrl">
        <el-input
          v-model="form.baseUrl"
          placeholder="例如: https://api.example.com:9090"
          clearable
        >
          <template #prepend>
            <el-select v-model="form.protocol" style="width: 80px">
              <el-option label="HTTP" value="http" />
              <el-option label="HTTPS" value="https" />
            </el-select>
          </template>
        </el-input>
        <div class="form-help">
          完整的API服务器地址，包含协议、域名/IP和端口
        </div>
      </el-form-item>

      <el-form-item label="预设配置">
        <el-select 
          v-model="selectedPreset" 
          placeholder="选择预设配置" 
          @change="applyPreset"
          clearable
        >
          <el-option
            v-for="preset in presets"
            :key="preset.name"
            :label="preset.name"
            :value="preset.name"
          />
        </el-select>
      </el-form-item>

      <el-form-item>
        <el-alert
          title="注意"
          type="info"
          :closable="false"
          show-icon
        >
          <template #default>
            <p>• 修改配置后页面将自动刷新</p>
            <p>• 确保API服务器地址正确且可访问</p>
            <p>• HTTPS需要有效的SSL证书</p>
          </template>
        </el-alert>
      </el-form-item>
    </el-form>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="visible = false">取消</el-button>
        <el-button @click="testConnection" :loading="testing">测试连接</el-button>
        <el-button type="primary" @click="saveConfig" :loading="saving">
          保存配置
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { setApiBaseUrl, getApiBaseUrl } from '../config'
import axios from 'axios'

// Props & Emits
interface Props {
  modelValue: boolean
}

interface Emits {
  (e: 'update:modelValue', value: boolean): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// 响应式数据
const formRef = ref<FormInstance>()
const testing = ref(false)
const saving = ref(false)
const selectedPreset = ref('')

const visible = ref(false)
const form = reactive({
  protocol: 'http',
  baseUrl: ''
})

// 预设配置
const presets = [
  { name: '本地开发', url: 'http://127.0.0.1:9090' },
  { name: '本地开发 (HTTPS)', url: 'https://127.0.0.1:9090' },
  { name: '局域网', url: 'http://192.168.1.100:9090' },
  { name: '生产环境', url: 'https://api.example.com' },
]

// 表单验证规则
const rules: FormRules = {
  baseUrl: [
    { required: true, message: '请输入API地址', trigger: 'blur' },
    { 
      pattern: /^https?:\/\/[\w\.-]+(:\d+)?$/,
      message: '请输入有效的URL格式',
      trigger: 'blur'
    }
  ]
}

// 监听显示状态
watch(() => props.modelValue, (newVal) => {
  visible.value = newVal
  if (newVal) {
    // 初始化表单
    const currentUrl = getApiBaseUrl()
    if (currentUrl) {
      form.baseUrl = currentUrl
      form.protocol = currentUrl.startsWith('https') ? 'https' : 'http'
    }
  }
})

watch(visible, (newVal) => {
  emit('update:modelValue', newVal)
})

// 应用预设配置
function applyPreset() {
  const preset = presets.find(p => p.name === selectedPreset.value)
  if (preset) {
    form.baseUrl = preset.url
    form.protocol = preset.url.startsWith('https') ? 'https' : 'http'
  }
}

// 测试连接
async function testConnection() {
  if (!form.baseUrl) {
    ElMessage.warning('请先输入API地址')
    return
  }

  testing.value = true
  try {
    // 测试健康检查端点
    const response = await axios.get(`${form.baseUrl}/health`, {
      timeout: 5000
    })
    
    if (response.status === 200) {
      ElMessage.success('连接测试成功！API服务器正常')
    } else {
      ElMessage.warning('服务器响应异常，请检查配置')
    }
  } catch (error: any) {
    console.error('连接测试失败:', error)
    if (error.code === 'ECONNREFUSED') {
      ElMessage.error('连接被拒绝，请检查服务器地址和端口')
    } else if (error.code === 'ETIMEDOUT') {
      ElMessage.error('连接超时，请检查网络和服务器状态')
    } else {
      ElMessage.error(`连接失败: ${error.message}`)
    }
  } finally {
    testing.value = false
  }
}

// 保存配置
async function saveConfig() {
  if (!formRef.value) return

  try {
    await formRef.value.validate()
    
    await ElMessageBox.confirm(
      '保存配置后页面将自动刷新，确定继续吗？',
      '确认保存',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )

    saving.value = true
    
    // 保存配置并刷新页面
    setApiBaseUrl(form.baseUrl)
    
  } catch (error) {
    if (error !== 'cancel') {
      console.error('保存配置失败:', error)
      ElMessage.error('保存配置失败')
    }
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.form-help {
  font-size: 12px;
  color: #6b7280;
  margin-top: 4px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

:deep(.el-alert__content) {
  font-size: 13px;
  line-height: 1.5;
}

:deep(.el-alert__content p) {
  margin: 2px 0;
}
</style>