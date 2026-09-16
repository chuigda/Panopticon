<script setup lang="ts">
withDefaults(defineProps<{
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost'
  size?: 'sm' | 'md'
  loading?: boolean
  disabled?: boolean
  block?: boolean
  type?: 'button' | 'submit' | 'reset'
}>(), {
  variant: 'primary',
  size: 'md',
  loading: false,
  disabled: false,
  block: false,
  type: 'button',
})
</script>

<template>
  <button
    class="button"
    :class="[`button--${variant}`, `button--${size}`, { 'button--block': block }]"
    :type="type"
    :disabled="disabled || loading"
    :aria-busy="loading || undefined"
  >
    <span v-if="loading" class="button__spinner" aria-hidden="true" />
    <span class="button__content"><slot /></span>
  </button>
</template>

<style scoped>
.button--block {
  display: flex;
  inline-size: 100%;
}
</style>
