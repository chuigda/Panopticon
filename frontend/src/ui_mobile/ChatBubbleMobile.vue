<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import type { ChatMessage, ChatTokenUsage } from '../session/message.ts'
import MarkdownEditMobile from './MarkdownEditMobile.vue'
import PlainTextEditMobile from './PlainTextEditMobile.vue'

const props = defineProps<{
  message: ChatMessage
  index: number
  userLabel: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:content', value: string): void
  (e: 'remove'): void
}>()

const markdownEditRef = ref<InstanceType<typeof MarkdownEditMobile> | null>(null)
const plainTextEditRef = ref<InstanceType<typeof PlainTextEditMobile> | null>(null)

const roleLabel: Record<ChatMessage['$k'], string> = {
  user: '',
  assistant: '模拟器',
  tool_call: '工具调用',
  tool_call_result: '工具结果',
  memory: '记忆',
  error: '错误',
}

const label = computed(() => props.message.$k === 'user' ? props.userLabel : roleLabel[props.message.$k])

function fmtUsage(u: ChatTokenUsage) {
  return `in ${u.inputTokens} · out ${u.outputTokens}${u.cacheInputTokens ? ` · c-in ${u.cacheInputTokens}` : ''}`
}

const confirming = ref(false)
let confirmTimer: ReturnType<typeof setTimeout> | undefined

function onRemoveClick() {
  clearTimeout(confirmTimer)
  if (confirming.value) {
    confirming.value = false
    emit('remove')
    return
  }
  confirming.value = true
  confirmTimer = setTimeout(() => (confirming.value = false), 3000)
}

function triggerEdit() {
  if (props.message.$k === 'user') {
    plainTextEditRef.value?.startEditing()
  } else if (props.message.$k === 'assistant' || props.message.$k === 'memory') {
    markdownEditRef.value?.startEditing()
  }
}

const isEditable = computed(() => props.message.$k === 'user' || props.message.$k === 'assistant' || props.message.$k === 'memory')

onBeforeUnmount(() => clearTimeout(confirmTimer))
</script>

<template>
  <article class="bubble" :class="`bubble--${message.$k}`">
    <header class="bubble__head">
      <div class="bubble__meta-group">
        <span class="bubble__role">#{{ index }} {{ label }}</span>
        <span v-if="message.$k === 'assistant'" class="bubble__meta">
          {{ fmtUsage(message.tokenUsage) }}
        </span>
      </div>

      <div class="bubble__actions">
        <button
          v-if="isEditable && !disabled"
          class="bubble__action-btn"
          type="button"
          title="编辑此消息"
          @click="triggerEdit"
        >
          编辑
        </button>
        <button
          class="bubble__remove"
          :class="{ 'is-confirming': confirming }"
          type="button"
          :disabled="disabled"
          :title="confirming ? '再点一次确认删除' : '删除此消息'"
          @click="onRemoveClick"
        >
          {{ confirming ? '确定?' : '×' }}
        </button>
      </div>
    </header>

    <template v-if="message.$k === 'user'">
      <PlainTextEditMobile
        ref="plainTextEditRef"
        class="bubble__body"
        :model-value="message.content"
        :rows="4"
        @update:model-value="emit('update:content', $event)"
      />
    </template>

    <template v-else-if="message.$k === 'assistant'">
      <details v-if="message.reasoning" class="bubble__fold">
        <summary class="bubble__summary">思考过程</summary>
        <pre class="bubble__reasoning">{{ message.reasoning }}</pre>
      </details>
      <MarkdownEditMobile
        ref="markdownEditRef"
        class="bubble__body"
        :model-value="message.content"
        :rows="8"
        @update:model-value="emit('update:content', $event)"
      />
    </template>

    <template v-else-if="message.$k === 'tool_call'">
      <details class="bubble__fold">
        <summary class="bubble__summary">{{ message.calls.map((c) => c.name).join(', ') }}</summary>
        <pre v-if="message.content" class="bubble__pre">{{ message.content }}</pre>
        <pre v-for="c in message.calls" :key="c.id" class="bubble__pre">{{ c.name }}({{ JSON.stringify(c.arguments, null, 2) }})</pre>
      </details>
    </template>

    <template v-else-if="message.$k === 'tool_call_result'">
      <details class="bubble__fold">
        <summary class="bubble__summary">{{ message.results.length }} 个工具返回结果</summary>
        <pre
          v-for="r in message.results"
          :key="r.id"
          class="bubble__pre"
          :class="{ 'is-error': r.isError }"
        >{{ JSON.stringify(r.content, null, 2) }}</pre>
      </details>
    </template>

    <template v-else-if="message.$k === 'memory'">
      <details class="bubble__fold">
        <summary class="bubble__summary">记忆压缩 ({{ fmtUsage(message.tokenUsage) }})</summary>
        <MarkdownEditMobile
          ref="markdownEditRef"
          class="bubble__body"
          :model-value="message.content"
          :rows="8"
          @update:model-value="emit('update:content', $event)"
        />
      </details>
    </template>

    <template v-else>
      <details class="bubble__fold" open>
        <summary class="bubble__summary is-error">{{ message.error.split('\n')[0] }}</summary>
        <pre class="bubble__pre is-error">{{ message.error }}</pre>
      </details>
    </template>
  </article>
