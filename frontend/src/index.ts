import { createApp } from 'vue'
import { marked } from 'marked'
import markedKatex from 'marked-katex-extension'
import markedCjkFriendly from 'marked-cjk-friendly'
import './style.css'
import App from './App.vue'

marked.use(markedKatex({
  throwOnError: false,
  displayMode: false
}))
marked.use(markedCjkFriendly())
createApp(App).mount('#app')
