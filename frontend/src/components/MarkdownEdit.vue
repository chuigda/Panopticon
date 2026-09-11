<script setup lang="ts">
import { computed, nextTick, ref, type Ref } from 'vue'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import Button from './Button.vue'
import Dialog from './Dialog.vue'

const props = defineProps<{
  modelValue: string,
  rows?: number,
  preprocess?: (markdown: string) => string
}>()
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
  return DOMPurify.sanitize(marked.parse(markdown, { async: false }))
})

async function startEditing() {
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
</script>

<template>
  <div v-if="status === 'display'"
       class="markdown"
       v-html="renderedMarkdown"
       role="button"
       @dblclick="startEditing"
  />
  <textarea v-else
            ref="editor"
            v-model="editValue"
            spellcheck="false"
            wrap="soft"
            @keydown="handleEditorKeydown"
            :rows="rows"
            :style="rows ? undefined : { width: 'calc(100% - 0.5em)', height: 'calc(100% - 0.5em)' }"
  />
  <Dialog v-model="discardDialogOpen" title="放弃修改">
    <p>确定要放弃当前修改吗？</p>
    <template #footer>
      <Button variant="ghost" @click="discardDialogOpen = false">取消</Button>
      <Button @click="discardEditing">放弃</Button>
    </template>
  </Dialog>
</template>
