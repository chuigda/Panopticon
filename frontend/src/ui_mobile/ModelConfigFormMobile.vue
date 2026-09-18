<script setup lang="ts">
import type { ModelConfig } from '../config.ts'
import Input from './Input.vue'
import ToggleButtonGroup from './ToggleButtonGroup.vue'

const model = defineModel<ModelConfig>({ required: true })

const enableDisableOptions = [
  { value: true, label: '启用' },
  { value: false, label: '关闭' }
] as const

const protocolOptions = [
  { value: 'messages', label: 'Anthropic' },
  { value: 'chat-completions', label: 'Chat Completions' },
] as const

const cacheOptions = [
  { value: '', label: '无' },
  { value: '5m', label: '5m' },
  { value: '1h', label: '1h' },
] as const

function num(v: string | undefined): number | undefined {
  if (v === undefined || v.trim() === '') return undefined
  const n = Number(v)
  return Number.isNaN(n) ? undefined : n
}

function str(v: number | undefined) {
  return v === undefined ? '' : String(v)
}
</script>

<template>
  <div class="mcf-mobile">
    <Input
      :model-value="model.uri"
      label="Base URL"
      placeholder="https://api.example.com"
      @update:model-value="model.uri = $event ?? ''"
    />
    <Input
      :model-value="model.apiKey"
      label="API Key"
      type="password"
      @update:model-value="model.apiKey = $event ?? ''"
    />
    <Input
      :model-value="model.modelName"
      label="模型名 (Model)"
      @update:model-value="model.modelName = $event ?? ''"
    />

    <ToggleButtonGroup
      :model-value="model.protocol"
      :options="protocolOptions"
      label="协议"
      @update:model-value="model.protocol = $event as ModelConfig['protocol']"
    />

    <div class="mcf-mobile__field-row">
      <ToggleButtonGroup
          v-model="model.direct"
          :options="enableDisableOptions"
          label="直连"
      />
      <ToggleButtonGroup v-model="model.thinkingEnabled" :options="enableDisableOptions" label="思考" />
    </div>

    <div class="mcf-mobile__field-row" v-if="model.protocol === 'messages'">
      <ToggleButtonGroup
        :model-value="model.cacheTtl ?? ''"
        :options="cacheOptions"
        label="消息缓存"
        @update:model-value="model.cacheTtl = ($event || undefined) as ModelConfig['cacheTtl']"
      />
      <ToggleButtonGroup
        :model-value="model.systemCacheTtl ?? ''"
        :options="cacheOptions"
        label="System 缓存"
        @update:model-value="model.systemCacheTtl = ($event || undefined) as ModelConfig['systemCacheTtl']"
      />
    </div>

    <div class="mcf-mobile__grid">
      <Input
        :model-value="String(model.outputBudget)"
        label="输出预算"
        type="number"
        @update:model-value="model.outputBudget = num($event) ?? model.outputBudget"
      />
      <Input
        v-model="model.reasoningEffort"
        label="推理强度"
        placeholder="low / med / hi"
        empty-behavior="undefined"
      />
      <Input
        :model-value="str(model.temperature)"
        label="Temperature"
        type="number"
        step="0.05"
        empty-behavior="undefined"
        @update:model-value="model.temperature = num($event)"
      />
      <Input
        :model-value="str(model.topP)"
        label="Top P"
        type="number"
        step="0.05"
        empty-behavior="undefined"
        @update:model-value="model.topP = num($event)"
      />
      <Input
        :model-value="str(model.topK)"
        label="Top K"
        type="number"
        step="1"
        empty-behavior="undefined"
        @update:model-value="model.topK = num($event)"
      />
      <Input
        :model-value="str(model.frequencyPenalty)"
        label="Freq Penalty"
        type="number"
        step="0.1"
        empty-behavior="undefined"
        @update:model-value="model.frequencyPenalty = num($event)"
      />
      <Input
        :model-value="str(model.presencePenalty)"
        label="Pres Penalty"
        type="number"
        step="0.1"
        empty-behavior="undefined"
        @update:model-value="model.presencePenalty = num($event)"
      />
    </div>
  </div>
</template>

<style scoped>
.mcf-mobile {
  display: flex;
  flex-direction: column;
  gap: 0.6em;
}

.mcf-mobile__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.5em;
}

.mcf-mobile__field-row {
  display: flex;
  align-items: center;
  gap: 0.5em;
  padding-block: 0.2em;
}
</style>
