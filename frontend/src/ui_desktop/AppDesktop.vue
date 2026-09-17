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
import ChatBubble from './ChatBubble.vue'
import Dialog from './Dialog.vue'
import ExampleBrowser from './ExampleBrowser.vue'
import Input from './Input.vue'
import MarkdownEdit from './MarkdownEdit.vue'
import ModelConfigForm from './ModelConfigForm.vue'
import PlainTextEdit from './PlainTextEdit.vue'
import ToggleButton from './ToggleButton.vue'
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

// ---------- 会话 ----------
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

// ---------- 流程 ----------
const notice = ref('')
function warn(msg: string) {
  notice.value = msg
  setTimeout(() => (notice.value === msg ? (notice.value = '') : undefined), 4000)
}

function playerArg() {
  return mode.value === 'mk' ? chr.player : undefined
}

const readiness = computed<{ ok: boolean; reason: string }>(() => {
  if (!chr.simulator) return { ok: false, reason: '未加载 SimulatorCHR' }
  if (chr.simulator.kind === 'mk' && !chr.player) return { ok: false, reason: '未加载 PlayerCHR' }
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
  run((signal) =>
    regenerateStatusBar(config, chr.simulator!, chr.additional, playerArg(), chatMessages, (s) => (stage.value = s), signal),
  )
}

function compress() {
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

// ---------- 文件 ----------
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
  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  download(`session-${chr.simulator?.universeName ?? 'untitled'}-${stamp}.json`, JSON.stringify(chatMessages, null, 2))
}

async function loadSession() {
  if (busy.value) return
  const [file] = await pickFile('application/json,.json')
  if (!file) return
  try {
    const data = JSON.parse(await file.text())
    if (!Array.isArray(data) || !data.every((m) => typeof m?.$k === 'string')) throw new Error('不是有效的会话文件')
    chatMessages.splice(0, chatMessages.length, ...(data as ChatMessage[]))
  } catch (e) {
    warn(`读取会话失败：${e instanceof Error ? e.message : e}`)
  }
}

// ---------- API 配置 ----------
const apiConfigStatus = ref('')
const apiConfigError = ref('')

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
    apiConfigError.value = '导入失败：配置文件格式无效或字段不完整。'
  }
}

// ---------- 模拟配置 ----------
const simDialogOpen = ref(false)
const simError = ref('')

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
    simError.value = droppedCount ? `已移除 ${droppedCount} 个类型不匹配的 AdditionalCHR` : ''
  } catch (e) {
    simError.value = `SimulatorCHR 解析失败：${e instanceof Error ? e.message : e}`
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
        errors.push(`${file.name}: 类型 ${parsed.kind} 与 SimulatorCHR (${chr.simulator.kind}) 不一致`)
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
    simError.value = `PlayerCHR 解析失败：${e instanceof Error ? e.message : e}`
  }
}

// ---------- 示例下载 ----------
const examplesDialogOpen = ref(false)

// ---------- API 配置 ----------
const apiDialogOpen = ref(false)
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

// ---------- 布局 ----------
const splitRatio = ref(0.62)
const layoutEl = ref<HTMLElement | null>(null)
const statusEl = ref<HTMLElement | null>(null)
const statusSplitRatio = ref(0.75)

function startDrag(e: PointerEvent) {
  const el = layoutEl.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  const move = (ev: PointerEvent) => {
    splitRatio.value = Math.min(0.85, Math.max(0.3, (ev.clientX - rect.left) / rect.width))
  }
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
  e.preventDefault()
}

function startStatusDrag(e: PointerEvent) {
  const el = statusEl.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  const move = (ev: PointerEvent) => {
    statusSplitRatio.value = Math.min(1, Math.max(0, (ev.clientY - rect.top) / rect.height))
  }
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
  e.preventDefault()
}

const scrollEl = ref<HTMLElement | null>(null)
const inputEl = ref<HTMLTextAreaElement | null>(null)
let stickToBottom = true

function onScroll() {
  const el = scrollEl.value
  if (!el) return
  stickToBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40
}

