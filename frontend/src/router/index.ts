import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import Layout from '../components/Layout.vue'
import { useAuthStore } from '../stores/auth'

const routes: Array<RouteRecordRaw> = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('../views/Login.vue'),
    meta: {
      title: '登录',
      requiresAuth: false
    }
  },
  {
    path: '/',
    component: Layout,
    redirect: '/dashboard',
    meta: {
      requiresAuth: true
    },
    children: [
      {
        path: '/dashboard',
        name: 'Dashboard',
        component: () => import('../views/Dashboard.vue'),
        meta: {
          title: '控制台',
          icon: 'Monitor',
          requiresAuth: true
        }
      },
      {
        path: '/config',
        name: 'ProxyConfig',
        component: () => import('../views/ProxyConfig.vue'),
        meta: {
          title: '代理配置',
          icon: 'Setting',
          requiresAuth: true
        }
      },
      {
        path: '/api-keys',
        name: 'ApiKeys',
        component: () => import('../views/ApiKeys.vue'),
        meta: {
          title: 'API 密钥',
          icon: 'Key',
          requiresAuth: true
        }
      },
      {
        path: '/load-balancing',
        name: 'LoadBalancing',
        component: () => import('../views/LoadBalancing.vue'),
        meta: {
          title: '负载均衡',
          icon: 'Scale',
          requiresAuth: true
        }
      },
      {
        path: '/tls-config',
        name: 'TlsConfig',
        component: () => import('../views/TlsConfig.vue'),
        meta: {
          title: 'TLS 配置',
          icon: 'Lock',
          requiresAuth: true
        }
      },
      {
        path: '/monitoring',
        name: 'Monitoring',
        component: () => import('../views/Monitoring.vue'),
        meta: {
          title: '监控指标',
          icon: 'DataAnalysis',
          requiresAuth: true
        }
      }
    ]
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/dashboard'
  }
]

const router = createRouter({
  history: createWebHistory('/'),
  routes
})

// 路由守卫
router.beforeEach(async (to, _from, next) => {
  const authStore = useAuthStore()
  
  // 检查路由是否需要认证
  const requiresAuth = to.matched.some(record => record.meta.requiresAuth !== false)
  
  if (requiresAuth) {
    // 需要认证的路由
    if (!authStore.token) {
      // 没有token，跳转到登录页
      next('/login')
      return
    }
    
    // 验证token有效性
    const isValid = await authStore.verifyToken()
    if (!isValid) {
      // token无效，跳转到登录页
      next('/login')
      return
    }
  } else {
    // 不需要认证的路由（如登录页）
    if (to.path === '/login' && authStore.isLoggedIn) {
      // 已经登录，跳转到首页
      next('/')
      return
    }
  }
  
  next()
})

export default router