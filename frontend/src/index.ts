import { createApp } from 'vue'
import { marked } from 'marked'
import markedCjkFriendly from 'marked-cjk-friendly'
import markedKatex from 'marked-katex-extension'
import 'katex/dist/katex.min.css'

import AppDesktop from './ui_desktop/AppDesktop.vue'
import AppMobile from './ui_mobile/AppMobile.vue'
import './style.css'

marked.use(markedCjkFriendly())
marked.use(markedKatex({ throwOnError: false }))

const urlParams = new URLSearchParams(window.location.search)
const forceMobile = urlParams.has('mobile') || urlParams.get('ui') === 'mobile'
const forceDesktop = urlParams.has('desktop') || urlParams.get('ui') === 'desktop'
const isMobile = forceMobile || (!forceDesktop && isMobileBrowser())

if (!isMobile) {
  await import('./ui_desktop/style.css')
  createApp(AppDesktop).mount('#app')
} else {
  await import('./ui_mobile/style.css')
  createApp(AppMobile).mount('#app')
}
