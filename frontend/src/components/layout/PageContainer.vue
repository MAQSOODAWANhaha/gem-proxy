<template>
  <div class="page-container" :class="containerClasses">
    <!-- 页面头部插槽 -->
    <header v-if="$slots.header" class="page-header-slot">
      <slot name="header" />
    </header>
    
    <!-- 主要内容区域 -->
    <main class="page-content" :class="contentClasses">
      <slot />
    </main>
    
    <!-- 页面底部插槽 -->
    <footer v-if="$slots.footer" class="page-footer-slot">
      <slot name="footer" />
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  /** 内容布局模式 */
  layout?: 'default' | 'wide' | 'narrow' | 'fluid'
  /** 是否全高度 */
  fullHeight?: boolean
  /** 自定义CSS类 */
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  layout: 'default',
  fullHeight: true,
  class: ''
})

const containerClasses = computed(() => ({
  'page-container--wide': props.layout === 'wide',
  'page-container--narrow': props.layout === 'narrow', 
  'page-container--fluid': props.layout === 'fluid',
  'page-container--full-height': props.fullHeight,
  [props.class]: !!props.class
}))

const contentClasses = computed(() => ({
  'page-content--grid': props.layout !== 'fluid'
}))
</script>

<style scoped>
.page-container {
  width: 100%;
  min-height: 100vh;
  container-type: inline-size;
  padding: var(--layout-container-padding);
  background-color: var(--color-background);
  display: grid;
  grid-template-rows: auto 1fr auto;
  gap: var(--layout-content-spacing);
}

.page-container--full-height {
  height: 100vh;
}

.page-container--wide {
  --layout-container-padding: clamp(1rem, 2vw, 2rem);
}

.page-container--narrow {
  --layout-container-padding: clamp(2rem, 8vw, 6rem);
}

.page-container--fluid {
  --layout-container-padding: 0;
}

.page-header-slot,
.page-footer-slot {
  width: 100%;
}

.page-content {
  width: 100%;
  min-height: 0; /* 防止内容溢出 */
}

.page-content--grid {
  display: grid;
  gap: var(--layout-content-spacing);
  /* 自适应内容排列 */
  grid-template-columns: 1fr;
  align-content: start;
}

/* 响应式调整 */
@container (min-width: 768px) {
  .page-content--grid {
    grid-template-columns: repeat(auto-fit, minmax(min(30rem, 100%), 1fr));
  }
}

@container (min-width: 1200px) {
  .page-content--grid {
    grid-template-columns: repeat(auto-fit, minmax(min(25rem, 100%), 1fr));
  }
}
</style>