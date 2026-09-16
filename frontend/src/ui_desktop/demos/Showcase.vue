<script setup lang="ts">
import { ref } from 'vue'
import Button from '../Button.vue'
import Dialog from '../Dialog.vue'
import Input from '../Input.vue'
import ToggleButton from '../ToggleButton.vue'
import ToggleButtonGroup from '../ToggleButtonGroup.vue'

const planName = ref('秋季排练计划')
const notes = ref('确认铜管声部的到场时间，并保留独奏排练窗口。')
const activeView = ref('agenda')
const notificationsEnabled = ref(true)
const dialogOpen = ref(false)
const saved = ref(false)

const viewOptions = [
  { value: 'agenda', label: '议程' },
  { value: 'players', label: '成员' },
  { value: 'scores', label: '乐谱' },
]

function saveChanges() {
  saved.value = true
  dialogOpen.value = false
}
</script>

<template>
  <main class="component-showcase">
    <header class="showcase-header">
      <div>
        <p class="showcase-header__eyebrow">CONDUCTING / CONTROL ROOM</p>
        <h1>排练控制台</h1>
        <p class="showcase-header__summary">秋季音乐会的排练安排与发布状态。</p>
      </div>
      <p class="theme-indicator">系统外观</p>
    </header>

    <section class="control-section" aria-labelledby="actions-title">
      <div class="section-heading">
        <div>
          <p class="section-heading__eyebrow">ACTIONS</p>
          <h2 id="actions-title">操作</h2>
        </div>
        <p class="status-line" :class="{ 'is-saved': saved }">
          {{ saved ? '草稿已保存' : '草稿待保存' }}
        </p>
      </div>
      <div class="button-row">
        <Button @click="dialogOpen = true">保存更改</Button>
        <Button variant="secondary">预览排练单</Button>
        <Button variant="danger">取消发布</Button>
        <Button variant="ghost" disabled>已锁定</Button>
      </div>
    </section>

    <div class="control-grid">
      <section class="control-section" aria-labelledby="form-title">
        <div class="section-heading">
          <div>
            <p class="section-heading__eyebrow">FIELDS</p>
            <h2 id="form-title">排练信息</h2>
          </div>
        </div>
        <div class="field-grid">
          <Input
            v-model="planName"
            label="计划名称"
            description="显示在本次排练单标题中。"
            placeholder="输入计划名称"
          />
          <label class="textarea-field">
            <span class="input-field__label">调度备注</span>
            <textarea v-model="notes" rows="4" placeholder="输入备注" />
          </label>
        </div>
      </section>

      <section class="control-section" aria-labelledby="switches-title">
        <div class="section-heading">
          <div>
            <p class="section-heading__eyebrow">SELECTION</p>
            <h2 id="switches-title">视图与通知</h2>
          </div>
        </div>
        <div class="selection-stack">
          <div class="selection-row">
            <span class="selection-row__label">排练视图</span>
            <ToggleButtonGroup v-model="activeView" :options="viewOptions" aria-label="排练视图" />
          </div>
          <div class="selection-row">
            <span class="selection-row__label">变动通知</span>
            <ToggleButton v-model="notificationsEnabled">启用</ToggleButton>
          </div>
        </div>
      </section>
    </div>

    <Dialog v-model="dialogOpen" title="保存排练单">
      <p>将“{{ planName || '未命名计划' }}”保存为当前排练草稿。</p>
      <template #footer>
        <Button variant="ghost" @click="dialogOpen = false">返回</Button>
        <Button @click="saveChanges">确认保存</Button>
      </template>
    </Dialog>
  </main>
</template>
