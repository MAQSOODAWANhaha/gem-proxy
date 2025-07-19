# 桌面端UI设计标准文档

## 📋 概述

本文档定义了基于 Element Plus 的桌面端专用UI设计标准，旨在创建统一、一致、高效的用户界面体验。

## 🎯 设计原则

### 1. Element Plus 原生优先
- **完全继承** Element Plus 的设计语言和组件行为
- **零自定义样式冲突**，只做组合和布局增强
- **保持原生组件特性**，确保升级兼容性

### 2. 桌面端专用优化
- **目标设备**: 桌面浏览器 (≥1024px)
- **不考虑移动端适配**，专注桌面体验
- **固定布局模式**，避免过度响应式设计

### 3. 全局统一标准
- **统一页面模板**：相同的结构、头部、内容布局
- **标准化组件使用**：卡片、表格、按钮的一致使用方式
- **可预测的视觉层次**：标题、间距、阴影的统一规格

## 🎨 设计令牌系统

### 字体系统
```css
/* 基于 Element Plus 标准字体 */
--font-size-extra-small: 12px;    /* 辅助信息 */
--font-size-small: 13px;          /* 表格内容 */
--font-size-base: 14px;           /* 正文内容 (Element Plus 默认) */
--font-size-medium: 16px;         /* 卡片标题 */
--font-size-large: 18px;          /* 次级标题 */
--font-size-extra-large: 20px;    /* 页面主标题 */

/* 字重系统 */
--font-weight-normal: 400;        /* 正文 */
--font-weight-medium: 500;        /* 强调 */
--font-weight-semibold: 600;      /* 标题 */
--font-weight-bold: 700;          /* 重要标题 */
```

### 间距系统
```css
/* 基于 Element Plus 的8px栅格系统 */
--spacing-mini: 4px;              /* 最小间距 */
--spacing-small: 8px;             /* 小间距 */
--spacing-base: 12px;             /* 基础间距 */
--spacing-medium: 16px;           /* 中等间距 */
--spacing-large: 20px;            /* 大间距 */
--spacing-extra-large: 24px;      /* 超大间距 */
--spacing-section: 32px;          /* 区块间距 */
```

### 布局系统
```css
/* 桌面端专用布局参数 */
--container-max-width: 1600px;    /* 页面最大宽度 */
--container-padding: 16px;        /* 容器内边距 */
--card-spacing: 24px;             /* 卡片间距 */
--border-radius-base: 6px;        /* 基础圆角 */
--border-radius-large: 8px;       /* 大圆角 */
```

### 色彩系统
```css
/* 使用 Element Plus 原生 CSS 变量 */
--color-primary: var(--el-color-primary);           /* #409EFF */
--color-success: var(--el-color-success);           /* #67C23A */
--color-warning: var(--el-color-warning);           /* #E6A23C */
--color-danger: var(--el-color-danger);             /* #F56C6C */
--color-info: var(--el-color-info);                 /* #909399 */

--text-color-primary: var(--el-text-color-primary); /* #303133 */
--text-color-regular: var(--el-text-color-regular); /* #606266 */
--text-color-secondary: var(--el-text-color-secondary); /* #909399 */

--border-color: var(--el-border-color);             /* #DCDFE6 */
--border-color-light: var(--el-border-color-light); /* #E4E7ED */
--bg-color: var(--el-bg-color);                     /* #FFFFFF */
--bg-color-page: var(--el-bg-color-page);           /* #F2F3F5 */
```

## 🏗️ 页面架构标准

### 页面结构模板
```vue
<template>
  <div class="app-page">
    <!-- 页面头部 -->
    <div class="page-header">
      <div class="header-content">
        <div class="header-main">
          <h1 class="page-title">页面标题</h1>
          <p class="page-description">页面描述（可选）</p>
        </div>
        <div class="header-actions">
          <!-- 操作按钮 -->
          <el-button type="primary">主要操作</el-button>
        </div>
      </div>
    </div>
    
    <!-- 页面内容 -->
    <div class="page-content">
      <el-row :gutter="24">
        <!-- 内容区域 -->
        <slot />
      </el-row>
    </div>
  </div>
</template>
```

### 页面样式规范
```css
.app-page {
  max-width: var(--container-max-width);
  margin: 0 auto;
  padding: var(--container-padding);
  min-height: 100vh;
  background-color: var(--bg-color-page);
}

.page-header {
  margin-bottom: var(--spacing-extra-large);
  padding-bottom: var(--spacing-medium);
  border-bottom: 1px solid var(--border-color-light);
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.page-title {
  font-size: var(--font-size-extra-large);
  font-weight: var(--font-weight-semibold);
  margin: 0 0 var(--spacing-mini) 0;
  color: var(--text-color-primary);
}

.page-description {
  font-size: var(--font-size-base);
  color: var(--text-color-regular);
  margin: 0;
}

.header-actions {
  display: flex;
  gap: var(--spacing-small);
}
```

## 📦 组件使用标准

### 统计卡片 (StatCard)
```vue
<el-col :span="6">
  <el-card shadow="hover" class="stat-card">
    <el-statistic 
      :title="统计项名称"
      :value="数值"
      :precision="0"
    >
      <template #prefix>
        <el-icon :style="{ color: iconColor }">
          <component :is="iconComponent" />
        </el-icon>
      </template>
    </el-statistic>
  </el-card>
</el-col>
```

