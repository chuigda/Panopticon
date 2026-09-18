<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'
import { ConfigSchema, defaultConfig, type Config } from '../config.ts'
import {
  parseAdditionalCHR,
  parsePlayerCHR,
  parseSimulatorCHR,
  type AdditionalCHR,
  type PlayerCHR,
  type SimulatorCHR,
} from '../session/chr.ts'
import type { ChatAssistantMessage, ChatMemoryMessage, ChatMessage } from '../session/message.ts'
import {
  manualCompressMemory,
  pipelineGenerate,
  regenerateStatusBar,
  type PipelineStage,
} from '../pipeline/pipeline.ts'
import { createAskQuestionTool } from '../pipeline/tool.ts'
import Button from './Button.vue'
import ChatBubbleMobile from './ChatBubbleMobile.vue'
import Dialog from './Dialog.vue'
import ExampleBrowserMobile from './ExampleBrowserMobile.vue'
import Input from './Input.vue'
import MarkdownEditMobile from './MarkdownEditMobile.vue'
import ModelConfigFormMobile from './ModelConfigFormMobile.vue'
import PlainTextEditMobile from './PlainTextEditMobile.vue'
import ToggleButtonGroup from './ToggleButtonGroup.vue'

// ---------- 持久化 ----------
const CONFIG_KEY = 'cd.config'
const CHR_KEY = 'cd.chr'

function loadJSON<T>(key: string): T | undefined {
  try {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as T) : undefined
  } catch {
    return undefined
  }
}

const storedConfig = loadJSON<Config>(CONFIG_KEY)
const config = reactive<Config>({
  ...defaultConfig,
  ...storedConfig,
  chatModel: { ...defaultConfig.chatModel, ...storedConfig?.chatModel },
  statusBarModel: { ...defaultConfig.statusBarModel, ...storedConfig?.statusBarModel },
  memoryModel: { ...defaultConfig.memoryModel, ...storedConfig?.memoryModel },
})
watch(config, (c) => localStorage.setItem(CONFIG_KEY, JSON.stringify(c)), { deep: true })

interface CHRBundle {
  simulator?: SimulatorCHR
  additional: AdditionalCHR[]
  player?: PlayerCHR
}
const chr = reactive<CHRBundle>(loadJSON<CHRBundle>(CHR_KEY) ?? { additional: [] })
watch(chr, (c) => localStorage.setItem(CHR_KEY, JSON.stringify(c)), { deep: true })

// ---------- 会话与流程 ----------
const chatMessages = reactive<ChatMessage[]>([])
const input = ref('')
const stage = ref<'' | PipelineStage | 'question'>('')
const busy = computed(() => stage.value !== '')
let controller: AbortController | undefined

const mode = computed(() => chr.simulator?.kind)
const modeLabel = computed(() => (mode.value === 'cd' ? '导演部' : mode.value === 'mk' ? '万华镜' : '未加载'))
const userLabel = computed(() => (mode.value === 'mk' ? chr.player?.name ?? '玩家' : '导演部'))

const stageLabel: Record<typeof stage.value, string> = {
  '': '就绪',
  simulate: '模拟器生成中',
  statusBar: '状态栏更新中',
  memory: '记忆压缩中',
  question: '等待回答',
}

const lastMessages = computed(() => {
  let last: ChatMessage | undefined
  let assistant: ChatAssistantMessage | undefined
  let memory: ChatMemoryMessage | undefined
  for (let i = chatMessages.length - 1; i >= 0; i--) {
    const m = chatMessages[i]
    if (!last && m.$k !== 'error') last = m
    if (!assistant && m.$k === 'assistant') assistant = m
    if (!memory && m.$k === 'memory') memory = m
    if (last && assistant && memory) break
  }
  return { last, assistant, memory }
})

const lastMessage = computed(() => lastMessages.value.last)
const lastAssistant = computed(() => lastMessages.value.assistant)
const lastMemory = computed(() => lastMessages.value.memory)

// ---------- 移动端视图切换 ----------
type ActiveTab = 'chat' | 'status' | 'memory'
const currentTab = ref<ActiveTab>('chat')
const navTabs = computed(() => [
  { value: 'chat', label: `对话 (${chatMessages.length})` },
  { value: 'status', label: `状态栏${lastAssistant.value?.statusBar ? ' •' : ''}` },
  { value: 'memory', label: `记忆${lastMemory.value ? ' •' : ''}` },
])

// 移动端管理菜单抽屉
const menuDialogOpen = ref(false)

// 示例下载
const examplesDialogOpen = ref(false)
function openExamplesDialog() {
  menuDialogOpen.value = false
  examplesDialogOpen.value = true
}

// ---------- ask_question ----------
interface PendingQuestion {
  id: number
  prompt: string
  options: string[]
  selected: string[]
  text: string
  resolve: (answer: string) => void
}

