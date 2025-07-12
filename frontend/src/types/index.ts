// API 配置类型定义
export interface ApiKey {
  id: string
  key: string
  weight: number
  max_requests_per_minute: number
}

export interface ServerConfig {
  host: string
  port: number
  workers: number
  max_connections: number
  tls: TlsConfig
}

export interface TlsConfig {
  enabled: boolean
  cert_path: string
  key_path: string
  acme?: AcmeConfig
}

export interface AcmeConfig {
  enabled: boolean
  domains: string[]
  email: string
  directory_url: string
}

export interface GeminiConfig {
  api_keys: ApiKey[]
  base_url: string
  timeout_seconds: number
}

export interface AuthConfig {
  enabled: boolean
  jwt_secret: string
  rate_limit_per_minute: number
}

export interface MetricsConfig {
  enabled: boolean
  prometheus_port: number
}

export interface ProxyConfig {
  server: ServerConfig
  gemini: GeminiConfig
  auth: AuthConfig
  metrics: MetricsConfig
}

// 健康检查相关类型
export interface CheckResult {
  status: string
  message: string
  duration_ms: number
}

export interface HealthStatus {
  status: string
  timestamp: number
  checks: Record<string, CheckResult>
}

// 监控指标类型
export interface MetricValue {
  value: number
  timestamp: number
}

export interface MetricData {
  name: string
  help: string
  type: string
  values: MetricValue[]
}

// UI 相关类型
export interface MenuItem {
  path: string
  name: string
  title: string
  icon: string
}

export interface TableColumn {
  prop: string
  label: string
  width?: string | number
  align?: 'left' | 'center' | 'right'
  sortable?: boolean
}

// API 响应类型
export interface ApiResponse<T = any> {
  success: boolean
  data?: T
  message?: string
  error?: string
}