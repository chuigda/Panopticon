# Panopticon

这天，从类脑社区归来的 Chuigda Whitegive，带来了比 [9M68](https://github.com/chuigda/9M68)，[mangekyou](https://github.com/chuigda/mangekyou-web) 和 [conducting-department-cli](https://github.com/chuigda/conducting-department-cli) 都更富有威力的 Panopticon。

你站在中央的塔楼上，整个世界在你的目光下运转。

## 快速开始

### 开发模式

```sh
cd frontend
pnpm install
pnpm dev
```

Vite dev server 会根据 `X-Upstream-Base-Url` 头直接把 `/v1/*` 请求代理到上游 API，无需启动后端。

### 生产构建

```sh
cd frontend
pnpm build          # 产出单文件 dist/index.html

cd ../backstage
cargo build --release
./target/release/panopticon 3000
```

`backstage` 是一个极简 Rust (axum) 服务：把 `index.html` 内嵌进二进制，并为 `/v1/messages` 与 `/v1/chat/completions` 做上游转发。单个可执行文件即完整部署。

## 角色卡格式

所有角色卡都是 TOML，首行注释声明类型。示例见 [example/](example/)。

### 模拟器卡 `*.simulator.chr.toml`

```toml
# type: simulator
kind = "cd"                          # 或 "mk"
universeName = "Harry Potter universe"
literalWorkName = "Harry Potter series"
prologue = """..."""
language = "zh_CN"                   # 或 "en_US"

[statusBar]
format = """..."""                   # Jinja 风格模板，必填
rule = """..."""                     # 状态栏维护规则
example = """..."""                  # 一份填好的示例

[simulator]                          # 全部可选
tasks = """..."""
commands = """..."""                 # mk 专用：$ 命令定义
world = """..."""
characters = """..."""
database = """..."""
behaviors = """..."""
prohibitions = """..."""
sections = """..."""

[memory]                             # 全部可选
rules = """..."""
sections = """..."""
```

### 玩家卡 `*.player.chr.toml`（mk 模式）

```toml
# type: player
name = "安东·尤宁采夫"
data = """..."""                     # 玩家角色设定
```

### Addon 卡 `*.addon.chr.toml`

```toml
# type: addon
kind = "*"                           # "cd" / "mk" / "*"
id = "hachimi"
name = "哈基米喔南北绿豆"

[statusBar]                          # 可选，字段同模拟器卡
[simulator]                          # 可选，字段同模拟器卡
[memory]                             # 可选，字段同模拟器卡
```
