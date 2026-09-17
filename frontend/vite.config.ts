import vue from '@vitejs/plugin-vue'
import { defineConfig, type Plugin } from 'vite'
import { viteSingleFile } from 'vite-plugin-singlefile'
import { createProxyServer } from 'http-proxy-3'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.dirname(fileURLToPath(import.meta.url))
const EXAMPLE_DIR = path.resolve(ROOT, '../example')
const SW_SRC = path.resolve(ROOT, 'sw/sw.js')

// 与 gen-examples.sh 生成的 examples.json 结构一致：目录 -> 对象，文件 -> URL
type ExampleTree = { [name: string]: ExampleTree | string }
function buildExampleTree(dir: string, rel = ''): ExampleTree {
  const tree: ExampleTree = {}
  for (const entry of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const relPath = rel ? `${rel}/${entry.name}` : entry.name
    if (entry.isDirectory()) {
      tree[entry.name] = buildExampleTree(path.join(dir, entry.name), relPath)
    } else if (entry.name.endsWith('.toml')) {
      tree[entry.name] = `/examples/${relPath}`
    }
  }
  return tree
}

// dev 模式下直接从 example/ 目录读取，对应 backstage 的 /examples 路由
function examplesServer(): Plugin {
  return {
    name: 'examples-server',
    configureServer(server) {
      server.middlewares.use('/examples', (req, res) => {
        const rel = decodeURIComponent((req.url ?? '/').split('?')[0]).replace(/^\/+/, '')
        if (rel === '' || rel === 'index.json') {
          res.setHeader('Content-Type', 'application/json')
          res.end(JSON.stringify(buildExampleTree(EXAMPLE_DIR)))
          return
        }
        const file = path.resolve(EXAMPLE_DIR, rel)
        // 防止路径穿越
        if (!file.startsWith(EXAMPLE_DIR + path.sep) || !file.endsWith('.toml') || !fs.existsSync(file)) {
          res.statusCode = 404
          res.end('not found')
          return
        }
        res.setHeader('Content-Type', 'application/toml')
        res.end(fs.readFileSync(file))
      })
    },
  }
}

// 构建期：把 example/ 复制到 dist/examples 并生成 index.json，使 dist/ 可以作为纯静态 PWA 托管；
// 同时给 sw.js 注入构建 ID 以淘汰旧缓存。
function pwaAssets(): Plugin {
  let outDir = 'dist'
  return {
    name: 'pwa-assets',
    apply: 'build',
    configResolved(config) {
      outDir = config.build.outDir
    },
    closeBundle() {
      const examplesOut = path.join(outDir, 'examples')
      fs.rmSync(examplesOut, { recursive: true, force: true })
      fs.cpSync(EXAMPLE_DIR, examplesOut, {
        recursive: true,
        filter: (src) => fs.statSync(src).isDirectory() || src.endsWith('.toml'),
      })
      fs.writeFileSync(path.join(examplesOut, 'index.json'), JSON.stringify(buildExampleTree(EXAMPLE_DIR)))

      fs.writeFileSync(
        path.join(outDir, 'sw.js'),
        fs.readFileSync(SW_SRC, 'utf8').replaceAll('__BUILD_ID__', Date.now().toString(36)),
      )
    },
  }
}

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
  plugins: [vue(), viteSingleFile(), upstreamProxy(), examplesServer(), pwaAssets()],
  server: {
    // 显式绑定 IPv4,避免某些环境用 IPv6 回环导致连不上
    host: '127.0.0.1',
  },
})
