import { createApp } from 'vue'
import './styles/landing-colors.css'
import LandingPage from './LandingPage.vue'

document.documentElement.lang = 'en'

createApp(LandingPage).mount('#app')

document.documentElement.classList.add('app-ready')
