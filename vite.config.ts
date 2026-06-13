import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig(({ mode }) => {
  const isTauriBuild = mode === 'tauri'

  return {
    plugins: [
      {
        name: 'ydlite-entry',
        enforce: 'pre',
        transformIndexHtml: {
          order: 'pre',
          handler(html) {
            return isTauriBuild ? html.replace('/src/main-web.ts', '/src/main-app.ts') : html
          },
        },
      },
      vue(),
    ],
    publicDir: isTauriBuild ? false : 'public',
    clearScreen: false,
    server: {
      port: 1420,
      strictPort: true,
      watch: {
        ignored: ['**/src-tauri/**'],
      },
    },
  }
})
