import { computed, ref, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import { cancelDownload, onDownloadProgress, parseVideo, startDownload } from '@/api/tauri'
import { classifyQueueError, collectQueueLinks, normalizeQueueUrl, queueLimit, queueStorageKey, readQueue } from '@/lib/downloadQueue'
import type { QueueJob } from '@/lib/downloadQueue'
import type { CookieSource, DownloadMode, DownloadProgressEvent, FileOptions } from '@/types'
import { readFileOptions } from '@/lib/downloadOptions'
import { useActivityStore } from './activity'
import { useTasksStore } from './tasks'

export type QueueOptions = { dir: string; mode: DownloadMode; formatId?: string | null; cookieSource: CookieSource; fileOptions?: FileOptions }
export const useDownloadQueueStore = defineStore('downloadQueue', () => {
  const jobs = ref<QueueJob[]>([])
  const storageError = ref('')
  const notice = ref('')
  const overwriteCount = ref(0)
  let overwriteResolver: ((confirmed: boolean) => void) | undefined
  function confirmOverwrite(confirmed: boolean) { overwriteCount.value = 0; overwriteResolver?.(confirmed); overwriteResolver = undefined }
  try { jobs.value = readQueue(localStorage.getItem(queueStorageKey)) }
  catch { storageError.value = 'Unable to read the saved queue. Its original data has been preserved.' }
  const running = ref(false)
  const activeId = ref('')
  const attemptId = ref('')
  const pauseRequested = ref(false)
  const progress = shallowRef<DownloadProgressEvent | null>(null)
  const logs = shallowRef<string[]>([])
  const activeJob = computed(() => jobs.value.find(job => job.id === activeId.value))
  const ready = computed(() => jobs.value.filter(job => ['pending', 'queued', 'interrupted'].includes(job.status)))
  const failed = computed(() => jobs.value.filter(job => job.status === 'failed'))
  const activity = useActivityStore()
  const history = useTasksStore()
  let listener: Promise<void> | undefined
  let unlisten: (() => void) | undefined
  let unregisterCanceller: (() => void) | undefined
  let cancelRequested = false
  let cancellation: Promise<void> | undefined
  let batch: string[] = []

  function commit(next: QueueJob[]) {
    if (storageError.value) throw new Error(storageError.value)
    try { localStorage.setItem(queueStorageKey, JSON.stringify({ version: 1, jobs: next })) }
    catch {
      storageError.value = 'Unable to save the queue. Downloading is paused; check available storage and try saving again.'
      pauseRequested.value = true
      throw new Error(storageError.value)
    }
    jobs.value = next
  }
  function patch(id: string, change: Partial<QueueJob>) {
    const next = jobs.value.map(job => job.id === id ? { ...job, ...change, updatedAt: new Date().toISOString() } : job)
    commit(next)
  }
  function saveAgain() {
    // Re-read failures must never overwrite the original stored data.
    if (storageError.value.startsWith('Unable to read')) return
    storageError.value = ''
    try { commit(jobs.value) } catch { /* Keep the visible error and in-memory queue. */ }
  }
  function makeJob(url: string, title: string, options: QueueOptions, analyzed: boolean): QueueJob {
    return { id: crypto.randomUUID(), url, title, dir: options.dir.trim(), mode: options.mode, formatId: options.formatId || null,
      cookieSource: { ...options.cookieSource }, status: analyzed ? 'queued' : 'pending', phase: analyzed ? 'download' : 'analyze',
      fileOptions: readFileOptions(options.fileOptions),
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() }
  }
  function add(entries: Array<{ url: string; title?: string; extractor?: string | null }>, options: QueueOptions, analyzed = true) {
    const seen = new Set(jobs.value.filter(job => job.status !== 'completed' && job.status !== 'cancelled').map(job => normalizeQueueUrl(job.url)))
    const added: QueueJob[] = []
    for (const entry of entries) {
      const url = normalizeQueueUrl(entry.url)
      if (!url || seen.has(url)) continue
      seen.add(url)
      added.push({ ...makeJob(url, entry.title || url, options, analyzed), extractor: entry.extractor })
    }
    if (jobs.value.length + added.length > queueLimit) throw new Error(`The queue can hold ${queueLimit} tasks. Remove finished tasks before importing more.`)
    if (added.length) commit([...jobs.value, ...added])
    return added.map(job => job.id)
  }
  function importLinks(text: string, options: QueueOptions) {
    const result = collectQueueLinks(text, jobs.value.filter(job => !['completed', 'cancelled'].includes(job.status)).map(job => job.url))
    const ids = add(result.urls.map(url => ({ url })), options, false)
    return { ids, duplicates: result.duplicates, invalid: result.invalid.length, invalidLines: result.invalid }
  }
  function handleProgress(event: DownloadProgressEvent) {
    if (!activeJob.value || activeJob.value.phase !== 'download') return
    if (event.requestId && event.requestId !== attemptId.value) return
    if (event.line) {
      // Keep diagnostics in memory only and publish a bounded buffer.
      logBuffer.push(event.line)
      if (logBuffer.length > 1000) logBuffer.shift()
      if (!logTimer) logTimer = window.setTimeout(() => { logs.value = [...logBuffer]; logTimer = undefined }, 200)
    }
    if (event.status === 'log') return
    progress.value = ['processing', 'finished'].includes(event.status)
      ? { ...event, metrics: event.metrics ?? progress.value?.metrics, total: event.total ?? progress.value?.total } : event
    if (event.status === 'cancelled') cancelRequested = true
    // Terminal events never advance the queue; the IPC promise owns completion.
  }
  const logBuffer: string[] = []
  let logTimer: number | undefined
  async function initialize() {
    if (!listener) listener = onDownloadProgress(handleProgress).then(stop => {
      unlisten = stop
      unregisterCanceller = activity.registerDownloadCanceller(stopQueue)
    }).catch(error => { listener = undefined; throw error })
    await listener
  }
  function dispose() {
    confirmOverwrite(false)
    unlisten?.(); unregisterCanceller?.(); listener = undefined
    if (logTimer) window.clearTimeout(logTimer)
  }
  function updateActivity() {
    const selected = jobs.value.filter(job => batch.includes(job.id))
    activity.updateDownloadQueue({ total: selected.length, current: Math.max(1, batch.indexOf(activeId.value) + 1),
      completed: selected.filter(job => job.status === 'completed').length,
      failed: selected.filter(job => job.status === 'failed').length,
      cancelled: selected.filter(job => job.status === 'cancelled').length, active: true }, activeJob.value?.title || 'Downloads', attemptId.value || undefined)
  }
  async function analyzeJob(id: string) {
    const job = jobs.value.find(job => job.id === id)!
    patch(id, { status: 'analyzing', phase: 'analyze', error: undefined })
    const info = await parseVideo({ url: job.url, options: { cookieSource: job.cookieSource } })
    if (cancelRequested) { patch(id, { status: 'cancelled' }); return }
    if (info.isPlaylist) {
      const entries = info.entries || []
      if (!entries.length) throw new Error('Empty playlist')
      // Expand atomically; preserve the source task until every entry fits in storage.
      const otherJobs = jobs.value.filter(item => item.id !== id)
      const seen = new Set(otherJobs.filter(item => !['completed', 'cancelled'].includes(item.status)).map(item => normalizeQueueUrl(item.url)))
      const expanded: QueueJob[] = []
      for (const entry of entries) {
        const url = normalizeQueueUrl(entry.url)
        if (!url || seen.has(url)) continue
        seen.add(url)
        expanded.push({ ...makeJob(url, entry.title, job, true), extractor: info.extractor })
      }
      if (otherJobs.length + expanded.length > queueLimit) throw new Error(`Playlist exceeds the ${queueLimit}-task queue limit. Select fewer entries in the single-link view.`)
      commit([...otherJobs, ...expanded])
      batch.splice(batch.indexOf(id), 1, ...expanded.map(item => item.id))
      notice.value = `Playlist expanded into ${expanded.length} new tasks. Duplicate links were skipped.`
    } else {
      const resolved = normalizeQueueUrl(info.resolvedUrl) || job.url
      if (jobs.value.some(other => other.id !== id && !['completed', 'cancelled'].includes(other.status) && normalizeQueueUrl(other.url) === resolved)) {
        commit(jobs.value.filter(other => other.id !== id))
        batch.splice(batch.indexOf(id), 1)
        notice.value = 'A link resolved to an existing task. The duplicate was skipped.'
        return
      }
      patch(id, { title: info.title, url: resolved, extractor: info.extractor,
        cookieSource: info.cookieSource || job.cookieSource, status: 'queued', phase: 'download', error: undefined })
    }
  }
  async function execute(ids: string[], analyzeOnly = false) {
    if (running.value || !ids.length || storageError.value) return
    running.value = true
    pauseRequested.value = false
    batch = [...ids]
    let index = 0
    let startupFailed = false
    try {
      await initialize()
      activity.beginDownload({ title: analyzeOnly ? 'Analyze links' : 'Downloads', queue: { total: batch.length, current: 1, completed: 0, failed: 0 } })
      const replacements = analyzeOnly ? 0 : jobs.value.filter(job => ids.includes(job.id) && !['completed', 'cancelled', 'failed'].includes(job.status) && job.fileOptions.conflict === 'overwrite').length
      if (replacements) {
        overwriteCount.value = replacements
        const approved = await new Promise<boolean>(resolve => { overwriteResolver = resolve })
        if (!approved) { pauseRequested.value = true; return }
      }
      while (index < batch.length && !pauseRequested.value) {
        const id = batch[index]!
        let job = jobs.value.find(item => item.id === id)
        if (!job || ['completed', 'cancelled', 'failed'].includes(job.status)) { index++; continue }
        activeId.value = id
        attemptId.value = ''
        cancelRequested = false
        cancellation = undefined
        progress.value = null
        logBuffer.length = 0; logs.value = []
        updateActivity()
        try {
          if (job.phase === 'analyze') {
            await analyzeJob(id)
            job = jobs.value.find(item => item.id === id)
            // Playlist replacement inserted new IDs at this position.
            if (!job) continue
          }
          if (!analyzeOnly && !cancelRequested && ['queued', 'interrupted'].includes(job.status)) {
            if (!job.dir.trim()) throw new Error('Choose a save folder before downloading.')
            patch(id, { status: 'downloading', phase: 'download', error: undefined, filePath: undefined })
            progress.value = { status: 'starting' }
            attemptId.value = crypto.randomUUID()
            updateActivity()
            const result = await startDownload({ requestId: attemptId.value, url: job.url, dir: job.dir, mode: job.mode, formatId: job.formatId, options: { cookieSource: job.cookieSource }, fileOptions: job.fileOptions, overwriteConfirmed: job.fileOptions.conflict === 'overwrite' })
            // A late cancellation command must settle before the next process starts.
            try { await cancellation } catch { /* A failed cancel must not turn a completed download into a failed one. */ }
            // IPC is authoritative; events can arrive late or be lost on reload.
            if (result?.cancelled ?? cancelRequested) patch(id, { status: 'cancelled' })
            else {
              const filePath = result?.filePath || progress.value?.filePath || undefined
              try { patch(id, { status: 'completed', filePath, skipped: result?.skipped === true, error: undefined }) }
              catch {
                jobs.value = jobs.value.map(item => item.id === id ? { ...item, status: 'completed', filePath, error: undefined } : item)
                pauseRequested.value = true
                notice.value = 'Download complete, but the queue could not be saved. Save again before continuing.'
              }
              if (filePath) {
                try { history.addDownload({ id, title: job.title, extractor: job.extractor, url: job.url, filePath, completedAt: new Date().toISOString() }) }
                catch { notice.value = 'Download complete. Unable to save history; the file and queue record are retained.' }
              }
            }
          }
        } catch (error) {
          try { await cancellation } catch { /* Cancellation already reports its own error. */ }
          const status = cancelRequested ? 'cancelled' : 'failed'
          // If persistence fails after IPC completion, retain the actual outcome in memory.
          const change = { status, error: cancelRequested ? undefined : classifyQueueError(error) } as Partial<QueueJob>
          if (storageError.value) {
            pauseRequested.value = true
            jobs.value = jobs.value.map(item => item.id === id ? { ...item, status: cancelRequested ? 'cancelled' : 'interrupted' } : item)
          } else patch(id, change)
          progress.value = { ...progress.value, status: cancelRequested ? 'cancelled' : 'error' }
        }
        index++
      }
    } catch (error) {
      startupFailed = true
      notice.value = error instanceof Error ? error.message : 'Unable to start the queue. Try again.'
    }
    finally {
      const selected = jobs.value.filter(job => batch.includes(job.id))
      const failures = selected.filter(job => job.status === 'failed').length
      const cancelled = selected.filter(job => job.status === 'cancelled').length
      updateActivity()
      activity.finishDownload({ state: startupFailed ? 'failed' : pauseRequested.value ? 'cancelled' : failures ? 'failed' : cancelled ? 'cancelled' : 'completed',
        detail: startupFailed ? 'Unable to start. Remaining tasks are saved.' : pauseRequested.value ? 'Queue paused. Remaining tasks are saved.' : analyzeOnly ? 'Link analysis complete' : `${selected.filter(job => job.status === 'completed').length} completed · ${failures} failed · ${cancelled} cancelled` })
      activeId.value = ''; attemptId.value = ''; running.value = false
    }
  }
  async function run(ids = ready.value.map(job => job.id)) { await execute(ids) }
  async function analyze(ids: string[], cookieSource?: CookieSource) {
    if (running.value) return
    commit(jobs.value.map(job => ids.includes(job.id) && job.status !== 'completed'
      ? { ...job, status: 'pending', phase: 'analyze', error: undefined, cookieSource: cookieSource ? { ...cookieSource } : job.cookieSource } : job))
    await execute(ids, true)
  }
  async function retry(ids: string[]) {
    if (running.value) return
    const retryIds = jobs.value.filter(job => ids.includes(job.id) && ['failed', 'cancelled', 'interrupted'].includes(job.status)).map(job => job.id)
    commit(jobs.value.map(job => retryIds.includes(job.id) ? { ...job, status: job.phase === 'analyze' ? 'pending' : 'queued', error: undefined } : job))
    await execute(retryIds)
  }
  async function cancel(id: string) {
    const job = jobs.value.find(item => item.id === id)
    if (!job || ['completed', 'cancelled', 'failed'].includes(job.status)) return
    if (id !== activeId.value) { patch(id, { status: 'cancelled' }); return }
    if (cancelRequested) return
    if (job.phase === 'analyze') {
      cancelRequested = true
      patch(id, { status: 'cancelling' })
      return // The parser has no cancellation API; discard its result when it settles.
    }
    if (['finished', 'error'].includes(progress.value?.status || '')) return
    patch(id, { status: 'cancelling' })
    cancelRequested = true
    cancellation = cancelDownload().catch(error => {
      cancelRequested = false
      if (activeId.value === id && jobs.value.find(item => item.id === id)?.status === 'cancelling') patch(id, { status: 'downloading' })
      throw error
    })
    // The worker observes this promise as well, without an unhandled rejection.
    void cancellation.catch(() => {})
    await cancellation
  }
  function pause() { pauseRequested.value = true }
  async function stopQueue() { pause(); if (activeId.value) await cancel(activeId.value) }
  function remove(id: string) { if (id !== activeId.value) commit(jobs.value.filter(job => job.id !== id)) }
  function clearCompleted() { commit(jobs.value.filter(job => job.status !== 'completed')) }
  function changeOptions(id: string, options: QueueOptions) {
    if (running.value) return
    patch(id, { dir: options.dir.trim(), mode: options.mode, formatId: options.formatId || null, cookieSource: { ...options.cookieSource }, fileOptions: readFileOptions(options.fileOptions) })
  }
  return { jobs, running, activeId, attemptId, activeJob, pauseRequested, ready, failed, progress, logs, notice, storageError, overwriteCount, confirmOverwrite,
    add, importLinks, initialize, dispose, run, analyze, retry, cancel, pause, stopQueue, remove, clearCompleted, changeOptions, saveAgain }
})
