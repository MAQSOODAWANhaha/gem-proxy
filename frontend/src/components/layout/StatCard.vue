<template>
  <el-col :span="span">
    <el-card 
      shadow="hover" 
      class="stat-card"
      :body-style="cardBodyStyle"
    >
      <el-statistic 
        :title="title"
        :value="value"
        :precision="precision"
        :formatter="computedFormatter"
        :value-style="computedValueStyle"
      >
        <template v-if="icon || $slots.prefix" #prefix>
          <slot name="prefix">
            <el-icon v-if="icon" :style="{ color: iconColor || 'var(--color-primary)' }">
              <component :is="icon" />
            </el-icon>
          </slot>
        </template>
        <template v-if="$slots.suffix" #suffix>
          <slot name="suffix" />
        </template>
      </el-statistic>
    </el-card>
  </el-col>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Component } from 'vue'
import { formatStatValue } from '../../utils/statCardFormatter'

interface Props {
  /** 统计项标题 */
  title: string
  /** 统计数值 */
  value: string | number
  /** 数值精度 */
  precision?: number
  /** 图标组件 */
  icon?: Component
  /** 图标颜色 */
  iconColor?: string
  /** 列宽度 (el-col span) */
  span?: number
  /** 数值格式化函数 */
  formatter?: (value: number) => string
  /** 数值样式 */
  valueStyle?: Record<string, string>
}

const props = withDefaults(defineProps<Props>(), {
  precision: 0,
  span: 6,
  iconColor: 'var(--color-primary)'
})

// Element Plus 推荐的 body-style 属性 - 直接控制Card高度
const cardBodyStyle = computed(() => ({
  height: '100px',
  minHeight: '100px',
  maxHeight: '100px',
  padding: '20px',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  boxSizing: 'border-box',
  overflow: 'hidden'
}))

// 标准化的格式化函数 - 使用统一的格式化工具
const standardFormatter = (value: number | string): string => {
  return formatStatValue(value)
}

// 使用标准化formatter或外部提供的formatter
const computedFormatter = computed(() => {
  return props.formatter || standardFormatter
})

// 标准化的数值样式 - 确保所有StatCard显示一致
const computedValueStyle = computed(() => ({
  color: '#1890ff',
  fontSize: '24px',
  fontWeight: '600',
  lineHeight: '1.2',
  textAlign: 'center',
  whiteSpace: 'nowrap',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  ...props.valueStyle // 允许外部覆盖
}))
</script>

<style scoped>
/* 统计卡片样式已在全局 layout.css 中定义 */
</style>