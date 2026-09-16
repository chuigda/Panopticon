import { createApp } from 'vue'
import { marked } from 'marked'
import markedCjkFriendly from 'marked-cjk-friendly'
import markedKatex from 'marked-katex-extension'
import 'katex/dist/katex.min.css'

import AppDesktop from './ui_desktop/AppDesktop.vue'

marked.use(markedCjkFriendly())
marked.use(markedKatex({ throwOnError: false }))

if (!isMobileBrowser()) {
  await import('./ui_desktop/style.css')
  createApp(AppDesktop).mount('#app')
} else {
}
