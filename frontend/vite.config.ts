import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig(({ mode }) => {
  // 加载环境变量
  const env = loadEnv(mode, '', '')
  
  // 构建API基础URL
  const getApiBaseUrl = () => {
    // 优先使用完整URL配置
    if (env.VITE_API_BASE_URL) {
      return env.VITE_API_BASE_URL
    }
    
    // 否则从协议、主机、端口组合
    const protocol = env.VITE_API_PROTOCOL || 'http'
    const host = env.VITE_API_HOST || '127.0.0.1'
    const port = env.VITE_API_PORT || '9090'
    
    return `${protocol}://${host}:${port}`
  }
  
  const apiBaseUrl = getApiBaseUrl()
  
  return {
    plugins: [vue()],
    resolve: {
      alias: {
        '@': resolve(process.cwd(), 'src'),
      },
    },
    server: {
      port: 3000,
      host: '0.0.0.0', // 允许外部访问
      proxy: {
        '/api': {
          target: apiBaseUrl,
          changeOrigin: true,
          secure: apiBaseUrl.indexOf('https') === 0, // 自动处理HTTPS
          rewrite: (path: string) => path.replace(/^\/api/, '/api'), // 保持/api前缀
        },
        '/auth': {
          target: apiBaseUrl,
          changeOrigin: true,
          secure: apiBaseUrl.indexOf('https') === 0,
        },
        '/metrics': {
          target: apiBaseUrl,
          changeOrigin: true,
          secure: apiBaseUrl.indexOf('https') === 0,
        },
        '/health': {
          target: apiBaseUrl,
          changeOrigin: true,
          secure: apiBaseUrl.indexOf('https') === 0,
        },
        '/performance': {
          target: apiBaseUrl,
          changeOrigin: true,
          secure: apiBaseUrl.indexOf('https') === 0,
        },
        '/errors': {
          target: apiBaseUrl,
          changeOrigin: true,
          secure: apiBaseUrl.indexOf('https') === 0,
        },
      },
    },
    build: {
      outDir: 'dist',
      sourcemap: true,
    },
    define: {
      // 将环境变量注入到前端代码中
      __API_BASE_URL__: JSON.stringify(apiBaseUrl),
    },
  }
})