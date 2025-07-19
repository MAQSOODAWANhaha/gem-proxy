/**
 * StatCard 内容格式化标准工具
 * 基于 Element Plus 最佳实践，确保所有 StatCard 显示内容的一致性
 */

/**
 * 标准数字格式化器
 * @param value 数字值
 * @returns 格式化后的字符串（无千位分隔符）
 */
export const formatNumber = (value: number | string): string => {
  if (typeof value === 'string') {
    return value
  }
  return String(value)
}

/**
 * 百分比格式化器
 * @param value 数字值（0-100）
 * @returns 带%符号的字符串
 */
export const formatPercentage = (value: number): string => {
  return `${Math.round(value)}%`
}

/**
 * 状态文字格式化器
 * 确保状态文字与数字具有相同的视觉权重
 * @param status 状态文字
 * @returns 标准化的状态文字
 */
export const formatStatus = (status: string): string => {
  // 确保状态文字长度一致（2个字符）
  const statusMap: Record<string, string> = {
    '需优化': '待优化',
    'unknown': '未知',
    'healthy': '健康',
    'degraded': '降级',
    'unhealthy': '异常'
  }
  
  return statusMap[status] || status
}

/**
 * 权重状态格式化器
 * @param effectiveness 负载均衡有效性（0-100）
 * @returns 2字符的状态文字
 */
export const formatWeightHealth = (effectiveness: number): string => {
  if (effectiveness >= 90) return '优秀'
  if (effectiveness >= 70) return '良好'
  if (effectiveness >= 50) return '一般'
  return '待优化'
}

/**
 * 时间格式化器
 * @param timestamp 时间戳或日期字符串
 * @returns 格式化的时间字符串
 */
export const formatTime = (timestamp: number | string): string => {
  if (!timestamp) return '未知'
  
  const date = typeof timestamp === 'number' 
    ? new Date(timestamp * 1000) 
    : new Date(timestamp)
    
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

/**
 * 评分格式化器
 * @param score 评分（0-100）
 * @returns 带单位的评分字符串
 */
export const formatScore = (score: number): string => {
  return `${Math.round(score)}/100`
}

/**
 * 通用 StatCard 值格式化器
 * 根据值类型自动选择合适的格式化方式
 * @param value 值
 * @param type 值类型（可选）
 * @returns 格式化后的字符串
 */
export const formatStatValue = (
  value: number | string, 
  type?: 'number' | 'percentage' | 'status' | 'time' | 'score'
): string => {
  switch (type) {
    case 'percentage':
      return formatPercentage(Number(value))
    case 'status':
      return formatStatus(String(value))
    case 'time':
      return formatTime(value)
    case 'score':
      return formatScore(Number(value))
    case 'number':
    default:
      return formatNumber(value)
  }
}

/**
 * StatCard 标准颜色配置
 */
export const STAT_COLORS = {
  primary: '#1890ff',
  success: '#52c41a',
  warning: '#faad14',
  danger: '#f5222d',
  info: '#722ed1',
  
  // 特殊状态颜色
  excellent: '#52c41a',  // 优秀
  good: '#faad14',      // 良好
  average: '#fa8c16',   // 一般
  poor: '#f5222d'       // 待优化
} as const

/**
 * 根据权重健康度获取颜色
 * @param effectiveness 有效性百分比
 * @returns 对应的颜色值
 */
export const getHealthColor = (effectiveness: number): string => {
  if (effectiveness >= 90) return STAT_COLORS.excellent
  if (effectiveness >= 70) return STAT_COLORS.good
  if (effectiveness >= 50) return STAT_COLORS.average
  return STAT_COLORS.poor
}