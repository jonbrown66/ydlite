import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import './styles/tailwind.css'
import './styles.css'

const app = createApp(App)
  .use(createPinia())
  .use(router)

app.mount('#app')

void router.isReady().then(() => {
  requestAnimationFrame(() => {
    document.documentElement.classList.add('app-ready')
    window.setTimeout(() => {
      document.querySelector('.startup-shell')?.remove()
      document.documentElement.classList.remove('tauri-shell-pending')
    }, 180)
  })
})
