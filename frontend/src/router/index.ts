import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import Layout from '../components/Layout.vue'

const routes: Array<RouteRecordRaw> = [
  {
    path: '/',
    component: Layout,
    redirect: '/dashboard',
    children: [
      {
        path: '/dashboard',
        name: 'Dashboard',
        component: () => import('../views/Dashboard.vue'),
        meta: {
          title: '控制台',
          icon: 'Monitor'
        }
      },
      {
        path: '/config',
        name: 'ProxyConfig',
        component: () => import('../views/ProxyConfig.vue'),
        meta: {
          title: '代理配置',
          icon: 'Setting'
        }
      },
      {
        path: '/api-keys',
        name: 'ApiKeys',
        component: () => import('../views/ApiKeys.vue'),
        meta: {
          title: 'API 密钥',
          icon: 'Key'
        }
      },
      {
        path: '/tls-config',
        name: 'TlsConfig',
        component: () => import('../views/TlsConfig.vue'),
        meta: {
          title: 'TLS 配置',
          icon: 'Lock'
        }
      },
      {
        path: '/monitoring',
        name: 'Monitoring',
        component: () => import('../views/Monitoring.vue'),
        meta: {
          title: '监控指标',
          icon: 'DataAnalysis'
        }
      }
    ]
  }
]

const router = createRouter({
  history: createWebHistory('/'),
  routes
})

export default router