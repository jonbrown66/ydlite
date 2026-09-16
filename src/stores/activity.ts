import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  cancelDownload,
  cancelSubtitleTask,
  onDownloadProgress,
  onSubtitleProgress,
  onToolInstallProgress,
  onWhisperDownloadProgress,
} from '@/api/tauri'
import type { DownloadProgressEvent, SubtitleProgressEvent, ToolInstallEvent, WhisperDownloadEvent } from '@/types'

export type ActivityTaskKind = 'download' | 'subtitle' | 'setup'
export type ActivityTaskState = 'active' | 'cancelling' | 'completed' | 'failed' | 'cancelled'
export type ActivityTarget = 'download' | 'subtitles' | 'settings'

type DownloadQueueState = {
  total: number
  current: number
  completed: number
  failed: number
  active: boolean
}

export type ActivityTask = {
  id: string
  kind: ActivityTaskKind
  state: ActivityTaskState
  title: string
  detail: string
  percent: number | null
  target: ActivityTarget
  projectId?: string
  cancellable: boolean
  queue?: DownloadQueueState
  updatedAt: number
}

type DownloadInput = {
  title: string
  queue?: Omit<DownloadQueueState, 'active'> & { active?: boolean }
}

type SubtitleInput = {
  projectId: string
  title: string
  detail?: string
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
    starting: '正在准备下载',
    downloading: '正在下载',
    processing: '正在处理文件',
    finished: '下载已完成',
    cancelled: '下载已取消',
    error: '下载失败',
    log: '',
  }
  return event.message || labels[event.status]
}

function subtitleStageLabel(event: SubtitleProgressEvent) {
  if (event.message && event.stage === 'rate_limited') return event.message
  const labels: Record<string, string> = {
    extracting: '正在准备音频',
    uploading: '正在上传处理音频',
    transcribing: '正在识别语音',
    whisper_transcription: '正在识别语音',
    gemini_transcription: '正在识别语音',
    translate: '正在翻译字幕',
    bing_translation: '正在翻译字幕',
    custom_ai_translation: '正在翻译字幕',
    glm_translation: '正在翻译字幕',
    gemini_translation: '正在翻译字幕',
    polish: '正在自动校对',
    glm_polish: '正在自动校对',
    gemini_polish: '正在自动校对',
    burning: '正在生成字幕视频',
    completed: '正在整理结果',
    rate_limited: '正在等待服务恢复',
  }
  return labels[event.stage] || event.message || '正在处理字幕'
}

function setupTitle(event: ToolInstallEvent | WhisperDownloadEvent) {
  if ('tool' in event) return event.tool === 'yt-dlp' ? '安装 yt-dlp' : '安装 FFmpeg'
  return event.assetType === 'model' ? '下载 Whisper 模型' : '安装 Whisper 运行组件'
}

function setupTarget(event: ToolInstallEvent | WhisperDownloadEvent): ActivityTarget {
  return 'tool' in event ? 'download' : 'settings'
}