watch(
  chatMessages,
  async () => {
    if (!stickToBottom) return
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

// 状态栏显示时隐藏 TIME 行，并把单换行转成 Markdown 硬换行
function preprocessStatusBar(text: string) {
  return text
    .split('\n')
    .filter((line) => !/^\s*TIME:/.test(line))
    .join('  \n')
    .replace(/^(\s*\n)+/, '')
}
</script>

<template>
  <div class="app">
    <header class="toolbar">
      <span class="toolbar__mode" :class="{ 'is-loaded': mode }">{{ modeLabel }}</span>
      <span class="toolbar__sep" />
      <Button variant="ghost" size="sm" :disabled="busy" @click="loadSession">读取会话</Button>
      <Button variant="ghost" size="sm" :disabled="!chatMessages.length" @click="saveSession">保存会话</Button>
      <span class="toolbar__sep" />
      <Button variant="ghost" size="sm" :disabled="busy" @click="apiDialogOpen = true">API 配置</Button>
      <Button variant="ghost" size="sm" :disabled="busy" @click="simDialogOpen = true">模拟配置</Button>
      <span class="toolbar__sep" />
      <Button variant="ghost" size="sm" @click="examplesDialogOpen = true">示例下载</Button>
      <span v-if="notice" class="toolbar__notice">{{ notice }}</span>
      <span class="toolbar__ready" :class="{ 'is-ok': readiness.ok }">{{ readiness.reason }}</span>
    </header>

    <main ref="layoutEl" class="layout" :style="{ gridTemplateColumns: `${splitRatio * 100}% 4px 1fr` }">
      <section class="chat">
        <div ref="scrollEl" class="chat__scroll" @scroll="onScroll">
          <p v-if="!chatMessages.length" class="chat__empty">
            {{ chr.simulator ? `${chr.simulator.literalWorkName} · ${chr.simulator.universeName}` : '尚未加载 SimulatorCHR' }}
          </p>
          <ChatBubble
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

        <div v-if="activeQuestion" class="composer composer--question">
          <div v-if="questions.length > 1" class="composer__tabs">
            <ToggleButtonGroup
              :model-value="String(activeQuestion.id)"
              :options="questionTabs"
              aria-label="待答问题"
              @update:model-value="switchQuestion"
            />
            <span class="composer__count">{{ questions.length }} 个待答</span>
          </div>
          <pre class="composer__prompt">{{ activeQuestion.prompt }}</pre>
          <div
            v-if="activeOptionItems.length"
            class="composer__options"
            role="group"
            aria-label="候选答案"
          >
            <label v-for="option in activeOptionItems" :key="option.value" class="composer__option">
              <input v-model="activeQuestion.selected" type="checkbox" :value="option.value" />
              <span>{{ option.label }}</span>
            </label>
          </div>
          <textarea
            ref="inputEl"
            v-model="activeQuestion.text"
            class="composer__input"
            rows="3"
            :placeholder="activeOptionItems.length ? '可补充自由回答，Ctrl+Enter 提交' : '自由回答，Ctrl+Enter 提交'"
            @keydown="onQuestionKeydown"
          />
          <div class="composer__bar">
            <Button size="sm" :disabled="!activeAnswer" @click="submitAnswer">回答</Button>
            <Button variant="danger" size="sm" @click="abort">中止</Button>
            <span class="stage is-busy"><span class="stage__dot" />{{ stageLabel[stage] }}</span>
          </div>
        </div>

        <div v-else class="composer">
          <textarea
            ref="inputEl"
            v-model="input"
            class="composer__input"
            rows="4"
            :placeholder="mode === 'mk' ? '玩家行动，Ctrl+Enter 发送' : '导演部指令，Ctrl+Enter 发送'"
            :disabled="busy"
            @keydown="onInputKeydown"
          />
          <div class="composer__bar">
            <Button size="sm" :disabled="!canSend" @click="onPrimary">{{ canRegenerate ? '重新生成' : '发送' }}</Button>
            <Button variant="ghost" size="sm" :disabled="busy || lastMessage?.$k !== 'assistant'" @click="regenStatus">重生成状态栏</Button>
            <Button variant="ghost" size="sm" :disabled="busy || !chatMessages.length" @click="compress">压缩记忆</Button>
            <Button variant="danger" size="sm" :disabled="!busy" @click="abort">中止</Button>
            <span class="stage" :class="{ 'is-busy': busy }">
              <span class="stage__dot" />{{ stageLabel[stage] }}
            </span>
          </div>
        </div>
      </section>

      <div class="splitter" @pointerdown="startDrag" />

      <aside
        ref="statusEl"
        class="status"
        :style="lastMemory ? { gridTemplateRows: `minmax(0, ${statusSplitRatio}fr) 4px minmax(0, ${1 - statusSplitRatio}fr)` } : undefined"
      >
        <section class="status__panel">
          <div class="status__head">状态栏</div>
          <div v-if="lastAssistant" class="status__body">
            <MarkdownEdit
              :model-value="lastAssistant.statusBar"
              :preprocess="preprocessStatusBar"
              @update:model-value="updateStatusBar"
            />
          </div>
          <p v-else class="status__empty">尚无状态</p>
        </section>

        <div v-if="lastMemory" class="status__splitter" @pointerdown="startStatusDrag" />

        <section v-if="lastMemory" class="memory">
          <div class="memory__head">记忆</div>
          <div class="memory__body">
            <PlainTextEdit
              :model-value="lastMemory.content"
              @update:model-value="updateMemory"
            />
          </div>
        </section>
      </aside>
    </main>

    <Dialog v-model="regenDialogOpen" title="重新生成">
      <p>将丢弃最后一条模拟器输出并重新生成。</p>
      <label class="regen-dialog__discard-tools">
        <input v-model="shouldDiscardToolCalls" type="checkbox" />
        丢弃工具调用
      </label>
      <template #footer>
        <Button variant="ghost" @click="regenDialogOpen = false">取消</Button>
        <Button @click="send(shouldDiscardToolCalls)">重新生成</Button>
      </template>
    </Dialog>

    <Dialog v-model="apiDialogOpen" title="API 配置" class="dialog--wide">
      <div class="api">
        <div class="api__actions">
          <Button variant="secondary" size="sm" @click="importApiConfig">导入配置</Button>
          <Button variant="secondary" size="sm" @click="exportApiConfig">导出配置</Button>
          <span v-if="apiConfigStatus" class="api__status" role="status">{{ apiConfigStatus }}</span>
        </div>
        <p v-if="apiConfigError" class="api__error" role="alert">{{ apiConfigError }}</p>
        <div class="api__globals">
          <Input :model-value="String(config.inlineMessageLimit)" label="内联消息上限" type="number" @update:model-value="setNum('inlineMessageLimit', $event)" />
          <Input :model-value="String(config.compressionSize)" label="压缩条数" type="number" @update:model-value="setNum('compressionSize', $event)" />
          <Input :model-value="String(config.outputLength)" label="输出长度" type="number" @update:model-value="setNum('outputLength', $event)" />
        </div>
        <div class="api__globals">
          <span class="input-field__label">直连模式</span>
          <ToggleButton v-model="config.directConnect">{{ config.directConnect ? '启用' : '关闭' }}</ToggleButton>
        </div>
        <ToggleButtonGroup :model-value="apiTab" :options="apiTabs" @update:model-value="apiTab = $event as ModelKey" />
        <ModelConfigForm v-model="config[apiTab]" />
      </div>
      <template #footer>
        <Button @click="apiDialogOpen = false">完成</Button>
      </template>
    </Dialog>

    <Dialog v-model="simDialogOpen" title="模拟配置" @close="simError = ''">
      <div class="sim">
        <div class="sim__row">
          <div class="sim__info">
            <span class="input-field__label">SimulatorCHR</span>
            <span v-if="chr.simulator">{{ chr.simulator.literalWorkName }} · {{ chr.simulator.universeName }} · {{ chr.simulator.kind }}</span>
            <span v-else class="sim__muted">未加载</span>
          </div>
          <Button variant="secondary" size="sm" @click="loadSimulatorCHR">{{ chr.simulator ? '替换' : '加载' }}</Button>
        </div>

        <div class="sim__row">
          <div class="sim__info">
            <span class="input-field__label">AdditionalCHR</span>
            <span v-if="!chr.additional.length" class="sim__muted">无</span>
          </div>
          <Button variant="secondary" size="sm" :disabled="!chr.simulator" @click="loadAdditionalCHR">添加</Button>
        </div>
        <ul v-if="chr.additional.length" class="sim__list">
          <li v-for="(a, i) in chr.additional" :key="a.id">
            <span>{{ a.name ?? a.id }} <span class="sim__muted">({{ a.kind }})</span></span>
            <button class="sim__remove" type="button" @click="chr.additional.splice(i, 1)">×</button>
          </li>
        </ul>

        <div v-if="chr.simulator?.kind === 'mk'" class="sim__row">
          <div class="sim__info">
            <span class="input-field__label">PlayerCHR</span>
            <span v-if="chr.player">{{ chr.player.name }}</span>
            <span v-else class="sim__muted">未加载</span>
          </div>
          <Button variant="secondary" size="sm" @click="loadPlayerCHR">{{ chr.player ? '替换' : '加载' }}</Button>
        </div>

        <pre v-if="simError" class="sim__error">{{ simError }}</pre>
      </div>
      <template #footer>
        <Button @click="simDialogOpen = false">完成</Button>
      </template>
    </Dialog>

    <Dialog v-model="examplesDialogOpen" title="示例下载">
      <ExampleBrowser v-if="examplesDialogOpen" />
      <template #footer>
        <Button @click="examplesDialogOpen = false">关闭</Button>
      </template>
    </Dialog>
  </div>
</template>

<style scoped>
.app {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  height: 100vh;
  overflow: hidden;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  border-block-end: 1px solid var(--line);
  padding: 4px 0.5em;
  background: var(--surface);
}

.toolbar__mode {
  min-width: 4em;
  padding: 2px 0.5em;
  color: var(--muted);
  font-weight: 700;
  font-size: 0.85em;
  border-inline-start: 2px solid var(--line);
}
.toolbar__mode.is-loaded {
  color: var(--accent);
  border-color: var(--accent);
}

.toolbar__sep {
  width: 1px;
  height: 1.4em;
  margin: 0 4px;
  background: var(--line);
}

.toolbar__notice {
  margin-inline-start: auto;
  color: var(--danger);
  font-size: 0.8em;
}

.toolbar__ready {
  margin-inline-start: auto;
  border-inline-end: 2px solid var(--focus);
  padding-inline-end: 0.5em;
  color: var(--muted);
  font-size: 0.8em;
  white-space: nowrap;
}
.toolbar__notice + .toolbar__ready { margin-inline-start: 0.5em; }
.toolbar__ready.is-ok {
  border-color: var(--accent);
  color: var(--accent);
}

.layout {
  display: grid;
  grid-template-rows: minmax(0, 1fr);
  min-height: 0;
  overflow: hidden;
}

.chat {
  display: grid;
  grid-template-rows: minmax(0, 1fr) auto;
  min-width: 0;
  min-height: 0;
}

.chat__scroll {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  align-content: start;
  gap: 0.5em;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 0.5em;
}

.chat__empty,
.status__empty {
  margin: 0;
  padding: 0.5em;
  color: var(--muted);
  font-size: 0.85em;
}

.composer {
  display: grid;
  gap: 4px;
  border-block-start: 1px solid var(--line);
  padding: 0.5em;
  background: var(--surface);
}
.composer--question {
  border-block-start-color: var(--focus);
}

.composer__prompt {
  margin: 0;
  white-space: pre-wrap;
  font: inherit;
  font-size: 0.9em;
}

.composer__tabs {
  display: flex;
  align-items: center;
  gap: 0.5em;
}

.composer__count {
  color: var(--muted);
  font-size: 0.78em;
}

.composer__options {
  display: grid;
  gap: 4px;
  justify-self: start;
  max-inline-size: 100%;
}

.composer__option {
  display: inline-flex;
  align-items: start;
  gap: 0.4em;
  color: var(--ink);
  cursor: pointer;
  line-height: 1.35;
}

.composer__option span {
  overflow-wrap: anywhere;
}

.composer__input {
  min-block-size: 3em;
  resize: vertical;
}

.composer__bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
}

.stage {
  display: inline-flex;
  align-items: center;
  gap: 0.4em;
  margin-inline-start: auto;
  color: var(--muted);
  font-size: 0.8em;
}

.stage__dot {
  width: 0.55em;
  height: 0.55em;
  background: var(--line-strong);
}
.stage.is-busy { color: var(--accent); }
.stage.is-busy .stage__dot {
  background: var(--accent);
  animation: blink 600ms steps(2, start) infinite;
}

@keyframes blink {
  to { visibility: hidden; }
}

.splitter {
  cursor: col-resize;
  background: var(--line);
  transition: background-color 80ms;
}
.splitter:hover { background: var(--accent); }

.status {
  display: grid;
  grid-template-rows: minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  background: var(--surface);
}

.status__panel,
.memory {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.status__head,
.memory__head {
  border-block-end: 1px solid var(--line);
  padding: 4px 0.5em;
  color: var(--muted);
  font-size: 0.78em;
  font-weight: 700;
}

.status__body,
.memory__body {
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 0.5em;
  font-size: 0.9em;
}

.status__splitter {
  cursor: row-resize;
  background: var(--line);
  touch-action: none;
  transition: background-color 80ms;
}
.status__splitter:hover { background: var(--accent); }

.api {
  display: grid;
  gap: 0.5em;
  color: var(--ink);
}

.api__actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5em;
}

.api__status {
  color: var(--accent);
  font-size: 0.85em;
}

.api__error {
  margin: 0;
  color: var(--danger);
  font-size: 0.85em;
}

.api > .toggle-button-group {
  justify-self: start;
}

.api__globals {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.5em;
}

.sim {
  display: grid;
  gap: 0.5em;
  color: var(--ink);
}

.sim__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
}

.sim__info {
  display: grid;
  gap: 2px;
  min-width: 0;
  font-size: 0.9em;
}

.sim__muted { color: var(--muted); }

.sim__list {
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: 0.9em;
}
.sim__list li {
  display: flex;
  justify-content: space-between;
  gap: 0.5em;
  border-inline-start: 2px solid var(--line);
  padding: 2px 0.5em;
}

.sim__remove {
  border: 0;
  padding: 0 0.3em;
  color: var(--muted);
  background: transparent;
}
.sim__remove:hover { color: var(--danger); }

.sim__error {
  margin: 0;
  white-space: pre-wrap;
  color: var(--danger);
  font: inherit;
  font-size: 0.85em;
}
</style>

<style>
.dialog--wide {
  inline-size: min(calc(100vw - 1em), 46rem);
}
</style>