const questions = ref<PendingQuestion[]>([])
const activeQuestionIndex = ref(0)
const activeQuestion = computed(() => questions.value[activeQuestionIndex.value])
const questionTabs = computed(() => questions.value.map((q, i) => ({ value: String(q.id), label: `Q${i + 1}` })))
const activeOptionItems = computed(() => activeQuestion.value?.options.map((o) => ({ value: o, label: o })) ?? [])
const activeAnswer = computed(() => {
  const question = activeQuestion.value
  if (!question) return ''
  return [...question.selected, question.text.trim()].filter(Boolean).join('\n')
})
let questionSeq = 0
let stageBeforeQuestion: typeof stage.value = ''

const tools = [
  createAskQuestionTool(
    ({ prompt, options }) =>
      new Promise<string>((resolve) => {
        if (!questions.value.length) {
          stageBeforeQuestion = stage.value
          stage.value = 'question'
          activeQuestionIndex.value = 0
        }
        questions.value.push({ id: questionSeq++, prompt, options: options ?? [], selected: [], text: '', resolve })
        // 切回对话视图保证用户能看到问题
        currentTab.value = 'chat'
        nextTick(() => inputEl.value?.focus())
      }),
  ),
]

function switchQuestion(id: string) {
  const i = questions.value.findIndex((q) => String(q.id) === id)
  if (i >= 0) activeQuestionIndex.value = i
}

function submitAnswer() {
  const q = activeQuestion.value
  const answer = activeAnswer.value
  if (!q || !answer) return
  q.resolve(answer)
  questions.value.splice(activeQuestionIndex.value, 1)
  activeQuestionIndex.value = Math.min(activeQuestionIndex.value, questions.value.length - 1)
  if (!questions.value.length) stage.value = stageBeforeQuestion
}

function abortQuestions() {
  for (const q of questions.value) q.resolve('(aborted)')
  questions.value = []
  stage.value = stageBeforeQuestion
}

// ---------- 运行与控制 ----------
const notice = ref('')
function warn(msg: string) {
  notice.value = msg
  setTimeout(() => (notice.value === msg ? (notice.value = '') : undefined), 4000)
}

function playerArg() {
  return mode.value === 'mk' ? chr.player : undefined
}

const readiness = computed<{ ok: boolean; reason: string }>(() => {
  if (!chr.simulator) return { ok: false, reason: '未加载 Simulator' }
  if (chr.simulator.kind === 'mk' && !chr.player) return { ok: false, reason: '未加载 Player' }
  if (!config.chatModel.uri || !config.chatModel.apiKey) return { ok: false, reason: '模拟器 API 未配置' }
  if (!config.statusBarModel.uri || !config.statusBarModel.apiKey) return { ok: false, reason: '状态栏 API 未配置' }
  if (!config.memoryModel.uri || !config.memoryModel.apiKey) return { ok: false, reason: '记忆 API 未配置' }
  return { ok: true, reason: '就绪' }
})

async function run(fn: (signal: AbortSignal) => Promise<void>) {
  if (busy.value || !readiness.value.ok) return
  controller = new AbortController()
  try {
    await fn(controller.signal)
  } finally {
    stage.value = ''
    controller = undefined
  }
}

const canRegenerate = computed(() => !input.value.trim() && lastMessage.value?.$k === 'assistant')
const canSend = computed(() => {
  if (busy.value || !readiness.value.ok) return false
  return !!input.value.trim() || canRegenerate.value
})
const regenDialogOpen = ref(false)
const shouldDiscardToolCalls = ref(false)

function onPrimary() {
  if (canRegenerate.value) {
    shouldDiscardToolCalls.value = false
    regenDialogOpen.value = true
    return
  }
  if (canSend.value) send(false)
}

function send(discardToolCalls: boolean) {
  regenDialogOpen.value = false
  if (!readiness.value.ok) return
  const text = input.value.trim()
  if (text) {
    chatMessages.push({ $k: 'user', content: text })
    input.value = ''
  }
  run((signal) =>
    pipelineGenerate(
      config,
      tools,
      chr.simulator!,
      chr.additional,
      playerArg(),
      chatMessages,
      discardToolCalls,
      (s) => (stage.value = s),
      signal,
    ),
  )
}

function regenStatus() {
  menuDialogOpen.value = false
  currentTab.value = 'status'
  run((signal) =>
    regenerateStatusBar(config, chr.simulator!, chr.additional, playerArg(), chatMessages, (s) => (stage.value = s), signal),
  )
}

function compress() {
  menuDialogOpen.value = false
  currentTab.value = 'memory'
  run((signal) =>
    manualCompressMemory(config, chr.simulator!, chr.additional, playerArg(), chatMessages, (s) => (stage.value = s), signal),
  )
}

function abort() {
  if (questions.value.length) abortQuestions()
  controller?.abort()
}

function updateContent(i: number, value: string) {
  const m = chatMessages[i]
  if ('content' in m) m.content = value
}

function updateStatusBar(value: string) {
  if (lastAssistant.value) lastAssistant.value.statusBar = value
}

function updateMemory(value: string) {
  if (lastMemory.value) lastMemory.value.content = value
}

function removeMessage(i: number) {
  chatMessages.splice(i, 1)
}

