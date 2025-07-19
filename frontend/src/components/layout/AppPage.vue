<template>
  <div class="app-page">
    <!-- 页面头部 -->
    <div v-if="title || $slots.header" class="page-header">
      <div class="header-content">
        <div class="header-main">
          <h1 v-if="title" class="page-title">{{ title }}</h1>
          <p v-if="description" class="page-description">{{ description }}</p>
        </div>
        <div v-if="$slots.actions" class="header-actions">
          <slot name="actions" />
        </div>
      </div>
      <slot name="header" />
    </div>
    
    <!-- 页面内容 -->
    <div class="page-content">
      <el-row :gutter="32">
        <slot />
      </el-row>
    </div>
  </div>
</template>

<script setup lang="ts">
interface Props {
  /** 页面标题 */
  title?: string
  /** 页面描述 */
  description?: string
}

defineProps<Props>()
</script>

<style scoped>
/* 所有样式已在全局 layout.css 中定义 */
/* 这里不需要额外的样式，遵循设计标准 */
</style>