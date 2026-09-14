import { createApp } from 'vue'
import { marked } from 'marked'
import markedKatex from 'marked-katex-extension'
import markedCjkFriendly from 'marked-cjk-friendly'
import 'katex/dist/katex.min.css'

import App from './App.vue'
import './style.css'

marked.use(markedKatex({ throwOnError: false }))
marked.use(markedCjkFriendly())
createApp(App).mount('#app')
