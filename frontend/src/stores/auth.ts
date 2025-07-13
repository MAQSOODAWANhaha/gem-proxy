// src/stores/auth.ts
import { defineStore } from 'pinia'
import axios from 'axios'

export interface User {
  id: string
  role: string
  sessionId: string
}

export interface LoginRequest {
  password: string
}

export interface LoginResponse {
  success: boolean
  token?: string
  refresh_token?: string
  expires_in?: number
  message: string
}

export interface Claims {
  sub: string
  exp: number
  iat: number
  role: string
  session_id: string
}

export const useAuthStore = defineStore('auth', {
  state: () => ({
    user: null as User | null,
    token: localStorage.getItem('auth_token') || null,
    refreshToken: localStorage.getItem('refresh_token') || null,
    isAuthenticated: false,
    isLoading: false,
    error: null as string | null,
  }),

  getters: {
    isLoggedIn: (state) => state.isAuthenticated && !!state.token,
    currentUser: (state) => state.user,
    authToken: (state) => state.token,
  },

  actions: {
    async login(password: string) {
      this.isLoading = true
      this.error = null

      try {
        const response = await axios.post<LoginResponse>('/api/auth/login', {
          password
        })

        const data = response.data

        if (data.success && data.token) {
          this.token = data.token
          this.refreshToken = data.refresh_token || null
          this.isAuthenticated = true

          // 保存到本地存储
          localStorage.setItem('auth_token', data.token)
          if (data.refresh_token) {
            localStorage.setItem('refresh_token', data.refresh_token)
          }

          // 解析token获取用户信息
          await this.loadUserFromToken()
          
          // 设置axios默认Authorization头
          this.setupAxiosInterceptors()

          return { success: true, message: data.message }
        } else {
          this.error = data.message || '登录失败'
          return { success: false, message: this.error }
        }
      } catch (error: any) {
        this.error = error.response?.data?.message || '网络错误'
        return { success: false, message: this.error }
      } finally {
        this.isLoading = false
      }
    },

    async logout() {
      try {
        if (this.user?.sessionId) {
          await axios.post('/api/auth/logout', {
            session_id: this.user.sessionId
          })
        }
      } catch (error) {
        console.warn('Logout request failed:', error)
      }

      this.clearAuthData()
    },

    async refreshAccessToken() {
      if (!this.refreshToken) {
        this.clearAuthData()
        return false
      }

      try {
        const response = await axios.post<LoginResponse>('/api/auth/refresh', {
          refresh_token: this.refreshToken
        })

        const data = response.data

        if (data.success && data.token) {
          this.token = data.token
          localStorage.setItem('auth_token', data.token)
          
          // 更新axios默认Authorization头
          this.setupAxiosInterceptors()
          
          return true
        } else {
          this.clearAuthData()
          return false
        }
      } catch (error) {
        this.clearAuthData()
        return false
      }
    },

    async verifyToken() {
      if (!this.token) {
        return false
      }

      try {
        const response = await axios.post('/api/auth/verify', {
          token: this.token
        })

        const data = response.data

        if (data.valid) {
          this.isAuthenticated = true
          await this.loadUserFromToken()
          return true
        } else {
          // 尝试刷新token
          return await this.refreshAccessToken()
        }
      } catch (error) {
        // 尝试刷新token
        return await this.refreshAccessToken()
      }
    },

    async loadUserFromToken() {
      if (!this.token) return

      try {
        // 解析JWT token获取claims
        const base64Url = this.token.split('.')[1]
        const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/')
        const jsonPayload = decodeURIComponent(
          atob(base64)
            .split('')
            .map(c => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2))
            .join('')
        )

        const claims: Claims = JSON.parse(jsonPayload)
        
        this.user = {
          id: claims.sub,
          role: claims.role,
          sessionId: claims.session_id
        }
      } catch (error) {
        console.error('Failed to parse token:', error)
        this.clearAuthData()
      }
    },

    clearAuthData() {
      this.user = null
      this.token = null
      this.refreshToken = null
      this.isAuthenticated = false
      this.error = null

      // 清除本地存储
      localStorage.removeItem('auth_token')
      localStorage.removeItem('refresh_token')

      // 清除axios默认Authorization头
      delete axios.defaults.headers.common['Authorization']
    },

    setupAxiosInterceptors() {
      if (this.token) {
        axios.defaults.headers.common['Authorization'] = `Bearer ${this.token}`
      }

      // 添加响应拦截器处理401错误
      axios.interceptors.response.use(
        (response) => response,
        async (error) => {
          const originalRequest = error.config

          if (error.response?.status === 401 && !originalRequest._retry) {
            originalRequest._retry = true

            // 尝试刷新token
            const refreshed = await this.refreshAccessToken()
            
            if (refreshed) {
              // 重新发送原始请求
              originalRequest.headers['Authorization'] = `Bearer ${this.token}`
              return axios(originalRequest)
            } else {
              // 刷新失败，跳转到登录页
              this.clearAuthData()
              window.location.href = '/login'
            }
          }

          return Promise.reject(error)
        }
      )
    },

    async initializeAuth() {
      if (this.token) {
        this.setupAxiosInterceptors()
        const isValid = await this.verifyToken()
        
        if (!isValid) {
          this.clearAuthData()
        }
      }
    },

    clearError() {
      this.error = null
    }
  }
})