### 内容卡片 (ContentCard)
```vue
<el-col :span="24">
  <el-card>
    <template #header>
      <div class="card-header">
        <span class="card-title">卡片标题</span>
        <div class="card-actions">
          <el-button size="small">操作按钮</el-button>
        </div>
      </div>
    </template>
    
    <!-- 卡片内容 -->
    <div class="card-content">
      <slot />
    </div>
  </el-card>
</el-col>
```

### 数据表格标准
```vue
<el-col :span="24">
  <el-card v-loading="loading">
    <el-table 
      :data="tableData"
      style="width: 100%"
      :default-sort="{ prop: 'date', order: 'descending' }"
      stripe
      highlight-current-row
    >
      <el-table-column prop="name" label="名称" />
      <el-table-column prop="value" label="值" />
      <el-table-column label="操作" width="200">
        <template #default="scope">
          <el-button size="small" type="primary">编辑</el-button>
          <el-button size="small" type="danger">删除</el-button>
        </template>
      </el-table-column>
    </el-table>
    
    <!-- 分页器 -->
    <div class="table-pagination">
      <el-pagination
        v-model:current-page="currentPage"
        v-model:page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
      />
    </div>
  </el-card>
</el-col>
```

### 表单布局标准
```vue
<el-col :span="24">
  <el-card>
    <template #header>
      <span class="card-title">表单标题</span>
    </template>
    
    <el-form 
      :model="formData"
      :rules="formRules"
      ref="formRef"
      label-width="120px"
      label-position="right"
    >
      <el-row :gutter="24">
        <el-col :span="12">
          <el-form-item label="字段名称" prop="fieldName">
            <el-input v-model="formData.fieldName" />
          </el-form-item>
        </el-col>
      </el-row>
      
      <el-form-item>
        <el-button type="primary" @click="submitForm">提交</el-button>
        <el-button @click="resetForm">重置</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</el-col>
```

## 🎯 统一样式规范

### 卡片样式增强
```css
.stat-card {
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--el-box-shadow-light);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-title {
  font-size: var(--font-size-medium);
  font-weight: var(--font-weight-semibold);
  color: var(--text-color-primary);
}

.card-actions {
  display: flex;
  gap: var(--spacing-small);
}

.card-content {
  margin-top: var(--spacing-medium);
}
```

### 表格样式增强
```css
.table-pagination {
  margin-top: var(--spacing-medium);
  display: flex;
  justify-content: flex-end;
}

/* Element Plus 组件微调 */
.el-card {
  margin-bottom: var(--spacing-extra-large);
}

.el-card:last-child {
  margin-bottom: 0;
}
```

## 📐 网格布局规范

### 标准列宽配置
```vue
<!-- 四列等宽布局 (统计卡片) -->
<el-row :gutter="24">
  <el-col :span="6">内容1</el-col>
  <el-col :span="6">内容2</el-col>
  <el-col :span="6">内容3</el-col>
  <el-col :span="6">内容4</el-col>
</el-row>

<!-- 两列布局 (图表并排) -->
<el-row :gutter="24">
  <el-col :span="12">左侧内容</el-col>
  <el-col :span="12">右侧内容</el-col>
</el-row>

<!-- 主次布局 (列表+详情) -->
<el-row :gutter="24">
  <el-col :span="16">主要内容</el-col>
  <el-col :span="8">侧边内容</el-col>
</el-row>

<!-- 全宽布局 (表格、表单) -->
<el-row :gutter="24">
  <el-col :span="24">全宽内容</el-col>
</el-row>
```

## 🚫 禁用的设计模式

### 避免使用的CSS特性
- ❌ `clamp()` 函数 (过度响应式)
- ❌ `vw`, `vh` 单位 (桌面端不需要)
- ❌ 复杂的媒体查询 (专注桌面端)
- ❌ CSS Grid 复杂布局 (使用Element Plus的el-row/el-col)
- ❌ 自定义CSS变量覆盖Element Plus

### 避免的布局模式
- ❌ 流体宽度容器 (`width: 100%`)
- ❌ 过度的动画效果
- ❌ 不规则的间距设置
- ❌ 非标准的组件组合

## ✅ 最佳实践

### 1. 页面开发流程
1. 使用标准页面模板 (`app-page`)
2. 定义页面头部 (标题 + 操作按钮)
3. 规划内容布局 (使用el-row/el-col)
4. 应用标准组件 (StatCard, ContentCard)
5. 遵循间距和字体规范

### 2. 组件开发原则
- 优先使用Element Plus原生组件
- 需要自定义时，基于原生组件扩展
- 保持样式简洁，避免过度设计
- 确保组件可复用性

### 3. 样式编写规范
- 使用CSS变量而非硬编码值
- 遵循BEM命名规范
- 样式写在scoped中
- 避免深层选择器嵌套

## 📊 设计检查清单

在实施设计时，请确认以下要点：

- [ ] 页面最大宽度为1600px
- [ ] 使用标准的24px间距
- [ ] 字体大小符合设计令牌规范
- [ ] 色彩使用Element Plus CSS变量
- [ ] 布局使用el-row/el-col栅格系统
- [ ] 卡片使用el-card原生组件
- [ ] 表格包含分页器和加载状态
- [ ] 按钮遵循Element Plus类型规范
- [ ] 表单使用标准验证和布局
- [ ] 无自定义样式与EP冲突

## 🔄 版本更新

- **v1.0** (当前): 基础设计标准制定
- 后续版本将根据实际使用情况进行优化调整

---

*本文档作为UI开发的强制性标准，所有前端开发都必须严格遵循。*