function setupTaskId(event: ToolInstallEvent | WhisperDownloadEvent) {
  return 'tool' in event ? `tool:${event.tool}` : `whisper:${event.assetType}:${event.id}`
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
      ? clampPercent(((queue.completed + queue.failed) / Math.max(queue.total, 1)) * 100)
      : 0
    return upsertTask('download', {
      kind: 'download',
      title: input.title || '视频下载',
      detail: queue ? `队列第 ${queue.current}/${queue.total} 项` : '正在准备下载',
      percent,
      target: 'download',
      cancellable: true,
      queue,
      state: 'active',
    })
  }

  function updateDownloadQueue(queue: Omit<DownloadQueueState, 'active'> & { active?: boolean }, title?: string) {
    const current = findTask('download')
    return upsertTask('download', {
      kind: 'download',
      title: title || current?.title || '视频下载',
      detail: `队列第 ${queue.current}/${queue.total} 项 · 完成 ${queue.completed} · 失败 ${queue.failed}`,
      percent: clampPercent(((queue.completed + queue.failed) / Math.max(queue.total, 1)) * 100),
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
      kind: 'download',
      title: current.title,
      detail: input.detail || (state === 'completed' ? '下载队列已完成' : state === 'cancelled' ? '下载队列已取消' : '下载队列有未完成的任务'),
      percent: state === 'completed' ? 100 : current.percent,
      target: 'download',
      cancellable: false,
      queue: current.queue ? { ...current.queue, active: false } : undefined,
      state,
    })
    retainThenDismiss(task)
  }

  function beginSubtitle(input: SubtitleInput) {
    const id = `subtitle:${input.projectId}`
    clearRetention(id)
    return upsertTask(id, {
      kind: 'subtitle',
      title: input.title || '自动字幕',
      detail: input.detail || '正在准备字幕任务',
      percent: 0,
      target: 'subtitles',
      projectId: input.projectId,
      cancellable: true,
      state: 'active',
    })
  }

  function finishSubtitle(projectId: string, input: FinishInput = {}) {
    const id = `subtitle:${projectId}`
    const current = findTask(id)
    if (!current) return
    const state = input.state ?? 'completed'
    const task = upsertTask(id, {
      kind: 'subtitle',
      title: current.title,
      detail: input.detail || (state === 'completed' ? '字幕任务已完成' : state === 'cancelled' ? '字幕任务已取消' : '字幕任务未完成'),
      percent: state === 'completed' ? 100 : current.percent,
      target: 'subtitles',
      projectId,
      cancellable: false,
      state,
    })
    retainThenDismiss(task)
  }

  function updateSubtitle(projectId: string, patch: Pick<Partial<ActivityTask>, 'title' | 'detail' | 'percent'>) {
    const id = `subtitle:${projectId}`
    const current = findTask(id)
    return upsertTask(id, {
      kind: 'subtitle',
      title: patch.title || current?.title || '自动字幕',
      detail: patch.detail || current?.detail || '正在处理字幕',
      percent: patch.percent ?? current?.percent ?? 0,
      target: 'subtitles',
      projectId,
      cancellable: true,
      state: 'active',
    })
  }

  function handleDownloadProgress(event: DownloadProgressEvent) {
    if (event.status === 'log') return
    const current = findTask('download') ?? beginDownload({ title: '视频下载' })
    const itemPercent = clampPercent(event.percent)
    const queue = current.queue
    const candidatePercent = queue?.active
      ? itemPercent === null
        ? current.percent
        : clampPercent(((queue.completed + queue.failed + itemPercent / 100) / Math.max(queue.total, 1)) * 100)
      : itemPercent ?? current.percent
    const percent = candidatePercent === null
      ? current.percent
      : Math.max(current.percent ?? 0, candidatePercent)
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
      detail: terminal && queue ? '正在继续队列中的下一项' : downloadStatusLabel(event),
      percent,
      target: 'download',
      cancellable: true,
      queue,
      state: 'active',
    })
  }

  function handleSubtitleProgress(event: SubtitleProgressEvent) {
    const current = findTask(`subtitle:${event.projectId}`)
    if (current && !['active', 'cancelling'].includes(current.state)) return
    updateSubtitle(event.projectId, {
      title: current?.title || '自动字幕',
      detail: subtitleStageLabel(event),
      percent: clampPercent(event.percent),
    })
  }

  function handleSetupProgress(event: ToolInstallEvent | WhisperDownloadEvent) {
    const isToolEvent = 'tool' in event
    const state = event.status === 'error' ? 'failed' : ['installed', 'completed'].includes(event.status) ? 'completed' : 'active'
    const task = upsertTask(setupTaskId(event), {
      kind: 'setup',
      title: setupTitle(event),
      detail: event.message || (state === 'completed' ? '准备完成' : '正在准备'),
      percent: clampPercent(event.percent),
      target: setupTarget(event),
      cancellable: false,
      state,
    })
    if (state !== 'active') retainThenDismiss(task)
    if (isToolEvent && state === 'completed') {
      // 已完成的工具安装不再影响后续下载任务。
      task.cancellable = false
    }
  }

  async function startListening() {
    if (listening) return
    listening = true
    try {
      unlisteners = await Promise.all([
        onDownloadProgress(handleDownloadProgress),
        onSubtitleProgress(handleSubtitleProgress),
        onToolInstallProgress(handleSetupProgress),
        onWhisperDownloadProgress(handleSetupProgress),
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
      detail: '正在取消任务…',
      percent: task.percent,
      target: task.target,
      projectId: task.projectId,
      cancellable: false,
      queue: task.queue,
      state: 'cancelling',
    })
    try {
      if (task.kind === 'download') {
        if (downloadCanceller) await downloadCanceller()
        else await cancelDownload()
      } else if (task.kind === 'subtitle' && task.projectId) {
        await cancelSubtitleTask(task.projectId)
      }
    } catch {
      const failed = upsertTask(id, {
        kind: task.kind,
        title: task.title,
        detail: '无法取消任务，请稍后重试。',
        percent: task.percent,
        target: task.target,
        projectId: task.projectId,
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
    beginSubtitle,
    updateSubtitle,
    finishSubtitle,
    startListening,
    stopListening,
    registerDownloadCanceller,
    cancelTask,
    dismissTask,
  }
})
