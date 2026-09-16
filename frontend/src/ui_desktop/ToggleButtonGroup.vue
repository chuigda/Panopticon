<script setup lang="ts">
import { ref } from 'vue'

type ToggleButtonOption = {
  value: string
  label: string
  disabled?: boolean
}

const props = withDefaults(defineProps<{
  modelValue: string
  options: readonly ToggleButtonOption[]
  ariaLabel?: string
  disabled?: boolean
  vertical?: boolean
}>(), {
  ariaLabel: 'Toggle options',
  disabled: false,
  vertical: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  change: [value: string]
}>()

const groupElement = ref<HTMLElement | null>(null)

function select(value: string, disabled?: boolean) {
  if (props.disabled || disabled || value === props.modelValue) {
    return
  }

  emit('update:modelValue', value)
  emit('change', value)
}

function focusOption(value: string) {
  const buttons = groupElement.value?.querySelectorAll<HTMLButtonElement>('.toggle-button')
  Array.from(buttons ?? []).find((button) => button.dataset.value === value)?.focus()
}

function handleKeydown(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) {
    return
  }

  const availableOptions = props.options.filter((option) => !option.disabled)
  if (!availableOptions.length) {
    return
  }

  event.preventDefault()

  const currentIndex = availableOptions.findIndex((option) => option.value === props.modelValue)
  const offset = event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : 1
  let nextIndex = currentIndex < 0 ? 0 : (currentIndex + offset + availableOptions.length) % availableOptions.length

  if (event.key === 'Home') {
    nextIndex = 0
  }
  if (event.key === 'End') {
    nextIndex = availableOptions.length - 1
  }

  const nextOption = availableOptions[nextIndex]
  select(nextOption.value)
  focusOption(nextOption.value)
}
</script>

<template>
  <div
    ref="groupElement"
    class="toggle-button-group"
    :class="{ 'is-vertical': vertical }"
    role="radiogroup"
    :aria-orientation="vertical ? 'vertical' : 'horizontal'"
    :aria-label="ariaLabel"
    :aria-disabled="disabled || undefined"
    @keydown="handleKeydown"
  >
    <button
      v-for="(option, index) in options"
      :key="option.value"
      class="toggle-button"
      :class="{ 'is-pressed': option.value === modelValue }"
      :data-value="option.value"
      type="button"
      role="radio"
      :aria-checked="option.value === modelValue"
      :tabindex="option.value === modelValue || (!options.some((item) => item.value === modelValue) && index === 0) ? 0 : -1"
      :disabled="disabled || option.disabled"
      @click="select(option.value, option.disabled)"
    >
      {{ option.label }}
    </button>
  </div>
</template>
