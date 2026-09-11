import vue from '@vitejs/plugin-vue'
import { defineConfig, type Plugin } from 'vite'
import { viteSingleFile } from 'vite-plugin-singlefile'
import { createProxyServer } from 'http-proxy-3'

// dev 模式代理:按 X-Upstream-Base-Url 头动态选目标,直接转发到远程 upstream,
// 无需跑 backstage Rust 服务器。无该头时回落到本地 backstage。
function upstreamProxy(): Plugin {
  const proxy = createProxyServer({})
  return {
    name: 'upstream-proxy',
    configureServer(server) {
      server.middlewares.use('/v1', (req, res) => {
        // Connect 挂在 /v1 上会把前缀剥掉,这里拼回完整路径,
        // 使上游收到的请求仍是 base + /v1/...
        req.url = '/v1' + req.url
        const base = req.headers['x-upstream-base-url']
        const target =
          typeof base === 'string' && base.length > 0
            ? base.trim()
            : 'http://127.0.0.1:3000'
        // 头只用于选路,不转发给上游
        proxy.web(req, res, { target, changeOrigin: true }, (err) => {
          if (err) {
            res.statusCode = 502
            res.end(`proxy error: ${err.message}`)
          }
        })
      })
      proxy.on('proxyReq', (proxyReq) => {
        proxyReq.removeHeader('x-upstream-base-url')
      })
    },
  }
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue(), viteSingleFile(), upstreamProxy()],
  server: {
    // 显式绑定 IPv4,避免某些环境用 IPv6 回环导致连不上
    host: '127.0.0.1',
  },
})
