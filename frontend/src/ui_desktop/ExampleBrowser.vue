<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { downloadExample, fetchExampleTree, type ExampleTree } from '../examples.ts'
import ExampleTreeNode from './ExampleTreeNode.vue'

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
  <div class="example-browser">
    <p class="example-browser__hint">下载后可在「模拟配置」中作为 SimulatorCHR / AdditionalCHR / PlayerCHR 加载。</p>
    <p v-if="loading" class="example-browser__muted">加载中…</p>
    <pre v-if="error" class="example-browser__error">{{ error }}</pre>
    <ExampleTreeNode v-if="tree" :tree="tree" @download="onDownload" />
  </div>
</template>

<style scoped>
.example-browser {
  display: grid;
  gap: 0.5em;
  max-block-size: 60vh;
  overflow-y: auto;
  color: var(--ink);
}

.example-browser__hint,
.example-browser__muted {
  margin: 0;
  color: var(--muted);
  font-size: 0.85em;
}

.example-browser__error {
  margin: 0;
  white-space: pre-wrap;
  color: var(--danger);
  font: inherit;
  font-size: 0.85em;
}
</style>
