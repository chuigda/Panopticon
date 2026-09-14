import { createApp } from 'vue'
import { marked } from 'marked'
import markedCjkFriendly from 'marked-cjk-friendly'
import markedKatex from 'marked-katex-extension'
import 'katex/dist/katex.min.css'

import App from './App.vue'
import './style.css'

marked.use(markedCjkFriendly())
marked.use(markedKatex({ throwOnError: false }))
createApp(App).mount('#app')
