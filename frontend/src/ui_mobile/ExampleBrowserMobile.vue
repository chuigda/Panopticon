<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { downloadExample, fetchExampleTree, type ExampleTree } from '../examples.ts'
import ExampleTreeNodeMobile from './ExampleTreeNodeMobile.vue'

const tree = ref<ExampleTree>()
const loading = ref(false)
const error = ref('')

async function load() {
  loading.value = true
  error.value = ''
  try {
    tree.value = await fetchExampleTree()
  } catch (e) {
    error.value = `获取示例列表失败：${e instanceof Error ? e.message : e}`
  } finally {
    loading.value = false
  }
}

async function onDownload(name: string, url: string) {
  try {
    await downloadExample(name, url)
  } catch (e) {
    error.value = `下载 ${name} 失败：${e instanceof Error ? e.message : e}`
  }
}

onMounted(load)
</script>

<template>
  <div class="m-example-browser">
    <p class="m-example-browser__hint">下载后可在「模拟配置」中作为 SimulatorCHR / AdditionalCHR / PlayerCHR 加载。</p>
    <p v-if="loading" class="m-example-browser__hint">加载中…</p>
    <pre v-if="error" class="m-example-browser__error">{{ error }}</pre>
    <ExampleTreeNodeMobile v-if="tree" :tree="tree" @download="onDownload" />
  </div>
</template>

<style scoped>
.m-example-browser {
  display: flex;
  flex-direction: column;
  gap: 0.6em;
}

.m-example-browser__hint {
  margin: 0;
  color: var(--muted);
  font-size: 0.82em;
  line-height: 1.4;
}

.m-example-browser__error {
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
