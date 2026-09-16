<script setup lang="ts">
import type { ExampleTree } from '../examples.ts'
import Button from './Button.vue'

defineProps<{
  tree: ExampleTree
}>()

const emit = defineEmits<{
  download: [name: string, url: string]
}>()
</script>

<template>
  <ul class="example-tree">
    <li v-for="(value, name) in tree" :key="name">
      <template v-if="typeof value === 'string'">
        <div class="example-tree__file">
          <span class="example-tree__name">{{ name }}</span>
          <Button variant="ghost" size="sm" @click="emit('download', String(name), value)">下载</Button>
        </div>
      </template>
      <template v-else>
        <div class="example-tree__dir">{{ name }}/</div>
        <ExampleTreeNode :tree="value" @download="(n, u) => emit('download', n, u)" />
      </template>
    </li>
  </ul>
</template>

<style scoped>
.example-tree {
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: 0.9em;
}

.example-tree .example-tree {
  margin-inline-start: 0.5em;
  border-inline-start: 1px solid var(--line);
  padding-inline-start: 0.5em;
}

.example-tree__dir {
  padding: 2px 0;
  color: var(--muted);
  font-weight: 700;
}

.example-tree__file {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  padding: 1px 0;
}

.example-tree__name {
  overflow-wrap: anywhere;
}
</style>
