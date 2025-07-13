<template>
  <div class="login-container">
    <div class="login-card">
      <div class="login-header">
        <el-icon size="48" color="#409EFF">
          <Lock />
        </el-icon>
        <h2>Gemini 代理管理系统</h2>
        <p>请输入管理员密码</p>
      </div>

      <el-form
        ref="loginFormRef"
        :model="loginForm"
        :rules="loginRules"
        class="login-form"
        @submit.prevent="handleLogin"
      >
        <el-form-item prop="password">
          <el-input
            v-model="loginForm.password"
            type="password"
            placeholder="管理员密码"
            size="large"
            show-password
            @keyup.enter="handleLogin"
          >
            <template #prefix>
              <el-icon>
                <Key />
              </el-icon>
            </template>
          </el-input>
        </el-form-item>

        <el-form-item>
          <el-button
            type="primary"
            size="large"
            :loading="authStore.isLoading"
            @click="handleLogin"
            class="login-button"
          >
            <span v-if="!authStore.isLoading">登录</span>
            <span v-else>登录中...</span>
          </el-button>
        </el-form-item>
      </el-form>

      <div v-if="authStore.error" class="error-message">
        <el-alert
          :title="authStore.error"
          type="error"
          show-icon
          :closable="false"
        />
      </div>

      <div class="login-footer">
        <p>
          <el-icon size="14">
            <InfoFilled />
          </el-icon>
          请使用配置文件中设置的管理员密码登录
        </p>
      </div>
    </div>

    <div class="background-animation">
      <div class="floating-shapes">
        <div v-for="i in 6" :key="i" class="shape" :style="getShapeStyle(i)"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElForm } from 'element-plus'
import { Lock, Key, InfoFilled } from '@element-plus/icons-vue'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const loginFormRef = ref<InstanceType<typeof ElForm>>()

const loginForm = reactive({
  password: ''
})

const loginRules = {
  password: [
    { required: true, message: '请输入管理员密码', trigger: 'blur' },
    { min: 8, message: '密码长度至少8个字符', trigger: 'blur' }
  ]
}

const handleLogin = async () => {
  if (!loginFormRef.value) return
  
  try {
    const valid = await loginFormRef.value.validate()
    if (!valid) return

    authStore.clearError()
    
    const result = await authStore.login(loginForm.password)
    
    if (result.success) {
      ElMessage.success('登录成功')
      router.push('/')
    } else {
      ElMessage.error(result.message || '登录失败')
    }
  } catch (error) {
    console.error('Login error:', error)
  }
}

const getShapeStyle = (index: number) => {
  const colors = ['#409EFF', '#67C23A', '#E6A23C', '#F56C6C', '#909399', '#7B68EE']
  const size = 20 + (index * 10)
  const left = (index * 15) % 100
  const animationDelay = index * 0.5
  
  return {
    width: `${size}px`,
    height: `${size}px`,
    left: `${left}%`,
    backgroundColor: colors[index % colors.length],
    animationDelay: `${animationDelay}s`
  }
}

onMounted(() => {
  // 清除可能存在的错误状态
  authStore.clearError()
  
  // 如果已经登录，直接跳转到主页
  if (authStore.isLoggedIn) {
    router.push('/')
  }
})
</script>

<style scoped>
.login-container {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  position: relative;
  overflow: hidden;
}

.login-card {
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(10px);
  border-radius: 16px;
  padding: 48px;
  width: 100%;
  max-width: 420px;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
  position: relative;
  z-index: 10;
}

.login-header {
  text-align: center;
  margin-bottom: 32px;
}

.login-header h2 {
  margin: 16px 0 8px 0;
  color: #303133;
  font-size: 24px;
  font-weight: 600;
}

.login-header p {
  color: #606266;
  font-size: 14px;
  margin: 0;
}

.login-form {
  margin-bottom: 24px;
}

.login-button {
  width: 100%;
  height: 48px;
  font-size: 16px;
  font-weight: 500;
}

.error-message {
  margin-bottom: 24px;
}

.login-footer {
  text-align: center;
  color: #909399;
  font-size: 12px;
}

.login-footer p {
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.background-animation {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.floating-shapes {
  position: relative;
  width: 100%;
  height: 100%;
}

.shape {
  position: absolute;
  border-radius: 50%;
  opacity: 0.1;
  animation: float 6s ease-in-out infinite;
}

@keyframes float {
  0%, 100% {
    transform: translateY(0px) rotate(0deg);
  }
  50% {
    transform: translateY(-20px) rotate(180deg);
  }
}

.shape:nth-child(odd) {
  animation-direction: reverse;
}

.shape:nth-child(even) {
  animation-duration: 8s;
}

/* 响应式设计 */
@media (max-width: 480px) {
  .login-card {
    margin: 20px;
    padding: 32px 24px;
  }
  
  .login-header h2 {
    font-size: 20px;
  }
}

/* 深色模式支持 */
@media (prefers-color-scheme: dark) {
  .login-card {
    background: rgba(30, 30, 30, 0.95);
    color: #E4E7ED;
  }
  
  .login-header h2 {
    color: #E4E7ED;
  }
  
  .login-header p {
    color: #909399;
  }
}
</style>