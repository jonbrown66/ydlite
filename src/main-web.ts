import { createApp } from 'vue'
import './styles/theme.css'
import './styles/controls.css'
import LandingPage from './LandingPage.vue'

document.documentElement.lang = 'en'

createApp(LandingPage).mount('#app')

document.documentElement.classList.add('app-ready')