function isSubmitKey(e: KeyboardEvent) {
  return e.key === 'Enter' && e.ctrlKey && !e.isComposing
}

function onInputKeydown(e: KeyboardEvent) {
  if (isSubmitKey(e)) {
    e.preventDefault()
    onPrimary()
  }
}

function onQuestionKeydown(e: KeyboardEvent) {
  if (isSubmitKey(e)) {
    e.preventDefault()
    submitAnswer()
  }
}

// ---------- 文件与会话 ----------
function pickFile(accept: string, multiple = false): Promise<File[]> {
  return new Promise((resolve) => {
    const el = document.createElement('input')
    el.type = 'file'
    el.accept = accept
    el.multiple = multiple
    el.onchange = () => resolve(Array.from(el.files ?? []))
    el.oncancel = () => resolve([])
    el.click()
  })
}

function download(name: string, text: string) {
  const a = document.createElement('a')
  a.href = URL.createObjectURL(new Blob([text], { type: 'application/json' }))
  a.download = name
  a.click()
  URL.revokeObjectURL(a.href)
}

function saveSession() {
  menuDialogOpen.value = false
  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  download(`session-${chr.simulator?.universeName ?? 'untitled'}-${stamp}.json`, JSON.stringify(chatMessages, null, 2))
}

async function loadSession() {
  menuDialogOpen.value = false
  if (busy.value) return
  const [file] = await pickFile('application/json,.json')
  if (!file) return
  try {
    const data = JSON.parse(await file.text())
    if (!Array.isArray(data) || !data.every((m) => typeof m?.$k === 'string')) throw new Error('不是有效的会话文件')
    chatMessages.splice(0, chatMessages.length, ...(data as ChatMessage[]))
    currentTab.value = 'chat'
  } catch (e) {
    warn(`读取会话失败：${e instanceof Error ? e.message : e}`)
  }
}

// ---------- API 配置 ----------
const apiDialogOpen = ref(false)
const apiConfigStatus = ref('')
const apiConfigError = ref('')

function openApiDialog() {
  menuDialogOpen.value = false
  apiDialogOpen.value = true
}

function exportApiConfig() {
  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  download(`api-config-${stamp}.json`, JSON.stringify(config, null, 2))
  apiConfigError.value = ''
  apiConfigStatus.value = '配置已导出'
}

async function importApiConfig() {
  const [file] = await pickFile('application/json,.json')
  if (!file) return

  try {
    const importedConfig: Config = ConfigSchema.parse(JSON.parse(await file.text()))
    Object.assign(config, importedConfig)
    apiConfigError.value = ''
    apiConfigStatus.value = '配置已导入'
  } catch {
    apiConfigStatus.value = ''
    apiConfigError.value = '导入失败：格式无效或字段不全'
  }
}

type ModelKey = 'chatModel' | 'statusBarModel' | 'memoryModel'
const apiTab = ref<ModelKey>('chatModel')
const apiTabs = [
  { value: 'chatModel', label: '模拟器' },
  { value: 'statusBarModel', label: '状态栏' },
  { value: 'memoryModel', label: '记忆' },
] as const

function setNum(key: 'inlineMessageLimit' | 'compressionSize' | 'outputLength', v: string | undefined) {
  const n = Number(v)
  if (Number.isFinite(n) && n > 0) config[key] = n
}

// ---------- 模拟配置 ----------
const simDialogOpen = ref(false)
const simError = ref('')

function openSimDialog() {
  menuDialogOpen.value = false
  simDialogOpen.value = true
}

async function loadSimulatorCHR() {
  const [file] = await pickFile('.toml')
  if (!file) return
  try {
    const parsed = parseSimulatorCHR(await file.text())
    const kept = chr.additional.filter((a) => a.kind === '*' || a.kind === parsed.kind)
    const droppedCount = chr.additional.length - kept.length
    chr.simulator = parsed
    chr.additional = kept
    if (parsed.kind !== 'mk') chr.player = undefined
    simError.value = droppedCount ? `已移除 ${droppedCount} 个不匹配的 AdditionalCHR` : ''
  } catch (e) {
    simError.value = `Simulator 解析失败：${e instanceof Error ? e.message : e}`
  }
}

async function loadAdditionalCHR() {
  if (!chr.simulator) return
  const files = await pickFile('.toml', true)
  const errors: string[] = []
  for (const file of files) {
    try {
      const parsed = parseAdditionalCHR(await file.text())
      if (parsed.kind !== '*' && parsed.kind !== chr.simulator.kind) {
        errors.push(`${file.name}: 类型 ${parsed.kind} 与 Simulator (${chr.simulator.kind}) 不一致`)
        continue
      }
      const idx = chr.additional.findIndex((a) => a.id === parsed.id)
      if (idx >= 0) chr.additional[idx] = parsed
      else chr.additional.push(parsed)
    } catch (e) {
      errors.push(`${file.name}: ${e instanceof Error ? e.message : e}`)
    }
  }
  simError.value = errors.join('\n')
}

