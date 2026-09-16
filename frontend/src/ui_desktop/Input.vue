<script setup lang="ts">
import { computed, ref, useId } from 'vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  modelValue: string | undefined
  label?: string
  description?: string
  error?: string
  id?: string
  type?: string
  disabled?: boolean
  // 输入框为空时 emit '' 还是 undefined
  emptyBehavior?: 'empty-string' | 'undefined'
}>(), {
  label: '',
  description: '',
  error: '',
  id: '',
  type: 'text',
  disabled: false,
  emptyBehavior: 'empty-string',
})

const emit = defineEmits<{
  'update:modelValue': [value: string | undefined]
}>()

const inputElement = ref<HTMLInputElement | null>(null)
const generatedId = useId()
const inputId = computed(() => props.id || `input-${generatedId}`)
const descriptionId = computed(() => `${inputId.value}-description`)
const describedBy = computed(() => props.description || props.error ? descriptionId.value : undefined)

function updateValue(event: Event) {
  const value = (event.target as HTMLInputElement).value
  emit('update:modelValue', value === '' && props.emptyBehavior === 'undefined' ? undefined : value)
}

defineExpose({
  focus: () => inputElement.value?.focus(),
})
</script>

<template>
  <div class="input-field" :class="{ 'has-error': error, 'is-disabled': disabled }">
    <label v-if="label" class="input-field__label" :for="inputId">{{ label }}</label>
    <input
      ref="inputElement"
      v-bind="$attrs"
      :id="inputId"
      class="input-field__control"
      :type="type"
      :value="modelValue ?? ''"
      :disabled="disabled"
      :aria-describedby="describedBy"
      :aria-invalid="error ? true : undefined"
      @input="updateValue"
    >
    <p
      v-if="description || error"
      :id="descriptionId"
      class="input-field__supporting-text"
      :class="{ 'is-error': error }"
      :role="error ? 'alert' : undefined"
    >
      {{ error || description }}
    </p>
  </div>
</template>
