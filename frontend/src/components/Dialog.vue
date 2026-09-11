<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'

const props = withDefaults(defineProps<{
  modelValue: boolean
  title?: string
  ariaLabel?: string
  closeLabel?: string
}>(), {
  title: '',
  ariaLabel: 'Dialog',
  closeLabel: '关闭',
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  close: []
}>()

const dialogElement = ref<HTMLDialogElement | null>(null)
const dialogId = useId()
let closingFromModel = false

async function synchronizeDialog(open: boolean) {
  await nextTick()

  const dialog = dialogElement.value
  if (!dialog) {
    return
  }

  if (open && !dialog.open) {
    dialog.showModal()
    return
  }

  if (!open && dialog.open) {
    closingFromModel = true
    dialog.close()
    closingFromModel = false
  }
}

function requestClose() {
  if (!props.modelValue) {
    return
  }

  emit('update:modelValue', false)
  emit('close')
}

function handleCancel(event: Event) {
  event.preventDefault()

  const dialog = dialogElement.value
  if (dialog?.open) {
    dialog.close()
    return
  }

  requestClose()
}

function handleNativeClose() {
  if (!closingFromModel) {
    requestClose()
  }
}

watch(() => props.modelValue, (open) => {
  void synchronizeDialog(open)
}, { immediate: true })

onBeforeUnmount(() => {
  if (dialogElement.value?.open) {
    closingFromModel = true
    dialogElement.value.close()
    closingFromModel = false
  }
})
</script>

<template>
  <dialog
    ref="dialogElement"
    class="control-dialog"
    closedby="closerequest"
    :aria-labelledby="props.title ? `${dialogId}-title` : undefined"
    :aria-label="props.title ? undefined : props.ariaLabel"
    @cancel="handleCancel"
    @close="handleNativeClose"
  >
    <div class="control-dialog__frame">
      <header class="control-dialog__header">
        <h2 v-if="props.title" :id="`${dialogId}-title`" class="control-dialog__title">
          {{ props.title }}
        </h2>
        <button class="control-dialog__close" type="button" :aria-label="props.closeLabel" @click="requestClose">
          {{ props.closeLabel }}
        </button>
      </header>
      <div class="control-dialog__body">
        <slot />
      </div>
      <footer v-if="$slots.footer" class="control-dialog__footer">
        <slot name="footer" />
      </footer>
    </div>
  </dialog>
</template>