async function loadPlayerCHR() {
  const [file] = await pickFile('.toml')
  if (!file) return
  try {
    chr.player = parsePlayerCHR(await file.text())
    simError.value = ''
  } catch (e) {
    simError.value = `Player 解析失败：${e instanceof Error ? e.message : e}`
  }
}

// ---------- 滚动与显示 ----------
const scrollEl = ref<HTMLElement | null>(null)
const inputEl = ref<HTMLTextAreaElement | null>(null)
let stickToBottom = true

function onScroll() {
  const el = scrollEl.value
  if (!el) return
  stickToBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 50
}

watch(
  chatMessages,
  async () => {
    if (!stickToBottom || currentTab.value !== 'chat') return
    await nextTick()
    const el = scrollEl.value
    if (el) el.scrollTop = el.scrollHeight
  },
  { deep: true, flush: 'post' },
)

onMounted(() => {
  const el = scrollEl.value
  if (el) el.scrollTop = el.scrollHeight
})

function preprocessStatusBar(text: string) {
  return text
    .split('\n')
    .filter((line) => !/^\s*TIME:/.test(line))
    .join('  \n')
    .replace(/^(\s*\n)+/, '')
}
</script>

<template>
  <div class="m-app">
    <!-- 顶部状态栏与工具栏 -->
    <header class="m-topbar">
      <div class="m-topbar__left">
        <span class="m-topbar__mode" :class="{ 'is-loaded': mode }">{{ modeLabel }}</span>
        <span class="m-topbar__ready" :class="{ 'is-ok': readiness.ok }">
          <span class="m-topbar__ready-dot" />
          {{ readiness.reason }}
        </span>
      </div>

      <div class="m-topbar__right">
        <Button v-if="busy" variant="danger" size="sm" @click="abort">中止</Button>
        <Button variant="ghost" size="sm" @click="menuDialogOpen = true">菜单 ☰</Button>
      </div>
    </header>

    <!-- 顶部轻提示 -->
    <div v-if="notice" class="m-notice" role="alert">
      {{ notice }}
    </div>

    <!-- 竖屏多视图切换 Tabs -->
    <nav class="m-nav">
      <ToggleButtonGroup
        :model-value="currentTab"
        :options="navTabs"
        aria-label="主要视图切换"
        @update:model-value="currentTab = $event as ActiveTab"
      />
    </nav>

    <!-- 主展示区 -->
    <main class="m-main">
      <!-- 视图 1: 对话 -->
      <section v-show="currentTab === 'chat'" class="m-chat-view">
        <div ref="scrollEl" class="m-chat__scroll" @scroll="onScroll">
          <div v-if="!chatMessages.length" class="m-chat__empty">
            <p class="m-chat__empty-title">
              {{ chr.simulator ? `${chr.simulator.literalWorkName} · ${chr.simulator.universeName}` : '尚未加载 SimulatorCHR' }}
            </p>
            <p class="m-chat__empty-desc">
              {{ chr.simulator ? '配置已就绪，可在下方输入行动开始模拟。' : '请点击右上角「菜单」进入「模拟配置」加载剧本。' }}
            </p>
          </div>

          <ChatBubbleMobile
            v-for="(m, i) in chatMessages"
            :key="i"
            :message="m"
            :index="i"
            :user-label="userLabel"
            :disabled="busy"
            @update:content="updateContent(i, $event)"
            @remove="removeMessage(i)"
          />
        </div>

        <!-- 底部输入与操作区 -->
        <footer class="m-composer">
          <!-- 问答模式 -->
          <div v-if="activeQuestion" class="m-composer__question">
            <div class="m-composer__question-header">
              <span class="m-composer__question-tag">问题决策</span>
              <div v-if="questions.length > 1" class="m-composer__qtabs">
                <ToggleButtonGroup
                  :model-value="String(activeQuestion.id)"
                  :options="questionTabs"
                  aria-label="待答问题"
                  @update:model-value="switchQuestion"
                />
              </div>
            </div>

            <pre class="m-composer__prompt">{{ activeQuestion.prompt }}</pre>

            <div v-if="activeOptionItems.length" class="m-composer__options" role="group" aria-label="候选答案">
              <label v-for="option in activeOptionItems" :key="option.value" class="m-composer__option-card">
                <input v-model="activeQuestion.selected" type="checkbox" :value="option.value" />
                <span>{{ option.label }}</span>
              </label>
            </div>

            <textarea
              ref="inputEl"
              v-model="activeQuestion.text"
              class="m-composer__textarea"
              rows="2"
              :placeholder="activeOptionItems.length ? '可自由补充说明...' : '请输入自由回答...'"
              @keydown="onQuestionKeydown"
            />

            <div class="m-composer__action-bar">
              <div class="m-stage is-busy">
                <span class="m-stage__dot" />
                <span>{{ stageLabel[stage] }}</span>
              </div>
              <div class="m-composer__btns">
                <Button variant="danger" size="sm" @click="abort">中止</Button>
                <Button size="sm" :disabled="!activeAnswer" @click="submitAnswer">提交回答</Button>
              </div>
            </div>
          </div>

          <!-- 常规输入模式 -->
          <div v-else class="m-composer__normal">
            <textarea
              ref="inputEl"
              v-model="input"
              class="m-composer__textarea"
              rows="3"
              :placeholder="mode === 'mk' ? `${userLabel} 的行动指令...` : '输入导演部指令...'"
              :disabled="busy"
              @keydown="onInputKeydown"
            />

            <div class="m-composer__action-bar">
              <div class="m-stage" :class="{ 'is-busy': busy }">
                <span class="m-stage__dot" />
                <span>{{ stageLabel[stage] }}</span>
              </div>

              <div class="m-composer__btns">
                <Button
                  v-if="canRegenerate"
                  variant="secondary"
                  size="sm"
                  :disabled="busy"
                  @click="onPrimary"
                >
                  重新生成
                </Button>
                <Button
                  size="sm"
                  :disabled="!canSend"
                  @click="onPrimary"
                >
                  {{ canRegenerate ? '重新生成' : '发送' }}
                </Button>
              </div>
            </div>
          </div>
        </footer>
      </section>

      <!-- 视图 2: 状态栏 -->
      <section v-show="currentTab === 'status'" class="m-panel-view">
        <header class="m-panel__header">
          <div class="m-panel__title">
            <span>状态栏</span>
            <span class="m-panel__subtitle">STATUS PANEL</span>
          </div>
          <div class="m-panel__actions">
            <Button
              variant="secondary"
              size="sm"
              :disabled="busy || lastMessage?.$k !== 'assistant'"
              @click="regenStatus"
            >
              重生成状态栏
            </Button>
          </div>
        </header>

        <div class="m-panel__body">
          <div v-if="lastAssistant" class="m-panel__content">
            <MarkdownEditMobile
              :model-value="lastAssistant.statusBar"
              :preprocess="preprocessStatusBar"
              :rows="14"
              @update:model-value="updateStatusBar"
            />
          </div>
          <div v-else class="m-panel__empty">
            <p>暂无状态栏数据</p>
            <p class="m-panel__empty-desc">模拟器生成助手消息后，解析出的世界状态、角色状态将在此呈现。</p>
          </div>
        </div>
      </section>

      <!-- 视图 3: 记忆 -->
      <section v-show="currentTab === 'memory'" class="m-panel-view">
        <header class="m-panel__header">
          <div class="m-panel__title">
            <span>压缩记忆</span>
            <span class="m-panel__subtitle">MEMORY CONTEXT</span>
          </div>
          <div class="m-panel__actions">
            <Button
              variant="secondary"
              size="sm"
              :disabled="busy || !chatMessages.length"
              @click="compress"
            >
              压缩记忆
            </Button>
          </div>
        </header>

        <div class="m-panel__body">
          <div v-if="lastMemory" class="m-panel__content">
            <PlainTextEditMobile
              :model-value="lastMemory.content"
              :rows="16"
              @update:model-value="updateMemory"
            />
          </div>
          <div v-else class="m-panel__empty">
            <p>暂无压缩记忆</p>
            <p class="m-panel__empty-desc">随着会话进行，达到设定的条数后会自动生成总结压缩，亦可点击右上角手动触发。</p>
          </div>
        </div>
      </section>
    </main>

    <!-- 重新生成确认弹窗 -->
    <Dialog v-model="regenDialogOpen" title="重新生成">
      <p>将丢弃最后一条模拟器输出并重新生成响应。</p>
      <label class="m-regen-checkbox">
        <input v-model="shouldDiscardToolCalls" type="checkbox" />
        <span>同时丢弃工具调用 (Discard Tools)</span>
      </label>
      <template #footer>
        <Button variant="ghost" @click="regenDialogOpen = false">取消</Button>
        <Button @click="send(shouldDiscardToolCalls)">确定重生成</Button>
      </template>
    </Dialog>

    <!-- 移动端管理菜单抽屉/弹窗 -->
    <Dialog v-model="menuDialogOpen" title="控制台菜单" close-label="关闭">
      <div class="m-menu">
        <!-- 概览卡片 -->
        <div class="m-menu__info-card">
          <div class="m-menu__info-item">
            <span class="m-menu__info-label">模拟工作区</span>
            <span class="m-menu__info-val">{{ chr.simulator ? chr.simulator.literalWorkName : '未加载剧本' }}</span>
          </div>
          <div class="m-menu__info-item">
            <span class="m-menu__info-label">运行模式</span>
            <span class="m-menu__info-val">{{ modeLabel }} ({{ mode ?? 'none' }})</span>
          </div>
          <div class="m-menu__info-item">
            <span class="m-menu__info-label">系统就绪</span>
            <span class="m-menu__info-val" :class="{ 'is-ok': readiness.ok }">{{ readiness.reason }}</span>
          </div>
        </div>

        <!-- 菜单操作按钮组 -->
        <div class="m-menu__section">
          <div class="m-menu__section-title">模拟与配置</div>
          <div class="m-menu__grid3">
            <Button variant="secondary" block @click="openSimDialog">
              模拟配置
            </Button>
            <Button variant="secondary" block @click="openApiDialog">
              API 配置
            </Button>
            <Button variant="secondary" block @click="openExamplesDialog">
              示例下载
            </Button>
          </div>
        </div>

        <div class="m-menu__section">
          <div class="m-menu__section-title">会话管理</div>
          <div class="m-menu__grid">
            <Button variant="secondary" block :disabled="busy" @click="loadSession">
              读取会话 (.json)
            </Button>
            <Button variant="secondary" block :disabled="!chatMessages.length" @click="saveSession">
              保存会话 (.json)
            </Button>
          </div>
        </div>

        <div class="m-menu__section">
          <div class="m-menu__section-title">管线快捷指令</div>
          <div class="m-menu__grid">
            <Button
              variant="secondary"
              block
              :disabled="busy || lastMessage?.$k !== 'assistant'"
              @click="regenStatus"
            >
              重生成状态栏
            </Button>
            <Button
              variant="secondary"
              block
              :disabled="busy || !chatMessages.length"
              @click="compress"
            >
              手动压缩记忆
            </Button>
          </div>
        </div>
      </div>
      <template #footer>
        <Button block @click="menuDialogOpen = false">返回应用</Button>
      </template>
    </Dialog>

    <!-- API 配置弹窗 -->
    <Dialog v-model="apiDialogOpen" title="API 配置">
      <div class="m-api-dialog">
        <div class="m-api-dialog__top-actions">
          <Button variant="secondary" size="sm" @click="importApiConfig">导入配置</Button>
          <Button variant="secondary" size="sm" @click="exportApiConfig">导出配置</Button>
        </div>
        <p v-if="apiConfigStatus" class="m-api-dialog__status" role="status">{{ apiConfigStatus }}</p>
        <p v-if="apiConfigError" class="m-api-dialog__error" role="alert">{{ apiConfigError }}</p>

        <div class="m-api-dialog__globals">
          <Input
            :model-value="String(config.inlineMessageLimit)"
            label="内联上限"
            type="number"
            @update:model-value="setNum('inlineMessageLimit', $event)"
          />
          <Input
            :model-value="String(config.compressionSize)"
            label="压缩条数"
            type="number"
            @update:model-value="setNum('compressionSize', $event)"
          />
          <Input
            :model-value="String(config.outputLength)"
            label="输出长度"
            type="number"
            @update:model-value="setNum('outputLength', $event)"
          />
        </div>

        <div class="m-api-dialog__tabs">
          <ToggleButtonGroup
            label="配置模型"
            :model-value="apiTab"
            :options="apiTabs"
            aria-label="配置目标模型"
            @update:model-value="apiTab = $event as ModelKey"
          />
        </div>

        <ModelConfigFormMobile v-model="config[apiTab]" />
      </div>
      <template #footer>
        <Button block @click="apiDialogOpen = false">完成</Button>
      </template>
    </Dialog>

    <!-- 模拟配置弹窗 -->
    <Dialog v-model="simDialogOpen" title="模拟配置 (CHR)" @close="simError = ''">
      <div class="m-sim-dialog">
        <!-- SimulatorCHR -->
        <div class="m-sim-item">
          <div class="m-sim-item__info">
            <span class="input-field__label">SimulatorCHR (剧本)</span>
            <span v-if="chr.simulator" class="m-sim-item__val">
              {{ chr.simulator.literalWorkName }} ({{ chr.simulator.kind }})
            </span>
            <span v-else class="m-sim-item__muted">未加载</span>
          </div>
          <Button variant="secondary" size="sm" @click="loadSimulatorCHR">
            {{ chr.simulator ? '替换' : '加载' }}
          </Button>
        </div>

        <!-- AdditionalCHR -->
        <div class="m-sim-item">
          <div class="m-sim-item__info">
            <span class="input-field__label">AdditionalCHR (插件/设定)</span>
            <span v-if="!chr.additional.length" class="m-sim-item__muted">尚未添加</span>
            <span v-else class="m-sim-item__val">{{ chr.additional.length }} 个已加载</span>
          </div>
          <Button variant="secondary" size="sm" :disabled="!chr.simulator" @click="loadAdditionalCHR">
            添加
          </Button>
        </div>

        <!-- AdditionalCHR 列表 -->
        <ul v-if="chr.additional.length" class="m-sim-list">
          <li v-for="(a, i) in chr.additional" :key="a.id">
            <div class="m-sim-list__desc">
              <span class="m-sim-list__name">{{ a.name ?? a.id }}</span>
              <span class="m-sim-list__tag">{{ a.kind }}</span>
            </div>
            <button class="m-sim-list__remove" type="button" title="删除" @click="chr.additional.splice(i, 1)">
              ✕
            </button>
          </li>
        </ul>

        <!-- PlayerCHR (仅万华镜 mk 模式) -->
        <div v-if="chr.simulator?.kind === 'mk'" class="m-sim-item">
          <div class="m-sim-item__info">
            <span class="input-field__label">PlayerCHR (玩家)</span>
            <span v-if="chr.player" class="m-sim-item__val">{{ chr.player.name }}</span>
            <span v-else class="m-sim-item__muted">未加载</span>
          </div>
          <Button variant="secondary" size="sm" @click="loadPlayerCHR">
            {{ chr.player ? '替换' : '加载' }}
          </Button>
        </div>

        <pre v-if="simError" class="m-sim-dialog__error">{{ simError }}</pre>
      </div>
      <template #footer>
        <Button block @click="simDialogOpen = false">完成</Button>
      </template>
    </Dialog>

    <!-- 示例下载弹窗 -->
    <Dialog v-model="examplesDialogOpen" title="示例下载">
      <ExampleBrowserMobile v-if="examplesDialogOpen" />
      <template #footer>
        <Button block @click="examplesDialogOpen = false">关闭</Button>
      </template>
    </Dialog>
  </div>
