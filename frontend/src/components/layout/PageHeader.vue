<template>
  <div class="page-header" :class="headerClasses">
    <div class="header-content">
      <!-- 标题区域 -->
      <div class="header-title-section">
        <h1 v-if="title" class="page-title">{{ title }}</h1>
        <slot v-else name="title" />
        
        <p v-if="description" class="page-description">{{ description }}</p>
        <slot v-else name="description" />
      </div>
      
      <!-- 操作区域 -->
      <div v-if="$slots.actions || showRefresh" class="header-actions">
        <slot name="actions" />
        
        <el-button 
          v-if="showRefresh"
          type="primary"
          :loading="refreshLoading"
          @click="handleRefresh"
          :size="actionSize"
        >
          <el-icon><Refresh /></el-icon>
          刷新
        </el-button>
      </div>
    </div>
    
    <!-- 扩展内容区域 -->
    <div v-if="$slots.extra" class="header-extra">
      <slot name="extra" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Refresh } from '@element-plus/icons-vue'

interface Props {
  /** 页面标题 */
  title?: string
  /** 页面描述 */
  description?: string
  /** 是否显示刷新按钮 */
  showRefresh?: boolean
  /** 刷新按钮加载状态 */
  refreshLoading?: boolean
  /** 操作按钮尺寸 */
  actionSize?: 'large' | 'default' | 'small'
  /** 标题对齐方式 */
  align?: 'left' | 'center' | 'right'
  /** 是否显示分割线 */
  divider?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  actionSize: 'default',
  align: 'left',
  divider: true
})

interface Emits {
  (e: 'refresh'): void
}

const emit = defineEmits<Emits>()

const headerClasses = computed(() => ({
  'page-header--center': props.align === 'center',
  'page-header--right': props.align === 'right',
  'page-header--no-divider': !props.divider
}))

function handleRefresh() {
  emit('refresh')
}
</script>

<style scoped>
.page-header {
  width: 100%;
  padding-block-end: var(--layout-item-spacing);
  border-block-end: 1px solid var(--color-border-light);
}

.page-header--no-divider {
  border-block-end: none;
  padding-block-end: 0;
}

.header-content {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--layout-item-spacing);
}

.page-header--center .header-content {
  justify-content: center;
  text-align: center;
}

.page-header--right .header-content {
  justify-content: flex-end;
  text-align: right;
}

.header-title-section {
  flex: 1;
  min-width: 0; /* 防止文本溢出 */
}

.page-title {
  font-size: var(--font-size-xlarge);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
  line-height: var(--line-height-tight);
  margin: 0;
  margin-block-end: var(--layout-small-spacing);
  
  /* 自适应文本换行 */
  text-wrap: balance;
  overflow-wrap: break-word;
}

.page-description {
  font-size: var(--font-size-normal);
  color: var(--color-text-secondary);
  line-height: var(--line-height-normal);
  margin: 0;
  
  /* 优化长文本显示 */
  text-wrap: pretty;
  overflow-wrap: break-word;
}

.header-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--layout-small-spacing);
  align-items: center;
  flex-shrink: 0;
}

.header-extra {
  width: 100%;
  margin-block-start: var(--layout-item-spacing);
}

/* 响应式调整 */
@container (max-width: 48rem) {
  .header-content {
    flex-direction: column;
    align-items: stretch;
  }
  
  .header-actions {
    justify-content: flex-start;
  }
  
  .page-header--center .header-content {
    align-items: center;
  }
  
  .page-header--center .header-actions {
    justify-content: center;
  }
  
  .page-header--right .header-content {
    align-items: flex-end;
  }
  
  .page-header--right .header-actions {
    justify-content: flex-end;
  }
}

/* 小屏幕操作按钮全宽 */
@container (max-width: 30rem) {
  .header-actions {
    width: 100%;
  }
  
  .header-actions :deep(.el-button) {
    flex: 1;
  }
}
</style>