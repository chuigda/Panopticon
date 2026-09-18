<script setup lang="ts" generic="T">
import { ref, useId } from 'vue'

defineOptions({ inheritAttrs: false })

type ToggleButtonOption<T = string> = {
  value: T
  label: string
  disabled?: boolean
}

const props = withDefaults(defineProps<{
  modelValue: T
  options: readonly ToggleButtonOption<T>[]
  label?: string
  ariaLabel?: string
  disabled?: boolean
  vertical?: boolean
}>(), {
  label: '',
  ariaLabel: 'Toggle options',
  disabled: false,
  vertical: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: T]
  change: [value: T]
}>()

const groupElement = ref<HTMLElement | null>(null)
const labelId = `toggle-group-${useId()}-label`

function select(value: T, disabled?: boolean) {
  if (props.disabled || disabled || value === props.modelValue) {
    return
  }

  emit('update:modelValue', value)
  emit('change', value)
}

function focusOption(value: T) {
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
  <div class="toggle-button-field" :class="{ 'has-label': label, 'is-disabled': disabled }">
    <span v-if="label" :id="labelId" class="input-field__label">{{ label }}</span>
    <div
      ref="groupElement"
      v-bind="$attrs"
      class="toggle-button-group"
      :class="{ 'is-vertical': vertical }"
      role="radiogroup"
      :aria-orientation="vertical ? 'vertical' : 'horizontal'"
      :aria-label="label ? undefined : ariaLabel"
      :aria-labelledby="label ? labelId : undefined"
      :aria-disabled="disabled || undefined"
      @keydown="handleKeydown"
    >
      <button
        v-for="(option, index) in options"
        :key="`${option.value}`"
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
  </div>
</template>
