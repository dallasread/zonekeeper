import { Buffer } from 'buffer'
(globalThis as any).Buffer = Buffer

import { createApp } from 'vue'
import App from './App.vue'
import './style.css'

createApp(App).mount('#app')
