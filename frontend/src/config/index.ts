// 配置管理模块
interface ApiConfig {
  baseURL: string;
  timeout: number;
}

// 从环境变量获取API配置
function getApiConfig(): ApiConfig {
  // 开发环境：使用代理，直接使用相对路径
  if (import.meta.env.DEV) {
    return {
      baseURL: '', // 开发环境使用代理
      timeout: 30000,
    };
  }

  // 生产环境：根据构建时注入的配置或运行时检测
  const buildTimeApiUrl = (globalThis as any).__API_BASE_URL__;
  
  // 运行时检测API地址（支持生产环境动态配置）
  const runtimeApiUrl = getRuntimeApiUrl();
  
  return {
    baseURL: runtimeApiUrl || buildTimeApiUrl || window.location.origin,
    timeout: 30000,
  };
}

// 运行时获取API地址
function getRuntimeApiUrl(): string | null {
  // 1. 从window全局变量获取（可以在index.html中注入）
  if ((window as any).API_BASE_URL) {
    return (window as any).API_BASE_URL;
  }

  // 2. 从meta标签获取
  const metaApiUrl = document.querySelector('meta[name="api-base-url"]');
  if (metaApiUrl) {
    return metaApiUrl.getAttribute('content');
  }

  // 3. 从localStorage获取（用户可以手动设置）
  const storedApiUrl = localStorage.getItem('API_BASE_URL');
  if (storedApiUrl) {
    return storedApiUrl;
  }

  return null;
}

// 设置API地址（用于运行时动态配置）
export function setApiBaseUrl(url: string): void {
  localStorage.setItem('API_BASE_URL', url);
  // 重新加载页面以应用新配置
  window.location.reload();
}

// 获取当前API配置
export function getApiBaseUrl(): string {
  return getApiConfig().baseURL;
}

// 导出配置
export const apiConfig = getApiConfig();

// 开发环境日志
if (import.meta.env.DEV) {
  console.log('🔧 API配置信息:', {
    环境: import.meta.env.MODE,
    baseURL: apiConfig.baseURL || '使用代理',
    构建时配置: (globalThis as any).__API_BASE_URL__,
    运行时配置: getRuntimeApiUrl(),
  });
}

// 类型声明
declare global {
  interface Window {
    API_BASE_URL?: string;
  }
}