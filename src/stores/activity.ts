import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  cancelDownload,
  onDownloadProgress,
  onToolInstallProgress,
} from '@/api/tauri'
import type { DownloadProgressEvent, ToolInstallEvent } from '@/types'

export type ActivityTaskKind = 'download' | 'setup'
export type ActivityTaskState = 'active' | 'cancelling' | 'completed' | 'failed' | 'cancelled'
export type ActivityTarget = 'download' | 'settings'

type DownloadQueueState = {
  total: number
  current: number
  completed: number
  failed: number
  cancelled?: number
  active: boolean
}

export type ActivityTask = {
  requestId?: string
  id: string
  kind: ActivityTaskKind
  state: ActivityTaskState
  title: string
  detail: string
  percent: number | null
  target: ActivityTarget
  cancellable: boolean
  queue?: DownloadQueueState
  updatedAt: number
}

type DownloadInput = {
  requestId?: string
  title: string
  queue?: Omit<DownloadQueueState, 'active'> & { active?: boolean }
}

type FinishInput = {
  state?: Extract<ActivityTaskState, 'completed' | 'failed' | 'cancelled'>
  detail?: string
}

const visibleTaskLimit = 3
const retainedTaskLimit = 8
const completionRetentionMs = 6_000

function clampPercent(value: number | null | undefined) {
  if (typeof value !== 'number' || !Number.isFinite(value)) return null
  return Math.max(0, Math.min(100, value))
}

function downloadStatusLabel(event: DownloadProgressEvent) {
  const labels: Record<DownloadProgressEvent['status'], string> = {
    starting: 'Preparing download…',
    downloading: 'Downloading…',
    processing: 'Processing file…',
    finished: 'Download complete',
    cancelled: 'Download cancelled',
    error: 'Download failed',
    log: '',
  }
  return labels[event.status] || event.message || ''
}

