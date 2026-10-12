import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
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
            if (!isTauriBuild) return html
              .replace(/<style>[\s\S]*?<\/style>/, '')
              .replace(/    <div class="startup-shell"[\s\S]*?(?=    <script type="module")/, '')
            return html
              .replace('<html lang="en">', '<html lang="en" class="tauri-shell-pending">')
              .replace(/<noscript>[\s\S]*?<\/noscript>/, '')
              .replace('/src/main-web.ts', '/src/main-app.ts')
          },
        },
      },
      vue(),
      tailwindcss(),
    ],
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
      },
    },
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
