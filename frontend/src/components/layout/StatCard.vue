<template>
  <el-col :span="span">
    <el-card shadow="hover" class="stat-card">
      <el-statistic 
        :title="title"
        :value="value"
        :precision="precision"
        :formatter="formatter"
        :value-style="valueStyle"
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
import type { Component } from 'vue'

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

withDefaults(defineProps<Props>(), {
  precision: 0,
  span: 6,
  iconColor: 'var(--color-primary)'
})
</script>

<style scoped>
/* 统计卡片样式已在全局 layout.css 中定义 */
</style>