export const useActivityStore = defineStore('activity', () => {
  const tasks = ref<ActivityTask[]>([])
  const visibleTasks = computed(() => [...tasks.value]
    .sort((left, right) => {
      const leftActive = ['active', 'cancelling'].includes(left.state) ? 1 : 0
      const rightActive = ['active', 'cancelling'].includes(right.state) ? 1 : 0
      return rightActive - leftActive || right.updatedAt - left.updatedAt
    })
    .slice(0, visibleTaskLimit))

  let listening = false
  let unlisteners: Array<() => void> = []
  let downloadCanceller: (() => void | Promise<void>) | undefined
  const retentionTimers = new Map<string, number>()

  function findTask(id: string) {
    return tasks.value.find(task => task.id === id)
  }

  function clearRetention(id: string) {
    const timer = retentionTimers.get(id)
    if (timer) window.clearTimeout(timer)
    retentionTimers.delete(id)
  }

  function writeTask(next: ActivityTask) {
    const index = tasks.value.findIndex(task => task.id === next.id)
    if (index < 0) {
      tasks.value = [next, ...tasks.value].slice(0, retainedTaskLimit)
      return next
    }
    const copy = [...tasks.value]
    copy[index] = next
    tasks.value = copy
    return next
  }

  function upsertTask(id: string, patch: Partial<ActivityTask> & Pick<ActivityTask, 'kind' | 'title' | 'target'>) {
    const current = findTask(id)
    const next: ActivityTask = current
      ? { ...current, ...patch, id, updatedAt: Date.now() }
      : {
          id,
          state: 'active',
          detail: '',
          percent: null,
          cancellable: false,
          updatedAt: Date.now(),
          ...patch,
        }
    return writeTask(next)
  }

  function retainThenDismiss(task: ActivityTask) {
    clearRetention(task.id)
    retentionTimers.set(task.id, window.setTimeout(() => {
      const current = findTask(task.id)
      if (current && !['active', 'cancelling'].includes(current.state)) dismissTask(task.id)
    }, completionRetentionMs))
  }

  function beginDownload(input: DownloadInput) {
    clearRetention('download')
    const queue = input.queue
      ? { ...input.queue, active: input.queue.active ?? true }
      : undefined
    const percent = queue
      ? clampPercent(((queue.completed + queue.failed + (queue.cancelled || 0)) / Math.max(queue.total, 1)) * 100)
      : 0
    return upsertTask('download', {
      requestId: input.requestId,
      kind: 'download',
      title: input.title || 'Downloads',
      detail: queue ? `Queue ${queue.current}/${queue.total}` : 'Preparing download…',
      percent,
      target: 'download',
      cancellable: true,
      queue,
      state: 'active',
    })
  }

  function updateDownloadQueue(queue: Omit<DownloadQueueState, 'active'> & { active?: boolean }, title?: string, requestId?: string) {
    const current = findTask('download')
    return upsertTask('download', {
      requestId,
      kind: 'download',
      title: title || current?.title || 'Downloads',
      detail: `Queue ${queue.current}/${queue.total} · Completed ${queue.completed} · Failed ${queue.failed}${queue.cancelled ? ` · Cancelled ${queue.cancelled}` : ''}`,
      percent: clampPercent(((queue.completed + queue.failed + (queue.cancelled || 0)) / Math.max(queue.total, 1)) * 100),
      target: 'download',
      cancellable: true,
      queue: { ...queue, active: queue.active ?? true },
      state: 'active',
    })
  }

  function finishDownload(input: FinishInput = {}) {
    const current = findTask('download')
    if (!current) return
    const state = input.state ?? 'completed'
    const task = upsertTask('download', {
      requestId: undefined,
      kind: 'download',
      title: current.title,
      detail: input.detail || (state === 'completed' ? 'Download queue complete' : state === 'cancelled' ? 'Download queue cancelled' : 'Some downloads are unfinished'),
      percent: state === 'completed' ? 100 : current.percent,
      target: 'download',
      cancellable: false,
      queue: current.queue ? { ...current.queue, active: false } : undefined,
      state,
    })
    retainThenDismiss(task)
  }

  function handleDownloadProgress(event: DownloadProgressEvent) {
    if (event.status === 'log') return
    if (event.requestId && findTask('download')?.requestId !== event.requestId) return
    const current = findTask('download') ?? beginDownload({ title: 'Downloads' })
    const itemPercent = clampPercent(event.percent)
    const queue = current.queue
    const unknown = ['starting', 'processing'].includes(event.status) || itemPercent === null;
    const candidatePercent = unknown ? null : queue?.active
      ? itemPercent === null
        ? null
        : clampPercent(((queue.completed + queue.failed + (queue.cancelled || 0) + itemPercent / 100) / Math.max(queue.total, 1)) * 100)
      : itemPercent
    const percent = candidatePercent
    const terminal = ['finished', 'cancelled', 'error'].includes(event.status)
    const keepsQueueActive = Boolean(queue?.active)

    if (terminal && !keepsQueueActive) {
      finishDownload({
        state: event.status === 'finished' ? 'completed' : event.status === 'cancelled' ? 'cancelled' : 'failed',
        detail: downloadStatusLabel(event),
      })
      return
    }

    upsertTask('download', {
      kind: 'download',
      title: current.title,
      detail: terminal && queue ? 'Starting next download…' : downloadStatusLabel(event),
      percent,
      target: 'download',
      cancellable: true,
      queue,
      state: 'active',
    })
  }

  function handleSetupProgress(event: ToolInstallEvent) {
    const state = event.status === 'error' ? 'failed' : ['installed', 'completed'].includes(event.status) ? 'completed' : 'active'
    const task = upsertTask(`tool:${event.tool}`, {
      kind: 'setup',
      title: event.tool === 'yt-dlp' ? 'Install yt-dlp' : 'Install FFmpeg',
      detail: state === 'completed' ? 'Ready' : state === 'failed' ? 'Installation failed' : event.status === 'extracting' ? 'Installing…' : 'Downloading…',
      percent: clampPercent(event.percent),
      target: 'settings',
      cancellable: false,
      state,
    })
    if (state !== 'active') retainThenDismiss(task)
    if (state === 'completed') {
      // Completed installations do not affect subsequent downloads.
      task.cancellable = false
    }
  }

  async function startListening() {
    if (listening) return
    listening = true
    try {
      unlisteners = await Promise.all([
        onDownloadProgress(handleDownloadProgress),
        onToolInstallProgress(handleSetupProgress),
      ])
    } catch {
      listening = false
      unlisteners.forEach(unlisten => unlisten())
      unlisteners = []
    }
  }

  function stopListening() {
    unlisteners.forEach(unlisten => unlisten())
    unlisteners = []
    listening = false
    retentionTimers.forEach(timer => window.clearTimeout(timer))
    retentionTimers.clear()
  }

  function registerDownloadCanceller(handler: () => void | Promise<void>) {
    downloadCanceller = handler
    return () => {
      if (downloadCanceller === handler) downloadCanceller = undefined
    }
  }

  async function cancelTask(id: string) {
    const task = findTask(id)
    if (!task || !task.cancellable || task.state === 'cancelling') return
    upsertTask(id, {
      kind: task.kind,
      title: task.title,
      detail: 'Cancelling…',
      percent: task.percent,
      target: task.target,
      cancellable: false,
      queue: task.queue,
      state: 'cancelling',
    })
    try {
      if (task.kind === 'download') {
        if (downloadCanceller) await downloadCanceller()
        else await cancelDownload()
      }
    } catch {
      const failed = upsertTask(id, {
        kind: task.kind,
        title: task.title,
        detail: 'Unable to cancel task. Try again.',
        percent: task.percent,
        target: task.target,
        cancellable: false,
        queue: task.queue,
        state: 'failed',
      })
      retainThenDismiss(failed)
    }
  }

  function dismissTask(id: string) {
    clearRetention(id)
    tasks.value = tasks.value.filter(task => task.id !== id)
  }

  return {
    visibleTasks,
    beginDownload,
    updateDownloadQueue,
    finishDownload,
    startListening,
    stopListening,
    registerDownloadCanceller,
    cancelTask,
    dismissTask,
  }
})
