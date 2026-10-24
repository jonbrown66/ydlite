import { computed, ref, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import { cancelDownload, onDownloadProgress, pauseDownload, startDownload } from '@/api/tauri'
import { classifyDownloadError } from '@/lib/download'
import { useTasksStore } from './tasks'
import type { CookieSource, DownloadMode, DownloadProgressEvent, FileOptions } from '@/types'

export type DownloadItem = { url: string; title?: string }
export type DownloadOptions = {
  dir: string
  mode: DownloadMode
  formatId?: string | null
  cookieSource: CookieSource
  fileOptions?: FileOptions
  overwriteConfirmed?: boolean
}
export type DownloadState = 'idle' | 'running' | 'paused' | 'completed' | 'cancelled' | 'failed'
export type ItemState = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'

/** Downloads links in order. A single link is the common case and needs no queue. */
export const useDownloadStore = defineStore('download', () => {
  const state = ref<DownloadState>('idle')
  const items = shallowRef<DownloadItem[]>([])
  const itemStates = ref<ItemState[]>([])
  const options = shallowRef<DownloadOptions | null>(null)
  const current = ref(0)
  const progress = shallowRef<DownloadProgressEvent | null>(null)
  const filePath = ref('')
  const failed = ref(0)
  const error = ref<{ message: string; detail: string } | null>(null)
  const history = useTasksStore()

  const total = computed(() => items.value.length)
  const running = computed(() => state.value === 'running')
  const paused = computed(() => state.value === 'paused')
  const title = computed(() => items.value[current.value]?.title || items.value[current.value]?.url || '')
  const itemPercent = computed(() => {
    const value = progress.value?.percent
    return typeof value === 'number' && Number.isFinite(value) ? Math.min(100, Math.max(0, value)) : null
  })
  const percent = computed(() => {
    if (state.value === 'completed') return 100
    if (!total.value) return null
    const done = itemStates.value.slice(0, current.value).filter(status => status === 'completed').length
    return Math.min(99.9, ((done + (itemPercent.value ?? 0) / 100) / total.value) * 100)
  })
  const indeterminate = computed(() => running.value && itemPercent.value === null)
  const speed = computed(() => progress.value?.metrics?.speedBytes ?? null)

  let attemptId = ''
  let unlisten: (() => void) | undefined
  let listener: Promise<void> | undefined
  let stopping: 'pause' | 'cancel' | null = null

  function handle(event: DownloadProgressEvent) {
    if (event.requestId && event.requestId !== attemptId) return
    if (event.status === 'log' || event.status === 'paused' || event.status === 'cancelled' || event.status === 'error') return
    if (event.status === 'finished') {
      if (event.filePath) filePath.value = event.filePath
      progress.value = { ...event, percent: 100 }
      return
    }
    progress.value = event.status === 'processing'
      ? { ...event, percent: progress.value?.percent ?? null, metrics: event.metrics ?? progress.value?.metrics, total: event.total ?? progress.value?.total }
      : event
  }

  async function ensureListener() {
    if (!listener) listener = onDownloadProgress(handle).then(stop => { unlisten = stop })
    await listener
  }

  async function run() {
    if (running.value || !options.value || !items.value.length) return
    const job = options.value
    state.value = 'running'
    stopping = null
    error.value = null
    await ensureListener().catch(() => {})

    while (current.value < items.value.length) {
      // A cancelled batch stops immediately, even if the caller resolves later.
      if (stopping === 'cancel' && state.value === 'cancelled') return
      const index = current.value
      const item = items.value[index]!
      itemStates.value = itemStates.value.map((value, position) => position === index ? 'running' : value)
      progress.value = null
      filePath.value = ''
      attemptId = crypto.randomUUID()
      try {
        const result = await startDownload({
          requestId: attemptId,
          url: item.url,
          dir: job.dir,
          mode: job.mode,
          formatId: job.formatId || null,
          options: { cookieSource: job.cookieSource },
          fileOptions: job.fileOptions,
          overwriteConfirmed: job.overwriteConfirmed === true,
        })
        if (result.cancelled) {
          itemStates.value = itemStates.value.map((value, position) => position === index ? 'cancelled' : value)
          state.value = 'cancelled'
          return
        }
        if (result.paused) {
          itemStates.value = itemStates.value.map((value, position) => position === index ? 'pending' : value)
          state.value = 'paused'
          return
        }
        if (result.filePath) filePath.value = result.filePath
        itemStates.value = itemStates.value.map((value, position) => position === index ? 'completed' : value)
        if (result.filePath) {
          try {
            history.addDownload({ id: crypto.randomUUID(), title: item.title || item.url, url: item.url, filePath: result.filePath, completedAt: new Date().toISOString() })
          } catch { /* History is optional. */ }
        }
        current.value = index + 1
      } catch (value) {
        if (stopping === 'pause') {
          itemStates.value = itemStates.value.map((status, position) => position === index ? 'pending' : status)
          state.value = 'paused'
          return
        }
        if (stopping === 'cancel') {
          itemStates.value = itemStates.value.map((status, position) => position === index ? 'cancelled' : status)
          state.value = 'cancelled'
          return
        }
        failed.value += 1
        error.value = classifyDownloadError(value)
        itemStates.value = itemStates.value.map((status, position) => position === index ? 'failed' : status)
        current.value = index + 1
      }
    }
    state.value = failed.value ? 'failed' : 'completed'
    if (state.value === 'completed') progress.value = { ...progress.value, status: 'finished', percent: 100, filePath: filePath.value || null }
  }

  async function start(next: DownloadItem[], job: DownloadOptions) {
    if (running.value) return
    items.value = next
    options.value = job
    itemStates.value = next.map(() => 'pending')
    current.value = 0
    failed.value = 0
    progress.value = null
    filePath.value = ''
    error.value = null
    state.value = 'idle'
    await run()
  }

  async function pause() {
    if (!running.value) return
    stopping = 'pause'
    try { await pauseDownload() } catch (value) { stopping = null; error.value = classifyDownloadError(value) }
  }

  async function cancel() {
    // A paused transfer has no request in flight, so tell the backend to clear
    // its reservation and settle the state here; otherwise Cancel does nothing.
    if (paused.value) {
      try { await cancelDownload() } catch { /* The backend slot is already idle. */ }
      itemStates.value = itemStates.value.map(status => status === 'pending' ? 'cancelled' : status)
      state.value = 'cancelled'
      stopping = null
      return
    }
    if (!running.value) return
    stopping = 'cancel'
    try { await cancelDownload() } catch (value) { stopping = null; error.value = classifyDownloadError(value) }
  }

  async function resume() {
    if (!paused.value) return
    await run()
  }

  function retryFailed() {
    if (running.value) return
    items.value = items.value.filter((_, index) => itemStates.value[index] === 'failed')
    itemStates.value = items.value.map(() => 'pending')
    current.value = 0
    failed.value = 0
    error.value = null
    state.value = 'idle'
    void run()
  }

  function reset() {
    if (running.value) return
    state.value = 'idle'
    items.value = []
    itemStates.value = []
    options.value = null
    current.value = 0
    failed.value = 0
    progress.value = null
    filePath.value = ''
    error.value = null
  }

  function dispose() {
    unlisten?.()
    unlisten = undefined
    listener = undefined
  }

  return { state, items, itemStates, options, current, total, title, progress, filePath, failed, error, running, paused, percent, itemPercent, indeterminate, speed,
    start, pause, cancel, resume, retryFailed, reset, dispose }
})