</template>

<style scoped>
.m-app {
  display: flex;
  flex-direction: column;
  height: 100dvh;
  width: 100vw;
  overflow: hidden;
  background: var(--canvas);
  color: var(--ink);
}

/* 顶部状态栏 */
.m-topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  padding: 0.4em 0.6em;
  border-block-end: 1px solid var(--line);
  background: var(--surface);
  flex-shrink: 0;
  padding-top: max(0.4em, env(safe-area-inset-top));
}

.m-topbar__left {
  display: flex;
  align-items: center;
  gap: 0.5em;
  min-width: 0;
}

.m-topbar__mode {
  padding: 2px 6px;
  font-weight: 700;
  font-size: 0.82em;
  color: var(--muted);
  border: 1px solid var(--line);
  background: var(--surface-strong);
  white-space: nowrap;
}

.m-topbar__mode.is-loaded {
  border-color: var(--accent);
  color: var(--accent-ink);
  background: var(--accent);
}

.m-topbar__ready {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--muted);
  font-size: 0.78em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.m-topbar__ready-dot {
  inline-size: 6px;
  block-size: 6px;
  background: var(--focus);
}

.m-topbar__ready.is-ok .m-topbar__ready-dot {
  background: var(--accent);
}

.m-topbar__ready.is-ok {
  color: var(--accent);
}

