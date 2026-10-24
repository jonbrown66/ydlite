import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { useAppearanceStore } from './stores/appearance'
import './styles.css'

document.documentElement.lang = 'en'

const pinia = createPinia()
const appearance = useAppearanceStore(pinia)
appearance.initialize()
if (import.meta.hot) import.meta.hot.dispose(() => appearance.stop())

const app = createApp(App)
  .use(pinia)
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
