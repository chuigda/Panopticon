<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { Config } from '../../config'
import type { SimulatorCHR, PlayerCHR } from '../../session/chr'
import type { ChatMessage } from '../../session/message'
import { pipelineGenerate, regenerateStatusBar, manualCompressMemory } from '../../pipeline/pipeline'
import { createAskQuestionTool } from '../../pipeline/tool'
import drillConfig from '../../../drill/config-drill.json' with { type: 'json' }

const baseUrl = ref(drillConfig.baseUrl)
const apiKey = ref(drillConfig.apiKey)
const chatModelName = ref('claude-fable-5-1')
const auxModelName = ref('claude-sonnet-4-6')
const mode = ref<'cd' | 'mk'>('cd')
const discardToolCalls = ref(false)

const config = computed<Config>(() => ({
  inlineMessageLimit: 3,
  compressionSize: 2,
  outputLength: 300,
  directConnect: false,
  chatModel: {
    uri: baseUrl.value,
    apiKey: apiKey.value,
    protocol: 'messages',
    modelName: chatModelName.value,
    outputBudget: 4096,
    thinkingEnabled: true,
    reasoningEffort: 'low',
    cacheTtl: '5m',
    systemCacheTtl: '5m',
  },
  statusBarModel: {
    uri: baseUrl.value,
    apiKey: apiKey.value,
    protocol: 'chat-completions',
    modelName: auxModelName.value,
    outputBudget: 2048,
    thinkingEnabled: false,
    temperature: 0.05,
  },
  memoryModel: {
    uri: baseUrl.value,
    apiKey: apiKey.value,
    protocol: 'chat-completions',
    modelName: auxModelName.value,
    outputBudget: 2048,
    thinkingEnabled: false,
    temperature: 0.05,
  },
}))

const simulatorCHR: SimulatorCHR = {
  kind: 'cd',
  universeName: '雾港',
  literalWorkName: '《雾港纪事》',
  language: 'zh_CN',
  statusBar: {
    format: [
      'LOCATION: {{location}}',
      'WEATHER: {{weather}}{% if fog %}，雾{% endif %}',
      'LIGHTHOUSE: {% if lighthouse_lit %}亮{% else %}暗{% endif %}',
      'CHARACTERS:',
      '{% for c in characters %}',
      '- {{c.name}}（{{c.mood}}）：{{c.action}}',
      '{% endfor %}',
      'TENSION: {{tension}}  <!-- 低/中/高 -->',
    ].join('\n'),
    rule: [
      '1. LIGHTHOUSE 仅在模拟器输出明确描述塔顶灯亮起时改为"亮"，否则保持不变。',
      '2. CHARACTERS 只列出当前场景在场角色；离场者移除，新登场者追加。',
      '3. TENSION 随冲突升级或缓和调整，不得跳级超过一档。',
    ].join('\n'),
    example: [
      'LOCATION: 雾港·旧灯塔',
      'WEATHER: 细雨，雾',
      'LIGHTHOUSE: 暗',
      'CHARACTERS:',
      '- 林砂（警惕）：右手持灯，贴在门框阴影中',
      '- 老渡（沉默）：坐在提灯旁抽烟斗',
      'TENSION: 中',
    ].join('\n'),
  },
  simulator: {
    world: [
      '雾港是一座常年被海雾笼罩的港口小城，时值 1897 年深秋。',
      '城中流传着"雾中灯"的传说：每逢浓雾之夜，旧灯塔会自行亮起，指引某艘不存在的船。',
      '市政厅、码头帮会与灯塔守夜人三方关系微妙。',
    ].join('\n'),
    characters: [
      '林砂：二十六岁，市政厅档案员，谨慎、好奇心极强，正在私下调查灯塔失踪案。',
      '老渡：年过六十的灯塔守夜人，话少，知道很多不肯说的事。',
      '赫尔曼：码头帮会头目，表面豪爽，实则精于算计。',
    ].join('\n'),
  },
}

const playerCHR: PlayerCHR = {
  name: '阿澈',
  data: '二十二岁的外来水手，三天前随货船抵达雾港，因船只滞留而被迫留下。性格直率，胆大，对本地传说一无所知。',
}

const zeroUsage = () => ({ inputTokens: 0, cacheInputTokens: 0, cacheCreateTokens: 0, outputTokens: 0 })

