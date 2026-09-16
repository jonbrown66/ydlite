<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import downloadIcon from 'iconoir/icons/download.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import { useActivityStore, type ActivityTask } from '@/stores/activity'

const router = useRouter()
const activity = useActivityStore()

const task = computed(() => activity.visibleTasks.find(item =>
  ['active', 'cancelling'].includes(item.state) && item.kind === 'download',
) ?? activity.visibleTasks.find(item => ['active', 'cancelling'].includes(item.state)))
const progress = computed(() => task.value?.percent ?? 0)

function iconStyle(url: string) {
  return { '--icon-url': `url("${url}")` }
}

function taskIcon(item: ActivityTask) {
  return item.kind === 'download' ? downloadIcon : mediaVideoIcon
}

function openTask() {
  if (!task.value) return
  void router.push({
    name: task.value.target,
    query: task.value.target === 'subtitles' && task.value.projectId
      ? { project: task.value.projectId }
      : undefined,
  })
}
</script>

<template>
  <Transition name="sidebar-progress">
    <button
      v-if="task"
      class="sidebar-progress"
      type="button"
      :title="`${task.title} · ${task.detail}`"
      aria-label="查看当前任务进度"
      @click="openTask"
    >
      <span class="sidebar-progress-head">
        <span class="sidebar-progress-name">
          <i class="icon" :style="iconStyle(taskIcon(task))" />
          <span>{{ task.title }}</span>
        </span>
        <strong>{{ task.percent === null ? '…' : `${Math.round(progress)}%` }}</strong>
      </span>
      <span class="sidebar-progress-track">
        <i :style="task.percent === null ? undefined : { width: `${progress}%` }" />
      </span>
    </button>
  </Transition>
</template>

<style scoped>
.sidebar-progress {
  display: grid;
  gap: 7px;
  width: 100%;
  margin: 0 0 8px;
  padding: 8px 10px 9px;
  border: 0;
  border-radius: var(--radius-control);
  background: color-mix(in oklch, var(--workspace-accent-soft) 68%, var(--workspace-surface));
  color: var(--workspace-muted);
  text-align: left;
  cursor: pointer;
  transition: background-color 160ms ease-out, transform 180ms var(--workspace-ease);
}

.sidebar-progress:hover {
  background: color-mix(in oklch, var(--workspace-accent-soft) 90%, var(--workspace-surface));
  transform: translateY(-1px);
}

.sidebar-progress-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
  font-size: 10px;
}

.sidebar-progress-name {
  display: inline-flex;
  align-items: center;
  min-width: 0;
  gap: 6px;
}

.sidebar-progress-name span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sidebar-progress-name .icon {
  width: 13px;
  height: 13px;
  color: var(--workspace-accent);
}

.sidebar-progress-head strong {
  flex: 0 0 auto;
  color: var(--workspace-accent);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}

.sidebar-progress-track {
  display: block;
  height: 3px;
  overflow: hidden;
  border-radius: 99px;
  background: color-mix(in oklch, var(--workspace-accent) 15%, var(--workspace-border));
}

.sidebar-progress-track i {
  display: block;
  width: 36%;
  height: 100%;
  border-radius: inherit;
  background: var(--workspace-accent);
  transition: width 220ms var(--workspace-ease);
}

.sidebar-progress-track i:not([style]) {
  animation: sidebar-progress-slide 1.2s ease-in-out infinite;
}

.sidebar-progress-enter-active,
.sidebar-progress-leave-active {
  transition: opacity 180ms ease-out, transform 220ms var(--workspace-ease);
}

.sidebar-progress-enter-from,
.sidebar-progress-leave-to {
  opacity: 0;
  transform: translateY(5px);
}

@keyframes sidebar-progress-slide {
  0% { transform: translateX(-110%); }
  55%, 100% { transform: translateX(310%); }
}

@media (prefers-reduced-motion: reduce) {
  .sidebar-progress-track i:not([style]) {
    animation: none;
  }
}
</style>