.m-topbar__right {
  display: flex;
  align-items: center;
  gap: 0.4em;
  flex-shrink: 0;
}

/* 顶部通知 */
.m-notice {
  padding: 0.3em 0.6em;
  background: rgb(174 58 45 / 12%);
  border-block-end: 1px solid var(--danger);
  color: var(--danger);
  font-size: 0.82em;
  flex-shrink: 0;
}

/* 视图导航 */
.m-nav {
  display: flex;
  border-block-end: 1px solid var(--line);
  background: var(--surface);
  flex-shrink: 0;
}

.m-nav :deep(.toggle-button-group) {
  width: 100%;
  border: 0;
}

.m-nav :deep(.toggle-button) {
  flex: 1;
  text-align: center;
  font-size: 0.85em;
  padding: 0.45em 0.2em;
  border-inline-end: 1px solid var(--line);
}

.m-nav :deep(.toggle-button:last-child) {
  border-inline-end: 0;
}

/* 主区域容器 */
.m-main {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* 对话视图 */
.m-chat-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.m-chat__scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0.6em 0.5em;
  display: flex;
  flex-direction: column;
  gap: 0.6em;
  -webkit-overflow-scrolling: touch;
}

.m-chat__empty {
  margin: auto;
  padding: 1.5em 1em;
  text-align: center;
  color: var(--muted);
}