// 预置 3 条 assistant，下一次生成即触发 inlineMessageLimit=3 的记忆压缩
const seedMessages: ChatMessage[] = [
  {
    $k: 'memory',
    content: '1897AD\n- 11.03. 林砂在市政厅档案室发现三年内有七名灯塔相关人员失踪，记录被人为抹去。她决定亲自夜探旧灯塔。',
    tokenUsage: zeroUsage(),
  },
  { $k: 'user', content: '开场：浓雾之夜，林砂偷偷来到旧灯塔门口，发现门没锁。' },
  {
    $k: 'assistant',
    content: '雾浓得像牛乳。林砂踏上礁石间的小径，铁门在她指尖下无声退开半寸——没锁。门内地面有一道新鲜的湿脚印，正朝螺旋阶梯延伸。她把身体贴进门框阴影里，屏住了呼吸。',
    statusBar: 'TIME: 1897-11-03 Wednesday\n\nLOCATION: 雾港·旧灯塔门口\nWEATHER: 阴，雾\nLIGHTHOUSE: 暗\nCHARACTERS:\n- 林砂（警惕）：贴在门框阴影中，屏住呼吸\nTENSION: 中',
    tokenUsage: zeroUsage(),
    statusBarTokenUsage: zeroUsage(),
  },
  { $k: 'user', content: '林砂顺着脚印上楼，在二层平台撞见老渡。' },
  {
    $k: 'assistant',
    content: '螺旋阶梯冰冷潮湿。二层平台上，一盏被布罩住的提灯透出微光，老渡坐在灯旁抽烟斗，像早已等在那里。“档案员小姐，”他没有抬头，“市政厅的钥匙，三年前就不在港务处了。”',
    statusBar: 'TIME: 1897-11-03 Wednesday\n\nLOCATION: 雾港·旧灯塔二层平台\nWEATHER: 阴，雾\nLIGHTHOUSE: 暗\nCHARACTERS:\n- 林砂（震惊）：手扶栏杆\n- 老渡（平静）：坐在提灯旁抽烟斗\nTENSION: 高',
    tokenUsage: zeroUsage(),
    statusBarTokenUsage: zeroUsage(),
  },
  { $k: 'user', content: '林砂追问失踪者的去向，老渡沉默片刻后指向塔顶。' },
  {
    $k: 'assistant',
    content: '老渡吐出一口烟，烟在雾气里几乎分辨不出。他用烟斗柄朝头顶点了点，那里是漆黑的塔顶。“他们都上去过。”他说，“下来的，只有我一个。”话音未落，塔顶忽然传来一声金属摩擦的轻响。',
    statusBar: 'TIME: 1897-11-03 Wednesday\n\nLOCATION: 雾港·旧灯塔二层平台\nWEATHER: 阴，起风，雾\nLIGHTHOUSE: 暗\nCHARACTERS:\n- 林砂（紧张）：望向塔顶\n- 老渡（沉重）：用烟斗柄指向塔顶\nTENSION: 高',
    tokenUsage: zeroUsage(),
    statusBarTokenUsage: zeroUsage(),
  },
]

const chatMessages = reactive<ChatMessage[]>(structuredClone(seedMessages))
const input = ref('')
const busy = ref<'' | 'generate' | 'status' | 'memory'>('')
let controller: AbortController | undefined

// window.prompt 在部分嵌入式浏览器中不可用，改用页内面板
const pendingQuestion = ref<{ prompt: string; options?: string[] } | null>(null)
const questionAnswer = ref('')
let resolveQuestion: ((answer: string) => void) | undefined

function answerQuestion(answer: string) {
  resolveQuestion?.(answer)
  resolveQuestion = undefined
  pendingQuestion.value = null
  questionAnswer.value = ''
}

const tools = [
  createAskQuestionTool(({ prompt, options }) => new Promise<string>((resolve) => {
    resolveQuestion = resolve
    pendingQuestion.value = { prompt, options }
  })),
]

const canSend = computed(() => {
  const last = chatMessages[chatMessages.length - 1]
  return !busy.value && (input.value.trim() || (last && last.$k !== 'assistant'))
})

async function run<T>(kind: typeof busy.value, fn: (signal: AbortSignal) => Promise<T>) {
  if (busy.value) return
  busy.value = kind
  controller = new AbortController()
  try {
    await fn(controller.signal)
  } finally {
    busy.value = ''
    controller = undefined
  }
}

