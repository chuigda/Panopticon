// 缓存策略：
//   - 应用壳（/、manifest、图标）：network-first，离线回落缓存
//   - /examples/*：stale-while-revalidate
//   - /v1/*（模型 API）与跨域请求：不拦截
// 缓存名中的占位符由 vite 构建注入构建 ID，每次构建产生新缓存并淘汰旧缓存。
const CACHE = 'panopticon-__BUILD_ID__'
const SHELL = ['/', '/manifest.webmanifest', '/icons/icon-192.png', '/icons/icon-512.png']

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE).then((c) => c.addAll(SHELL)).then(() => self.skipWaiting())
  )
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  )
})

self.addEventListener('fetch', (event) => {
  const req = event.request
  if (req.method !== 'GET') return
  const url = new URL(req.url)
  if (url.origin !== self.location.origin) return
  if (url.pathname.startsWith('/v1/')) return

  if (url.pathname.startsWith('/examples')) {
    event.respondWith(staleWhileRevalidate(req))
    return
  }

  // SPA 导航一律回到 /
  const key = req.mode === 'navigate' ? '/' : req
  event.respondWith(networkFirst(req, key))
})

async function networkFirst(req, key) {
  const cache = await caches.open(CACHE)
  try {
    const res = await fetch(req)
    if (res.ok) cache.put(key, res.clone())
    return res
  } catch {
    const hit = await cache.match(key)
    if (hit) return hit
    throw new Error('offline and not cached')
  }
}

async function staleWhileRevalidate(req) {
  const cache = await caches.open(CACHE)
  const hit = await cache.match(req)
  const refresh = fetch(req).then((res) => {
    if (res.ok) cache.put(req, res.clone())
    return res
  }).catch(() => undefined)
  return hit ?? (await refresh) ?? new Response('offline', { status: 503 })
}