.m-chat__empty-title {
  margin: 0 0 0.4em;
  font-weight: 700;
  color: var(--ink);
  font-size: 0.95em;
}

.m-chat__empty-desc {
  margin: 0;
  font-size: 0.82em;
  line-height: 1.4;
}

/* 底部 Composer */
.m-composer {
  border-block-start: 1px solid var(--line);
  background: var(--surface);
  padding: 0.5em 0.6em;
  padding-bottom: max(0.5em, env(safe-area-inset-bottom));
  flex-shrink: 0;
}

.m-composer__textarea {
  min-block-size: 3.6em;
  max-block-size: 8em;
  font-size: 0.95rem;
  line-height: 1.35;
  resize: none;
}

.m-composer__action-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  margin-block-start: 0.4em;
}

.m-composer__btns {
  display: flex;
  align-items: center;
  gap: 0.4em;
}

/* 问答模式 */
.m-composer__question {
  display: flex;
  flex-direction: column;
  gap: 0.45em;
  border-block-start: 2px solid var(--focus);
  padding-block-start: 0.2em;
}

.m-composer__question-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.4em;
}

.m-composer__question-tag {
  font-size: 0.78em;
  font-weight: 700;
  color: var(--focus);
}

.m-composer__prompt {
  margin: 0;
  font-family: inherit;
  font-size: 0.88em;
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--surface-strong);
  padding: 0.4em 0.5em;
  border-inline-start: 2px solid var(--focus);
}

.m-composer__options {
  display: flex;
  flex-direction: column;
  gap: 0.35em;
  max-height: 9em;
  overflow-y: auto;
}