function send() {
  const text = input.value.trim()
  if (text) {
    chatMessages.push({ $k: 'user', content: text })
    input.value = ''
  }
  run('generate', (signal) =>
    pipelineGenerate(
      config.value,
      tools,
      simulatorCHR,
      [],
      mode.value === 'mk' ? playerCHR : undefined,
      chatMessages,
      discardToolCalls.value,
      () => {},
      signal,
    ),
  )
}

function regenStatus() {
  run('status', (signal) =>
    regenerateStatusBar(config.value, simulatorCHR, [], mode.value === 'mk' ? playerCHR : undefined, chatMessages, () => {}, signal),
  )
}

function compress() {
  run('memory', (signal) =>
    manualCompressMemory(config.value, simulatorCHR, [], mode.value === 'mk' ? playerCHR : undefined, chatMessages, () => {}, signal),
  )
}

function abort() {
  if (pendingQuestion.value) answerQuestion('(aborted)')
  controller?.abort()
}

function clear() {
  chatMessages.splice(0, chatMessages.length)
}

function resetSeed() {
  chatMessages.splice(0, chatMessages.length, ...structuredClone(seedMessages))
}

function removeLast() {
  chatMessages.pop()
}

function fmtUsage(u: { inputTokens: number; cacheInputTokens: number; cacheCreateTokens: number; outputTokens: number }) {
  return `in ${u.inputTokens} / cache ${u.cacheInputTokens} / create ${u.cacheCreateTokens} / out ${u.outputTokens}`
}

const roleLabel: Record<ChatMessage['$k'], string> = {
  user: '导演部 / 玩家',
  assistant: '模拟器',
  tool_call: '工具调用',
  tool_call_result: '工具结果',
  memory: '记忆压缩',
  error: '错误',
}
</script>

<template>
  <main class="pd">
    <h1>Pipeline Demo</h1>

    <section class="pd-card pd-config">
      <label>Base URL <input v-model="baseUrl" /></label>
      <label>API Key <input v-model="apiKey" type="password" /></label>
      <label>Chat 模型 (messages) <input v-model="chatModelName" /></label>
      <label>StatusBar / Memory 模型 (chat-completions) <input v-model="auxModelName" /></label>
      <label>
        模式
        <select v-model="mode" :disabled="chatMessages.length > 0">
          <option value="cd">cd（导演部）</option>
          <option value="mk">mk（玩家：{{ playerCHR.name }}）</option>
        </select>
      </label>
      <label class="pd-inline"><input v-model="discardToolCalls" type="checkbox" /> discardToolCalls</label>
      <span class="pd-meta">
        inlineMessageLimit={{ config.inlineMessageLimit }} · compressionSize={{ config.compressionSize }} · outputLength={{ config.outputLength }}
      </span>
    </section>

    <section class="pd-card pd-log">
      <div v-if="!chatMessages.length" class="pd-empty">还没有消息</div>
      <div v-for="(m, i) in chatMessages" :key="i" class="pd-msg" :class="`pd-msg--${m.$k}`">
        <div class="pd-msg__head">#{{ i }} {{ roleLabel[m.$k] }}</div>

        <template v-if="m.$k === 'user'">
          <pre>{{ m.content }}</pre>
        </template>

        <template v-else-if="m.$k === 'assistant'">
          <details v-if="m.reasoning">
            <summary>reasoning</summary>
            <pre>{{ m.reasoning }}</pre>
          </details>
          <pre>{{ m.content || '…' }}</pre>
          <pre class="pd-status">{{ m.statusBar || '(status bar 尚未生成)' }}</pre>
          <div class="pd-meta">chat: {{ fmtUsage(m.tokenUsage) }}</div>
          <div class="pd-meta">status: {{ fmtUsage(m.statusBarTokenUsage) }}</div>
        </template>

        <template v-else-if="m.$k === 'tool_call'">
          <pre v-if="m.content">{{ m.content }}</pre>
          <pre v-for="c in m.calls" :key="c.id">{{ c.name }}({{ JSON.stringify(c.arguments) }})  [{{ c.id }}]</pre>
          <div class="pd-meta">{{ fmtUsage(m.tokenUsage) }}</div>
        </template>

        <template v-else-if="m.$k === 'tool_call_result'">
          <pre v-for="r in m.results" :key="r.id" :class="{ 'pd-err': r.isError }">[{{ r.id }}] {{ JSON.stringify(r.content) }}</pre>
        </template>

        <template v-else-if="m.$k === 'memory'">
          <pre>{{ m.content }}</pre>
          <div class="pd-meta">{{ fmtUsage(m.tokenUsage) }}</div>
        </template>

        <template v-else>
          <pre class="pd-err">{{ m.error }}</pre>
        </template>
      </div>
    </section>

    <section v-if="pendingQuestion" class="pd-card pd-question">
      <div class="pd-msg__head">模拟器提问</div>
      <pre>{{ pendingQuestion.prompt }}</pre>
      <div class="pd-actions">
        <button v-for="o in pendingQuestion.options" :key="o" @click="answerQuestion(o)">{{ o }}</button>
      </div>
      <div class="pd-actions">
        <input v-model="questionAnswer" placeholder="自由回答" @keydown.enter.prevent="questionAnswer.trim() && answerQuestion(questionAnswer.trim())" />
        <button :disabled="!questionAnswer.trim()" @click="answerQuestion(questionAnswer.trim())">回答</button>
      </div>
    </section>

    <section class="pd-card pd-input">
      <textarea
        v-model="input"
        rows="3"
        :placeholder="mode === 'cd' ? '导演部指令，Enter 发送' : '玩家行动，Enter 发送'"
        @keydown.enter.exact.prevent="canSend && send()"
      ></textarea>
      <div class="pd-actions">
        <button :disabled="!canSend" @click="send">{{ busy === 'generate' ? '生成中…' : '发送 / 生成' }}</button>
        <button :disabled="!!busy || chatMessages[chatMessages.length - 1]?.$k !== 'assistant'" @click="regenStatus">
          {{ busy === 'status' ? '生成中…' : '重生成 StatusBar' }}
        </button>
        <button :disabled="!!busy || !chatMessages.length" @click="compress">
          {{ busy === 'memory' ? '压缩中…' : '手动压缩记忆' }}
        </button>
        <button :disabled="!busy" @click="abort">中止</button>
        <button :disabled="!!busy || !chatMessages.length" @click="removeLast">删最后一条</button>
        <button :disabled="!!busy || !chatMessages.length" @click="clear">清空</button>
        <button :disabled="!!busy" @click="resetSeed">重置为种子数据</button>
      </div>
    </section>
  </main>
