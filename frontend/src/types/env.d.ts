/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL: string
  readonly VITE_API_PROTOCOL: string
  readonly VITE_API_HOST: string
  readonly VITE_API_PORT: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

// 全局变量声明
declare const __API_BASE_URL__: string

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}