.m-composer__option-card {
  display: flex;
  align-items: center;
  gap: 0.5em;
  padding: 0.4em 0.5em;
  background: var(--surface-strong);
  border: 1px solid var(--line);
  font-size: 0.86em;
  cursor: pointer;
}

.m-composer__option-card span {
  flex: 1;
  word-break: break-word;
}

/* 状态指示点 */
.m-stage {
  display: inline-flex;
  align-items: center;
  gap: 0.4em;
  color: var(--muted);
  font-size: 0.78em;
  white-space: nowrap;
}

.m-stage__dot {
  inline-size: 6px;
  block-size: 6px;
  background: var(--line-strong);
}

.m-stage.is-busy {
  color: var(--accent);
}

.m-stage.is-busy .m-stage__dot {
  background: var(--accent);
  animation: blink 600ms steps(2, start) infinite;
}

@keyframes blink {
  to { visibility: hidden; }
}

/* 状态栏 & 记忆全屏面板 */
.m-panel-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--surface);
}

.m-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  padding: 0.5em 0.6em;
  border-block-end: 1px solid var(--line);
  background: var(--surface-strong);
  flex-shrink: 0;
}

.m-panel__title {
  display: flex;
  align-items: baseline;
  gap: 0.5em;
  font-weight: 700;
  font-size: 0.9em;
}

.m-panel__subtitle {
  font-size: 0.75em;
  color: var(--muted);
  font-weight: normal;
}

.m-panel__body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.6em;
  -webkit-overflow-scrolling: touch;
}

.m-panel__empty {
  text-align: center;
  padding: 2em 1em;
  color: var(--muted);
  font-size: 0.9em;
}

.m-panel__empty-desc {
  font-size: 0.8em;
  margin-top: 0.5em;
  line-height: 1.4;
}

/* 重生成弹窗复选框 */
.m-regen-checkbox {
  display: flex;
  align-items: center;
  gap: 0.5em;
  margin-block-start: 0.6em;
  font-size: 0.9em;
  cursor: pointer;
}

/* 抽屉菜单 */
.m-menu {
  display: flex;
  flex-direction: column;
  gap: 0.8em;
  padding-block: 0.2em;
}

.m-menu__info-card {
  display: flex;
  flex-direction: column;
  gap: 0.35em;
  padding: 0.55em;
  background: var(--surface-strong);
  border: 1px solid var(--line);
}

.m-menu__info-item {
  display: flex;
  justify-content: space-between;
  gap: 0.5em;
  font-size: 0.82em;
}

.m-menu__info-label {
  color: var(--muted);
}

.m-menu__info-val {
  font-weight: 700;
  text-align: end;
  word-break: break-all;
}

.m-menu__info-val.is-ok {
  color: var(--accent);
}

.m-menu__section {
  display: flex;
  flex-direction: column;
  gap: 0.4em;
}

.m-menu__section-title {
  font-size: 0.78em;
  font-weight: 700;
  color: var(--muted);
  border-block-end: 1px solid var(--line);
  padding-block-end: 2px;
}

.m-menu__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.4em;
}

.m-menu__grid3 {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.4em;
}

/* API 对话框 */
.m-api-dialog {
  display: flex;
  flex-direction: column;
  gap: 0.6em;
}

.m-api-dialog__top-actions {
  display: flex;
  gap: 0.5em;
}

.m-api-dialog__status {
  margin: 0;
  color: var(--accent);
  font-size: 0.82em;
}

.m-api-dialog__error {
  margin: 0;
  color: var(--danger);
  font-size: 0.82em;
}

.m-api-dialog__globals {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.4em;
}

.m-api-dialog__tabs {
  margin-block-start: 0.2em;
}

/* 模拟配置对话框 */
.m-sim-dialog {
  display: flex;
  flex-direction: column;
  gap: 0.6em;
}

.m-sim-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  padding-block: 0.2em;
}

.m-sim-item__info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.m-sim-item__val {
  font-size: 0.85em;
  color: var(--ink);
  word-break: break-all;
}

.m-sim-item__muted {
  font-size: 0.82em;
  color: var(--muted);
}

.m-sim-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 0.3em;
}

.m-sim-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  padding: 0.3em 0.5em;
  background: var(--surface-strong);
  border-inline-start: 2px solid var(--accent);
}

.m-sim-list__desc {
  display: flex;
  align-items: baseline;
  gap: 0.4em;
  min-width: 0;
  overflow: hidden;
}

.m-sim-list__name {
  font-size: 0.85em;
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
}

.m-sim-list__tag {
  font-size: 0.75em;
  color: var(--muted);
}

.m-sim-list__remove {
  border: 0;
  padding: 2px 6px;
  background: transparent;
  color: var(--muted);
  font-size: 0.9em;
  line-height: 1;
}

.m-sim-list__remove:active {
  color: var(--danger);
}

.m-sim-dialog__error {
  margin: 0;
  padding: 0.4em;
  background: rgb(174 58 45 / 8%);
  border-inline-start: 2px solid var(--danger);
  color: var(--danger);
  font-size: 0.82em;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