</template>

<style scoped>
.pd { max-width: 900px; margin: 0 auto; padding: 24px; font-family: system-ui, sans-serif; }
.pd h1 { font-size: 20px; margin: 0 0 16px; }
.pd-card { border: 1px solid #ddd; border-radius: 8px; padding: 14px; margin-bottom: 14px; }
.pd-config { display: flex; flex-wrap: wrap; gap: 10px 16px; font-size: 13px; }
.pd-config label { display: flex; align-items: center; gap: 6px; }
.pd-config input:not([type=checkbox]) { padding: 4px 6px; min-width: 200px; }
.pd-inline { gap: 4px; }
.pd-log { max-height: 520px; overflow-y: auto; }
.pd-empty { color: #999; font-size: 13px; }
.pd-msg { padding: 8px 10px; border-left: 3px solid #ccc; margin-bottom: 10px; font-size: 14px; }
.pd-msg__head { font-weight: 600; font-size: 12px; color: #666; margin-bottom: 4px; }
.pd-msg pre { white-space: pre-wrap; margin: 4px 0; font-family: inherit; }
.pd-msg--user { border-color: #2563eb; }
.pd-msg--assistant { border-color: #16a34a; }
.pd-msg--tool_call, .pd-msg--tool_call_result { border-color: #ca8a04; background: #fefce8; }
.pd-msg--memory { border-color: #7c3aed; background: #f5f3ff; }
.pd-msg--error { border-color: #dc2626; background: #fef2f2; }
.pd-status { font-family: ui-monospace, monospace !important; font-size: 12px; background: #f3f4f6; padding: 6px; border-radius: 4px; }
.pd-meta { font-size: 11px; color: #888; }
.pd-err { color: #b91c1c; }
.pd-input textarea { width: 100%; box-sizing: border-box; padding: 8px; resize: vertical; }
.pd-question { border-color: #ca8a04; background: #fefce8; }
.pd-question pre { white-space: pre-wrap; margin: 4px 0; font-family: inherit; }
.pd-question input { flex: 1; padding: 6px; }
.pd-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 8px; }
.pd-actions button { padding: 6px 14px; border: none; border-radius: 6px; background: #2563eb; color: #fff; cursor: pointer; }
.pd-actions button:disabled { background: #9ca3af; cursor: not-allowed; }
</style>
