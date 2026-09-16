<script setup lang="ts">
import { computed, nextTick, ref, type Ref } from 'vue'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import Button from './Button.vue'
import Dialog from './Dialog.vue'

const props = withDefaults(defineProps<{
  modelValue: string
  rows?: number
  preprocess?: (markdown: string) => string
  allowDirectEdit?: boolean
}>(), {
  rows: 8,
  allowDirectEdit: true,
})

const emit = defineEmits<{ (e: 'update:modelValue', value: string): void }>()

const status: Ref<'display' | 'edit'> = ref('display')
const editValue = ref('')
const editor = ref<HTMLTextAreaElement | null>(null)
const discardDialogOpen = ref(false)

const renderedMarkdown = computed(() => {
  let markdown = props.modelValue
  if (props.preprocess) {
    markdown = props.preprocess(markdown)
  }
  return DOMPurify.sanitize(marked.parse(markdown, { async: false }), {
    ADD_TAGS: ['semantics', 'annotation'],
    ADD_ATTR: ['encoding'],
  })
})

async function startEditing() {
  if (!props.allowDirectEdit) return
  editValue.value = props.modelValue
  status.value = 'edit'
  await nextTick()
  editor.value?.focus()
}

function commitEditing() {
  emit('update:modelValue', editValue.value)
  status.value = 'display'
}

function cancelEditing() {
  if (editValue.value === props.modelValue) {
    status.value = 'display'
    return
  }
  discardDialogOpen.value = true
}

function discardEditing() {
  discardDialogOpen.value = false
  editValue.value = props.modelValue
  status.value = 'display'
}

function handleEditorKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault()
    cancelEditing()
    return
  }

  if (event.key === 'Enter' && event.ctrlKey) {
    event.preventDefault()
    commitEditing()
  }
}

defineExpose({
  startEditing,
  commitEditing,
  cancelEditing,
  status,
})
</script>

<template>
  <div v-if="status === 'display'" class="edit-wrapper">
    <div
      class="markdown"
      v-html="renderedMarkdown"
      role="button"
      tabindex="0"
      @dblclick="startEditing"
    />
  </div>
  <div v-else class="editor-pane">
    <textarea
      ref="editor"
      v-model="editValue"
      spellcheck="false"
      wrap="soft"
      class="editor-textarea"
      :rows="rows"
      @keydown="handleEditorKeydown"
    />
    <div class="editor-actions">
      <Button variant="ghost" size="sm" @click="cancelEditing">取消</Button>
      <Button size="sm" @click="commitEditing">保存</Button>
    </div>
  </div>

  <Dialog v-model="discardDialogOpen" title="放弃修改">
    <p>确定要放弃当前修改吗？</p>
    <template #footer>
      <Button variant="ghost" @click="discardDialogOpen = false">取消</Button>
      <Button @click="discardEditing">放弃</Button>
    </template>
  </Dialog>
</template>

<style scoped>
.edit-wrapper {
  position: relative;
  min-height: 1.5em;
}

.editor-pane {
  display: flex;
  flex-direction: column;
  gap: 0.5em;
  margin-block: 0.3em;
}

.editor-textarea {
  inline-size: 100%;
  border: 1px solid var(--accent);
  background: var(--surface);
  color: var(--ink);
  padding: 0.5em;
  font-size: 0.95rem;
  line-height: 1.4;
  resize: vertical;
}

.editor-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.5em;
}
</style>
