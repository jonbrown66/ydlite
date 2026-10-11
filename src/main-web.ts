import { createApp } from 'vue'
import LandingPage from './LandingPage.vue'
import './styles/theme.css'
import './styles/controls.css'

document.documentElement.lang = 'en'

createApp(LandingPage).mount('#app')

document.documentElement.classList.add('app-ready')