</template>

<style scoped>
.bubble {
  border-inline-start: 3px solid var(--line);
  padding: 4px 0.5em;
  background: var(--surface);
  font-size: 0.92em;
  display: flex;
  flex-direction: column;
  gap: 0.25em;
}

.bubble--user { border-color: var(--line-strong); }
.bubble--assistant { border-color: var(--accent); }
.bubble--tool_call, .bubble--tool_call_result { border-color: var(--focus); }
.bubble--memory { border-color: var(--muted); }
.bubble--error { border-color: var(--danger); background: rgb(174 58 45 / 5%); }

.bubble__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.4em;
  color: var(--muted);
  font-size: 0.8em;
  border-block-end: 1px dashed var(--line);
  padding-block-end: 2px;
}

.bubble__meta-group {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.4em;
  min-width: 0;
}

.bubble__role {
  font-weight: 700;
  color: var(--ink);
}

.bubble__meta {
  color: var(--muted);
  font-size: 0.85em;
}

.bubble__actions {
  display: inline-flex;
  align-items: center;
  gap: 0.3em;
  flex-shrink: 0;
}

.bubble__action-btn,
.bubble__remove {
  border: none;
  padding: 2px 6px;
  background: var(--surface);
  color: var(--ink);
  font-size: 12pt;
  line-height: 1.2;
}

.bubble__action-btn:active {
  border-color: var(--accent);
  color: var(--accent);
}

.bubble__remove {
  min-inline-size: 1.8em;
  text-align: center;
}

.bubble__remove.is-confirming {
  border-color: var(--danger);
  color: var(--danger);
  font-weight: 700;
}

.bubble__body {
  margin-block-start: 2px;
}

.bubble__fold {
  margin-block: 2px;
}

.bubble__summary {
  cursor: pointer;
  color: var(--muted);
  font-size: 0.84em;
  padding: 2px 0;
  user-select: none;
}

.bubble__reasoning {
  margin: 2px 0;
  padding: 0.4em 0.5em;
  background: var(--surface-strong);
  color: var(--muted);
  font-size: 0.85em;
  border-inline-start: 2px solid var(--line-strong);
}

.bubble__pre {
  margin: 2px 0;
  padding: 0.4em 0.5em;
  background: var(--surface-strong);
  white-space: pre-wrap;
  word-break: break-word;
  overflow-x: auto;
  font-size: 0.85em;
  font-family: inherit;
}

.is-error {
  color: var(--danger);
}
</style>
