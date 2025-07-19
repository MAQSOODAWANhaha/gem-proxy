<template>
  <el-col :span="span">
    <el-card v-loading="loading" :shadow="shadow">
      <!-- 卡片头部 -->
      <template v-if="title || $slots.header || $slots.actions" #header>
        <div class="card-header">
          <span v-if="title" class="card-title">{{ title }}</span>
          <slot v-else name="title" />
          <div v-if="$slots.actions" class="card-actions">
            <slot name="actions" />
          </div>
        </div>
        <slot name="header" />
      </template>
      
      <!-- 卡片内容 -->
      <div v-if="!loading" class="card-content">
        <slot />
      </div>
    </el-card>
  </el-col>
</template>

<script setup lang="ts">
interface Props {
  /** 卡片标题 */
  title?: string
  /** 加载状态 */
  loading?: boolean
  /** 阴影效果 */
  shadow?: 'always' | 'hover' | 'never'
  /** 列宽度 (el-col span) */
  span?: number
}

withDefaults(defineProps<Props>(), {
  shadow: 'always',
  span: 24,
  loading: false
})
</script>

<style scoped>
/* 卡片样式已在全局 layout.css 中定义 */
.card-content {
  /* 移除额外间距，使用全局标准 */
  margin: 0;
}
</style>