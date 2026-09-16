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
  <ul class="m-example-tree">
    <li v-for="(value, name) in tree" :key="name">
      <template v-if="typeof value === 'string'">
        <div class="m-example-tree__file">
          <span class="m-example-tree__name">{{ name }}</span>
          <Button variant="secondary" size="sm" @click="emit('download', String(name), value)">下载</Button>
        </div>
      </template>
      <template v-else>
        <div class="m-example-tree__dir">{{ name }}/</div>
        <ExampleTreeNodeMobile :tree="value" @download="(n, u) => emit('download', n, u)" />
      </template>
    </li>
  </ul>
</template>

<style scoped>
.m-example-tree {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 0.3em;
}

.m-example-tree .m-example-tree {
  margin-inline-start: 0.4em;
  border-inline-start: 1px solid var(--line);
  padding-inline-start: 0.5em;
}

.m-example-tree__dir {
  padding: 0.2em 0;
  color: var(--muted);
  font-size: 0.82em;
  font-weight: 700;
}

.m-example-tree__file {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  padding: 0.3em 0.5em;
}

.m-example-tree__name {
  min-width: 0;
  font-size: 0.85em;
  word-break: break-all;
}
</style>
