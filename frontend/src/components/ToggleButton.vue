<script setup lang="ts">
const props = withDefaults(defineProps<{
  modelValue: boolean
  disabled?: boolean
  type?: 'button' | 'submit' | 'reset'
}>(), {
  disabled: false,
  type: 'button',
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  change: [value: boolean]
}>()

function toggle() {
  if (props.disabled) {
    return
  }

  const value = !props.modelValue
  emit('update:modelValue', value)
  emit('change', value)
}
</script>

<template>
  <button
    class="toggle-button"
    :class="{ 'is-pressed': modelValue }"
    :type="type"
    :disabled="disabled"
    :aria-pressed="modelValue"
    @click="toggle"
  >
    <slot />
  </button>
</template>