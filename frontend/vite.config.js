import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueDevTools from 'vite-plugin-vue-devtools'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    vueDevTools(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    },
  },
  server: {
    port: process.env.VITE_DEV_PORT || 5173,
    proxy: {
      '/api': {
        target: process.env.VITE_API_URL || `http://localhost:${process.env.VITE_API_PORT || '3000'}`,
        changeOrigin: true,
      },
    },
  },
})
