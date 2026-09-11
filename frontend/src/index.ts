import { createApp } from 'vue'
import { marked } from 'marked'
import markedCjkFriendly from 'marked-cjk-friendly'
import './style.css'
import App from './App.vue'

marked.use(markedCjkFriendly())
createApp(App).mount('#app')
