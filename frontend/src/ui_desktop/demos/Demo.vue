<script setup lang="ts">
import { ref } from 'vue'
import Anthropic from '@anthropic-ai/sdk'
import OpenAI from 'openai'

const proxyOrigin = window.location.origin

const protocol = ref<'anthropic' | 'openai'>('anthropic')
const upstream = ref('https://api.anthropic.com')
const apiKey = ref('')
const model = ref('claude-sonnet-4-5')
const system = ref('')

type Msg = { role: 'user' | 'assistant'; content: string }
const messages = ref<Msg[]>([])
const input = ref('')
const sending = ref(false)
const error = ref('')

const isOpenAI = () => protocol.value === 'openai'

async function send() {
  const text = input.value.trim()
  if (!text || sending.value) return
  if (!apiKey.value) {
    error.value = '请先填写 API Key'
    return
  }
  error.value = ''
  messages.value.push({ role: 'user', content: text })
  input.value = ''
  messages.value.push({ role: 'assistant', content: '' })
  const target = messages.value[messages.value.length - 1]
  sending.value = true

  try {
    if (isOpenAI()) {
      await sendOpenAI(target)
    } else {
      await sendAnthropic(target)
    }
  } catch (e: any) {
    target.content = `[错误] ${e?.message ?? e}`
  } finally {
    sending.value = false
  }
}

async function sendAnthropic(target: Msg) {
  const client = new Anthropic({
    baseURL: proxyOrigin,
    apiKey: apiKey.value,
    dangerouslyAllowBrowser: true,
    defaultHeaders: { 'X-Upstream-Base-Url': upstream.value },
  })
  const stream = client.messages.stream({
    model: model.value,
    max_tokens: 4096,
    system: system.value || undefined,
    messages: messages.value
      .filter((m) => m.content)
      .map((m) => ({ role: m.role, content: m.content })),
  })
  for await (const event of stream) {
    if (event.type === 'content_block_delta' && event.delta.type === 'text_delta') {
      target.content += event.delta.text
    }
  }
}

async function sendOpenAI(target: Msg) {
  const client = new OpenAI({
    baseURL: proxyOrigin + '/v1',
    apiKey: apiKey.value,
    dangerouslyAllowBrowser: true,
    defaultHeaders: { 'X-Upstream-Base-Url': upstream.value },
  })
  const stream = await client.chat.completions.create({
    model: model.value,
    stream: true,
    messages: [
      ...(system.value ? [{ role: 'system' as const, content: system.value }] : []),
      ...messages.value
        .filter((m) => m.content)
        .map((m) => ({ role: m.role, content: m.content })),
    ],
  })
  for await (const chunk of stream) {
    const delta = chunk.choices[0]?.delta?.content
    if (delta) target.content += delta
  }
}
</script>

<template>
  <main style="max-width: 860px; margin: 0 auto; padding: 24px; font-family: system-ui, sans-serif">
    <h1 style="font-size: 20px; margin: 0 0 16px">Conducting Department</h1>

    <section style="border: 1px solid #ddd; border-radius: 8px; padding: 16px; margin-bottom: 16px">
      <div style="display: flex; gap: 12px; flex-wrap: wrap">
        <label style="display: flex; align-items: center; gap: 6px">
          协议
          <select v-model="protocol">
            <option value="anthropic">Anthropic</option>
            <option value="openai">OpenAI</option>
          </select>
        </label>
        <label style="flex: 1; min-width: 200px; display: flex; align-items: center; gap: 6px">
          远程 URL
          <input v-model="upstream" style="flex: 1; padding: 6px" />
        </label>
        <label style="flex: 1; min-width: 180px; display: flex; align-items: center; gap: 6px">
          API Key
          <input v-model="apiKey" type="password" style="flex: 1; padding: 6px" />
        </label>
        <label style="display: flex; align-items: center; gap: 6px">
          模型
          <input v-model="model" style="padding: 6px" />
        </label>
      </div>
      <div style="margin-top: 10px">
        <label style="display: flex; align-items: center; gap: 6px">
          System
          <input v-model="system" style="flex: 1; padding: 6px" />
        </label>
      </div>
    </section>

    <section style="border: 1px solid #ddd; border-radius: 8px; padding: 16px; margin-bottom: 16px; max-height: 420px; overflow-y: auto">
      <div v-for="(m, i) in messages" :key="i" style="margin-bottom: 12px">
        <div style="font-weight: 600; font-size: 12px; color: #666">
          {{ m.role === 'user' ? '你' : '助手' }}
        </div>
        <div style="white-space: pre-wrap; font-size: 14px">{{ m.content || '…' }}</div>
      </div>
      <div v-if="!messages.length" style="color: #999; font-size: 13px">还没有消息</div>
    </section>

    <section style="display: flex; gap: 8px">
      <textarea
        v-model="input"
        rows="3"
        placeholder="输入消息，Enter 发送"
        style="flex: 1; padding: 8px; resize: vertical"
        @keydown.enter.exact.prevent="send"
      ></textarea>
      <button
        :disabled="sending"
        @click="send"
        style="padding: 8px 20px; align-self: stretch; cursor: pointer; border: none; border-radius: 6px; background: #2563eb; color: #fff"
      >
        {{ sending ? '…' : '发送' }}
      </button>
    </section>

    <div v-if="error" style="margin-top: 12px; color: #b91c1c; font-size: 13px">{{ error }}</div>
  </main>
</template>
