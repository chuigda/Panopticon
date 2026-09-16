<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import type { ChatMessage, ChatTokenUsage } from '../session/message.ts'
import MarkdownEdit from './MarkdownEdit.vue'
import PlainTextEdit from './PlainTextEdit.vue'

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
  return `in ${u.inputTokens} · cache ${u.cacheInputTokens} · create ${u.cacheCreateTokens} · out ${u.outputTokens}`
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

onBeforeUnmount(() => clearTimeout(confirmTimer))
</script>

<template>
  <article class="bubble" :class="`bubble--${message.$k}`">
    <header class="bubble__head">
      <span class="bubble__role">#{{ index }} {{ label }}</span>
      <span v-if="message.$k === 'assistant'" class="bubble__meta">
        {{ fmtUsage(message.tokenUsage) }}
      </span>
      <button
        class="bubble__remove"
        :class="{ 'is-confirming': confirming }"
        type="button"
        :disabled="disabled"
        :title="confirming ? '再点一次确认删除' : '删除此消息'"
        @click="onRemoveClick"
      >{{ confirming ? '确定吗？' : '×' }}</button>
    </header>

    <template v-if="message.$k === 'user'">
      <PlainTextEdit class="bubble__body"
                     :model-value="message.content"
                     @update:model-value="emit('update:content', $event)"
                     :rows="6"
      />
    </template>

    <template v-else-if="message.$k === 'assistant'">
      <details v-if="message.reasoning" class="bubble__fold">
        <summary>思考</summary>
        <pre>{{ message.reasoning }}</pre>
      </details>
      <MarkdownEdit class="bubble__body"
                    :model-value="message.content"
                    @update:model-value="emit('update:content', $event)"
                    :rows="18"
      />
    </template>

    <template v-else-if="message.$k === 'tool_call'">
      <details class="bubble__fold">
        <summary>{{ message.calls.map((c) => c.name).join(', ') }}</summary>
        <pre v-if="message.content">{{ message.content }}</pre>
        <pre v-for="c in message.calls" :key="c.id">{{ c.name }}({{ JSON.stringify(c.arguments, null, 2) }})</pre>
      </details>
    </template>

    <template v-else-if="message.$k === 'tool_call_result'">
      <details class="bubble__fold">
        <summary>{{ message.results.length }} 个结果</summary>
        <pre v-for="r in message.results" :key="r.id" :class="{ 'is-error': r.isError }">{{ JSON.stringify(r.content, null, 2) }}</pre>
      </details>
    </template>

    <template v-else-if="message.$k === 'memory'">
      <details class="bubble__fold">
        <summary>记忆压缩 · {{ fmtUsage(message.tokenUsage) }}</summary>
        <MarkdownEdit :model-value="message.content"
                      @update:model-value="emit('update:content', $event)"
                      :rows="18"
        />
      </details>
    </template>

    <template v-else>
      <details class="bubble__fold" open>
        <summary>{{ message.error.split('\n')[0] }}</summary>
        <pre class="is-error">{{ message.error }}</pre>
      </details>
    </template>
  </article>
</template>

<style scoped>
.bubble {
  border-inline-start: 2px solid var(--line);
  padding: 2px 0.5em;
  font-size: 0.9em;
}

.bubble--user { border-color: var(--line-strong); }
.bubble--assistant { border-color: var(--accent); }
.bubble--tool_call, .bubble--tool_call_result { border-color: var(--focus); }
.bubble--memory { border-color: var(--muted); }
.bubble--error { border-color: var(--danger); }

.bubble__head {
  display: flex;
  align-items: center;
  gap: 0.5em;
  color: var(--muted);
  font-size: 0.78em;
}

.bubble__role { font-weight: 700; }
.bubble__meta { flex: 1; text-align: end; }

.bubble__remove {
  margin-inline-start: auto;
  border: 0;
  padding: 0 0.3em;
  color: var(--muted);
  background: transparent;
  line-height: 1;
}
.bubble__meta + .bubble__remove { margin-inline-start: 0; }
.bubble__remove:hover:not(:disabled),
.bubble__remove.is-confirming { color: var(--danger); }

.bubble__body { margin-block-start: 2px; }

.bubble__fold summary {
  cursor: pointer;
  color: var(--muted);
  font-size: 0.85em;
}

.bubble pre {
  margin: 2px 0;
  white-space: pre-wrap;
  word-break: break-word;
  font: inherit;
}

.is-error { color: var(--danger); }
</style>
