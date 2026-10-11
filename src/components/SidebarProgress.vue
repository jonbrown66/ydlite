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
const activeCount = computed(() => activity.visibleTasks.filter(item => ['active', 'cancelling'].includes(item.state)).length)

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
      :title="`${activeCount} tasks · ${task.title} · ${task.detail}`"
      aria-label="View task progress"
      @click="openTask"
    >
      <span class="sidebar-progress-head">
        <span class="sidebar-progress-name">
          <i v-if="task.percent === null" class="progress-spinner" aria-hidden="true" />
          <i v-else class="icon" :style="iconStyle(taskIcon(task))" />
          <span>{{ activeCount > 1 ? `${activeCount} tasks · ` : '' }}{{ task.title }}</span>
        </span>
        <strong>{{ task.percent === null ? '…' : `${Math.round(progress)}%` }}</strong>
      </span>
      <span class="sidebar-progress-track">
        <i :style="{ width: `${task.percent === null ? 0 : progress}%` }" />
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
  background: var(--surface);
  box-shadow: var(--control-shadow);
  color: var(--workspace-muted);
  text-align: left;
  cursor: pointer;
}

.sidebar-progress:hover {
  background: var(--surface-hover);
}

.sidebar-progress-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
  font-size: 14px;
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
  color: var(--workspace-ink);
}

.sidebar-progress-head strong {
  flex: 0 0 auto;
  color: var(--workspace-accent);
  font-size: 14px;
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

.sidebar-progress-enter-active,
.sidebar-progress-leave-active {
  transition: opacity 180ms ease-out, transform 220ms var(--workspace-ease);
}

.sidebar-progress-enter-from,
.sidebar-progress-leave-to {
  opacity: 0;
  transform: translateY(5px);
}

</style>
