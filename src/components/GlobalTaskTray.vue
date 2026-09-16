<script setup lang="ts">
import { useRouter } from 'vue-router'
import downloadIcon from 'iconoir/icons/download.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import settingsIcon from 'iconoir/icons/settings.svg?url'
import xmarkIcon from 'iconoir/icons/xmark.svg?url'
import { useActivityStore, type ActivityTask } from '@/stores/activity'

const router = useRouter()
const activity = useActivityStore()

function iconStyle(url: string) {
  return { '--icon-url': `url("${url}")` }
}

function kindIcon(task: ActivityTask) {
  if (task.kind === 'download') return downloadIcon
  if (task.kind === 'setup') return settingsIcon
  return mediaVideoIcon
}

function kindLabel(task: ActivityTask) {
  if (task.kind === 'download') return '下载'
  if (task.kind === 'setup') return '准备'
  return '字幕'
}

function queueLabel(task: ActivityTask) {
  const queue = task.queue
  if (!queue || queue.total <= 1) return ''
  return `第 ${queue.current}/${queue.total} 项 · 完成 ${queue.completed}${queue.failed ? ` · 失败 ${queue.failed}` : ''}`
}

function openTask(task: ActivityTask) {
  void router.push({
    name: task.target,
    query: task.target === 'subtitles' && task.projectId ? { project: task.projectId } : undefined,
  })
}
</script>

<template>
  <TransitionGroup v-if="activity.visibleTasks.length" name="activity-task" tag="section" class="global-task-tray" aria-live="polite" aria-label="任务状态">
    <article v-for="task in activity.visibleTasks" :key="task.id" class="global-task" :class="[`is-${task.state}`, `is-${task.kind}`]">
      <span class="global-task-kind" :title="kindLabel(task)">
        <i class="icon" :style="iconStyle(kindIcon(task))" />
      </span>

      <button class="global-task-main" type="button" @click="openTask(task)">
        <span class="global-task-heading">
          <strong>{{ task.title }}</strong>
          <small>{{ queueLabel(task) || task.detail }}</small>
        </span>
        <span class="global-task-meta">
          <b v-if="task.percent !== null">{{ Math.round(task.percent) }}%</b>
          <span v-else>{{ task.state === 'completed' ? '完成' : task.state === 'failed' ? '失败' : task.state === 'cancelled' ? '已取消' : '处理中' }}</span>
        </span>
      </button>

      <div class="global-task-progress" :class="{ indeterminate: task.percent === null && task.state === 'active' }">
        <i :style="task.percent === null ? undefined : { width: `${task.percent}%` }" />
      </div>

      <div class="global-task-actions">
        <button v-if="task.cancellable" class="global-task-action" type="button" @click="activity.cancelTask(task.id)">取消</button>
        <button v-else-if="!['active', 'cancelling'].includes(task.state)" class="global-task-dismiss" type="button" aria-label="关闭任务提示" @click="activity.dismissTask(task.id)">
          <i class="icon" :style="iconStyle(xmarkIcon)" />
        </button>
      </div>
    </article>
  </TransitionGroup>
</template>

<style scoped>
.global-task-tray {
  position: absolute;
  right: clamp(14px, 2vw, 24px);
  bottom: clamp(14px, 2vw, 24px);
  z-index: 40;
  display: grid;
  width: min(440px, calc(100% - 28px));
  gap: 10px;
  pointer-events: none;
}

.global-task {
  position: relative;
  display: grid;
  grid-template-columns: 30px minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  min-height: 72px;
  overflow: hidden;
  padding: 11px 11px 11px 13px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-panel);
  background: color-mix(in oklch, var(--surface) 97%, var(--canvas));
  box-shadow: var(--shadow-float);
  pointer-events: auto;
}

.global-task.is-completed {
  border-color: color-mix(in oklch, var(--success) 34%, var(--border));
}

.global-task.is-failed,
.global-task.is-cancelled {
  border-color: color-mix(in oklch, var(--danger) 30%, var(--border));
}

.global-task-kind {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-control);
  background: var(--primary);
  color: var(--surface);
}

.is-setup .global-task-kind {
  background: var(--surface-hover);
  color: var(--muted);
}

.is-completed .global-task-kind {
  background: var(--success-soft);
  color: var(--success);
}

.is-failed .global-task-kind,
.is-cancelled .global-task-kind {
  background: var(--danger-soft);
  color: var(--danger);
}

.global-task-kind .icon {
  width: 14px;
  height: 14px;
}

.global-task-main {
  display: grid;
  min-width: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.global-task-main:focus-visible {
  outline: 2px solid var(--primary);
  outline-offset: 3px;
}

.global-task-heading {
  display: grid;
  min-width: 0;
  gap: 3px;
}

.global-task-heading strong,
.global-task-heading small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.global-task-heading strong {
  color: var(--ink);
  font-size: 12px;
  font-weight: 700;
}

.global-task-heading small {
  color: var(--subtle);
  font-size: 10px;
}

.global-task-meta {
  margin-right: 3px;
  color: var(--muted);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.global-task-meta b {
  color: var(--ink);
  font-weight: 700;
}

.global-task-progress {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  height: 3px;
  overflow: hidden;
  background: var(--border);
}

.global-task-progress i {
  display: block;
  width: 0;
  height: 100%;
  border-radius: inherit;
  background: var(--primary);
  transition: width 180ms linear;
}

.is-completed .global-task-progress i {
  background: var(--success);
}

.is-failed .global-task-progress i,
.is-cancelled .global-task-progress i {
  background: var(--danger);
}

.global-task-progress.indeterminate i {
  width: 36%;
  animation: global-task-slide 1.2s ease-in-out infinite;
}

.global-task-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  min-width: 30px;
}

.global-task-action,
.global-task-dismiss {
  display: inline-grid;
  place-items: center;
  min-width: 30px;
  height: 30px;
  padding: 0 7px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--muted);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
}

.global-task-action:hover,
.global-task-dismiss:hover {
  background: var(--surface-hover);
  color: var(--ink);
}

.global-task-dismiss .icon {
  width: 14px;
  height: 14px;
}

.activity-task-enter-active,
.activity-task-leave-active,
.activity-task-move {
  transition: transform 220ms var(--ease-out), opacity 160ms ease-out;
}

.activity-task-enter-from,
.activity-task-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

@keyframes global-task-slide {
  0% { transform: translateX(-110%); }
  55%, 100% { transform: translateX(310%); }
}

@container workspace (max-width: 620px) {
  .global-task-tray {
    right: 12px;
    bottom: 12px;
    width: calc(100% - 24px);
  }
}
</style>
