<template>
  <AppPage title="TLS 配置" description="配置 HTTPS 证书和 ACME 自动证书管理">
    <template #actions>
      <el-button 
        type="primary" 
        @click="saveConfig"
        :loading="configStore.loading"
      >
        保存配置
      </el-button>
      <el-button @click="resetForm">重置</el-button>
    </template>
    
    <ContentCard title="TLS 配置管理" :span="24" :loading="configStore.loading">
      <el-form 
        v-if="localConfig"
        ref="formRef"
        :model="localConfig.server.tls" 
        label-width="140px"
        class="tls-form"
      >
        <!-- TLS 基本设置 -->
        <el-divider content-position="left">
          <h3>TLS 基本设置</h3>
        </el-divider>
        
        <el-row :gutter="32">
          <el-col :span="8">
            <el-form-item label="启用 TLS">
              <el-switch 
                v-model="localConfig.server.tls.enabled"
                active-text="开启"
                inactive-text="关闭"
              />
            </el-form-item>
          </el-col>
          <el-col v-if="localConfig.server.tls.enabled" :span="8">
            <el-form-item label="证书文件路径">
              <el-input 
                v-model="localConfig.server.tls.cert_path" 
                placeholder="/path/to/cert.pem"
              />
            </el-form-item>
          </el-col>
          <el-col v-if="localConfig.server.tls.enabled" :span="8">
            <el-form-item label="私钥文件路径">
              <el-input 
                v-model="localConfig.server.tls.key_path" 
                placeholder="/path/to/key.pem"
              />
            </el-form-item>
          </el-col>
        </el-row>

        <!-- ACME 配置 -->
        <el-divider content-position="left">
          <h3>ACME 自动证书</h3>
        </el-divider>
        
        <el-row :gutter="32">
          <el-col :span="8">
            <el-form-item label="启用 ACME">
              <el-switch 
                v-model="acmeEnabled"
                active-text="开启"
                inactive-text="关闭"
              />
            </el-form-item>
          </el-col>
          <el-col v-if="acmeEnabled && localConfig.server.tls.acme" :span="8">
            <el-form-item label="联系邮箱">
              <el-input 
                v-model="localConfig.server.tls.acme.email" 
                placeholder="admin@example.com"
              />
            </el-form-item>
          </el-col>
          <el-col v-if="acmeEnabled && localConfig.server.tls.acme" :span="8">
            <el-form-item label="ACME 服务">
              <el-select 
                v-model="localConfig.server.tls.acme.directory_url"
                style="width: 100%"
              >
                <el-option
                  label="Production"
                  value="https://acme-v02.api.letsencrypt.org/directory"
                />
                <el-option
                  label="Staging"
                  value="https://acme-staging-v02.api.letsencrypt.org/directory"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        
        <!-- 域名管理区域 -->
        <el-row v-if="acmeEnabled && localConfig.server.tls.acme" :gutter="32">
          <el-col :span="24">
            <el-form-item label="域名列表">
              <div class="domain-management">
                <el-tag
                  v-for="domain in localConfig.server.tls.acme.domains"
                  :key="domain"
                  closable
                  @close="removeDomain(domain)"
                  class="domain-tag"
                >
                  {{ domain }}
                </el-tag>
                <el-input
                  v-if="inputVisible"
                  ref="inputRef"
                  v-model="inputValue"
                  size="small"
                  class="domain-input"
                  @keyup.enter="handleInputConfirm"
                  @blur="handleInputConfirm"
                />
                <el-button v-else size="small" @click="showInput">
                  + 添加域名
                </el-button>
              </div>
            </el-form-item>
          </el-col>
        </el-row>

      </el-form>
      
      <div v-else class="loading-placeholder">
        <el-empty description="配置加载中..." />
      </div>
    </ContentCard>
  </AppPage>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { ElMessage, type FormInstance } from 'element-plus'
import { useConfigStore } from '../stores/config'
import type { ProxyConfig } from '../types'
import AppPage from '../components/layout/AppPage.vue'
import ContentCard from '../components/layout/ContentCard.vue'

const configStore = useConfigStore()
const formRef = ref<FormInstance>()
const inputRef = ref()

// 本地配置副本
const localConfig = ref<ProxyConfig | null>(null)

// ACME 域名输入
const inputVisible = ref(false)
const inputValue = ref('')

// 计算属性
const acmeEnabled = computed({
  get: () => localConfig.value?.server.tls.acme?.enabled || false,
  set: (value: boolean) => {
    if (localConfig.value) {
      if (!localConfig.value.server.tls.acme) {
        localConfig.value.server.tls.acme = {
          enabled: value,
          domains: [],
          email: '',
          directory_url: 'https://acme-v02.api.letsencrypt.org/directory'
        }
      } else {
        localConfig.value.server.tls.acme.enabled = value
      }
    }
  }
})

// 监听配置变化
watch(() => configStore.config, (newConfig) => {
  if (newConfig) {
    localConfig.value = JSON.parse(JSON.stringify(newConfig))
  }
}, { immediate: true, deep: true })

// 方法
function removeDomain(domain: string) {
  if (localConfig.value?.server.tls.acme) {
    const index = localConfig.value.server.tls.acme.domains.indexOf(domain)
    if (index > -1) {
      localConfig.value.server.tls.acme.domains.splice(index, 1)
    }
  }
}

function showInput() {
  inputVisible.value = true
  nextTick(() => {
    inputRef.value?.focus()
  })
}

function handleInputConfirm() {
  if (inputValue.value && localConfig.value?.server.tls.acme) {
    if (!localConfig.value.server.tls.acme.domains.includes(inputValue.value)) {
      localConfig.value.server.tls.acme.domains.push(inputValue.value)
    }
  }
  inputVisible.value = false
  inputValue.value = ''
}

async function saveConfig() {
  if (!localConfig.value) return
  
  try {
    await configStore.updateConfig(localConfig.value)
    ElMessage.success('TLS 配置保存成功')
  } catch (error) {
    console.error('保存配置失败:', error)
    ElMessage.error('保存配置失败')
  }
}

function resetForm() {
  if (configStore.config) {
    localConfig.value = JSON.parse(JSON.stringify(configStore.config))
    ElMessage.info('已重置为原始配置')
  }
}
</script>

<style scoped>
/* 使用全局样式和工具类，大幅简化自定义样式 */

.domain-management {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-small);
}

.domain-tag {
  margin: 0;
}

.domain-input {
  width: 120px;
}


.loading-placeholder {
  text-align: center;
  padding: var(--spacing-section) 0;
}

:deep(.el-divider__text) {
  background-color: var(--bg-color);
  padding: 0 var(--spacing-large);
}

:deep(.el-divider__text h3) {
  margin: 0;
  font-size: var(--font-size-medium);
  color: var(--text-color-primary);
  font-weight: var(--font-weight-semibold);
}
</style>