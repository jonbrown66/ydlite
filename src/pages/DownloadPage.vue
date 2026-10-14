<script setup lang="ts">
import { computed, nextTick, onActivated, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import checkIcon from 'iconoir/icons/check.svg?url'
import checkSquareIcon from 'iconoir/icons/check-square.svg?url'
import copyIcon from 'iconoir/icons/copy.svg?url'
import cookieIcon from 'iconoir/icons/cookie.svg?url'
import downloadIcon from 'iconoir/icons/download.svg?url'
import folderIcon from 'iconoir/icons/folder.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import navArrowRightIcon from 'iconoir/icons/nav-arrow-right.svg?url'
import openNewWindowIcon from 'iconoir/icons/open-new-window.svg?url'
import refreshIcon from 'iconoir/icons/refresh.svg?url'
import settingsIcon from 'iconoir/icons/settings.svg?url'
import trashIcon from 'iconoir/icons/trash.svg?url'
import xmarkIcon from 'iconoir/icons/xmark.svg?url'
import {
  checkDependencies,
  checkYtdlpUpdate,
  getToolsDirectory,
  installMissingTools,
  onDownloadProgress,
  onToolInstallProgress,
  openParentFolder,
  openPath,
  parseVideo,
  selectCookiesFile,
  selectDirectory,
  updateYtdlp,
} from '../api/tauri'
import { AppSelect } from '@/components/ui/select'
import AppDialog from '@/components/ui/AppDialog.vue'
import AppCollapsible from '@/components/ui/AppCollapsible.vue'
import DisclosureChevron from '@/components/ui/DisclosureChevron.vue'
import PageHeader from '@/components/ui/PageHeader.vue'
import { usePreferencesStore } from '@/stores/preferences'
import { useDownloadQueueStore } from '@/stores/downloadQueue'
import DownloadQueue from '@/components/DownloadQueue.vue'
import DownloadOptionsFields from '@/components/DownloadOptionsFields.vue'
import { normalizeQueueUrl, classifyQueueError } from '@/lib/downloadQueue'
import { formatLabel, readFileOptions } from '@/lib/downloadOptions'
import type { CookieSource, DependencyStatus, DownloadProgressEvent, ToolInstallEvent, VideoInfo, VideoFormat, DownloadMode } from '../types'

const preferences = usePreferencesStore()
const downloadQueue = useDownloadQueueStore()
const icons = {
  check: checkIcon,
  checkSquare: checkSquareIcon,
  copy: copyIcon,
  cookie: cookieIcon,
  download: downloadIcon,
  folder: folderIcon,
  mediaVideo: mediaVideoIcon,
  navArrowRight: navArrowRightIcon,
  openNewWindow: openNewWindowIcon,
  refresh: refreshIcon,
  settings: settingsIcon,
  trash: trashIcon,
  xmark: xmarkIcon,
} as const

type IconName = keyof typeof icons

type DownloadTab = 'single' | 'multiple'
const downloadTab = ref<DownloadTab>('single')
const downloadTabs: Array<{ value: DownloadTab; label: string }> = [
  { value: 'single', label: 'Single link' },
  { value: 'multiple', label: 'Multiple links' },
]
const queueWorkspace = ref<InstanceType<typeof DownloadQueue> | null>(null)
function onDownloadTabKeydown(event: KeyboardEvent, index: number) {
  const key = event.key
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(key)) return
  event.preventDefault()
  const target = key === 'Home' ? 0 : key === 'End' ? downloadTabs.length - 1
    : (index + (key === 'ArrowRight' ? 1 : -1) + downloadTabs.length) % downloadTabs.length
  const tab = downloadTabs[target]!
  downloadTab.value = tab.value
  document.getElementById(`downloads-tab-${tab.value}`)?.focus()
}

const url = ref('')
const saveDir = ref('')
const defaultDir = ref('')
const setAsDefault = ref(false)
const cookiesFilePath = ref('')
const deps = ref<DependencyStatus | null>(null)
const video = ref<VideoInfo | null>(null)
const loadingDeps = ref(false)
const toolsOpen = ref(false)
const parsing = ref(false)
const downloading = computed(() => downloadQueue.running)
const progress = ref<DownloadProgressEvent | null>(null)
const errorMessage = ref('')
const errorDetail = ref('')
const logs = shallowRef<string[]>([])
const hasLogs = ref(false)
const logBuffer: string[] = []
let logFlushTimer: number | undefined
const completedFilePath = ref('')
const displayPercent = ref(0)
const cancelling = computed(() => downloadQueue.activeJob?.status === 'cancelling')
const logsOpen = ref(false)
const urlInput = ref<HTMLInputElement | null>(null)
const dragActive = ref(false)
const toolsDir = ref('')
const showInstallConfirm = ref(false)
const installing = ref(false)
const updatingYtdlp = ref(false)
const installEvents = ref<Record<string, ToolInstallEvent>>({})
const detailsSection = ref<HTMLElement | null>(null)
let unlistenDownloadProgress: (() => void) | undefined
let unlistenToolProgress: (() => void) | undefined
const lastParsedUrl = ref('')
const clipboardTipUrl = ref('')
const showClipboardTip = ref(false)

const selectedFormatId = ref<string>('best')
const showDetailedFormats = ref(false)
const fileOptions = ref(readFileOptions(preferences.settings.fileOptions))
const selectedAudioId = computed({ get: () => fileOptions.value.audioFormatId || '', set: (value: string) => { fileOptions.value.audioFormatId = value || null } })
const audioOptions = computed(() => [{ value: '', label: 'Automatic audio track' }, ...(video.value?.formats || []).filter(f => f.vcodec === 'none' && f.acodec && f.acodec !== 'none').map(f => ({ value: f.formatId, label: formatLabel(f) }))])
watch(selectedFormatId, () => { fileOptions.value.audioFormatId = null })

const selectedEntries = ref<Record<string, boolean>>({})

const queueActive = computed(() => downloadQueue.running)
const currentDownloadUrl = ref('')
const failedEntries = computed(() => downloadQueue.failed.filter(job => video.value?.entries?.some(entry => normalizeQueueUrl(entry.url) === normalizeQueueUrl(job.url))))

const missingTools = computed(() => {
  const missing: string[] = []
  if (deps.value && !deps.value.ytdlp_ok) missing.push('yt-dlp')
  if (deps.value && !deps.value.ffmpeg_ok) missing.push('ffmpeg')
  return missing
})

const canParse = computed(() => Boolean(url.value.trim() && !parsing.value && !downloading.value))
const canDownload = computed(() => Boolean(video.value && saveDir.value && !parsing.value && !downloading.value && !hasMissingTools.value))
const selectedCount = computed(() => video.value?.entries?.filter(entry => selectedEntries.value[entry.url]).length ?? 0)
const hasMissingTools = computed(() => missingTools.value.length > 0)
const depsChecked = computed(() => deps.value !== null)
const toolStatusText = computed(() => {
  if (loadingDeps.value) return 'Checking…'
  if (!depsChecked.value) return 'Not checked'
  if (deps.value?.ytdlp_ok && deps.value?.ffmpeg_ok) return 'Tools ready'
  return `Install ${missingTools.value.join(', ')}`
})

const progressPercent = computed(() => {
  return Math.max(0, Math.min(100, displayPercent.value))
})

function resetDisplayedProgress() { displayPercent.value = 0 }

function updateDisplayedProgress(percent: number, terminal = false) {
  if (!Number.isFinite(percent)) return
  displayPercent.value = terminal ? 100 : Math.max(0, Math.min(99.8, percent))
}

const progressIndeterminate = computed(() => Boolean(progress.value && (
  ['starting', 'processing'].includes(progress.value.status)
  || (progress.value.status === 'downloading' && !Number.isFinite(progress.value.percent))
)))
const progressTrackPercent = computed(() => progressIndeterminate.value ? 0 : progressPercent.value)

const progressStatusText = computed(() => {
  if (cancelling.value) return 'Cancelling…'
  switch (progress.value?.status) {
    case 'starting':
      return 'Preparing…'
    case 'downloading':
      return progress.value?.message?.startsWith('Downloading stream') ? progress.value.message : 'Downloading…'
    case 'processing':
      return 'Processing media…'
    case 'finished':
      return 'Completed'
    case 'cancelled':
      return 'Cancelled'
    case 'error':
      return 'Download failed'
    default:
      return 'Ready'
  }
})

const durationText = computed(() => {
  if (!video.value?.duration) return 'Unknown duration'
  const total = video.value.duration
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = Math.floor(total % 60)
  return hours > 0
    ? `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
    : `${minutes}:${seconds.toString().padStart(2, '0')}`
})

const availableFormats = computed(() => (video.value?.formats || [])
  .filter(fmt => fmt.vcodec && fmt.vcodec !== 'none')
  .map(fmt => ({ label: formatLabel(fmt), formatId: fmt.formatId, height: fmt.height || 0 }))
  .sort((a, b) => b.height - a.height))

const selectedFormat = computed(() => {
  if (!video.value?.formats) return null
  return video.value.formats.find(f => f.formatId === selectedFormatId.value) || null
})

function formatBytes(bytes: number) {
  const units = ['B', 'KiB', 'MiB', 'GiB']
  const unit = Math.min(3, Math.max(0, Math.floor(Math.log2(Math.max(1, bytes)) / 10)))
  return `${(bytes / 1024 ** unit).toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`
}

function formatSize(format: VideoFormat | undefined | null) {
  if (!format) return null
  const size = format.filesize || format.filesizeApprox
  if (size && size > 0) return size
  if (format.bitrateKbps && video.value?.duration) return format.bitrateKbps * 1000 / 8 * video.value.duration
  return null
}

const estimatedDownloadBytes = computed(() => {
  if (video.value?.isPlaylist) return null
  // yt-dlp returns formats in ascending preference order. This is a preview;
  // the actual requested streams and byte totals take over during download.
  const audio = video.value?.formats?.filter(f => f.vcodec === 'none' && f.acodec && f.acodec !== 'none' && (!fileOptions.value.audioFormatId || f.formatId === fileOptions.value.audioFormatId)) || []
  if (selectedFormatId.value === 'bestaudio') return formatSize(audio.at(-1))
  const candidates = video.value?.formats?.filter(f => f.vcodec && f.vcodec !== 'none' && (selectedFormatId.value !== 'p720' || (f.height || 0) <= 720)) || []
  const chosen = ['best', 'p720'].includes(selectedFormatId.value)
    ? (fileOptions.value.quality === 'quality' ? candidates.at(-1) : candidates.filter(f => f.ext === 'mp4').at(-1) || candidates.at(-1)) : selectedFormat.value
  const videoSize = formatSize(chosen)
  if (!videoSize || !chosen) return null
  if (chosen.acodec && chosen.acodec !== 'none') return videoSize
  const audioSize = formatSize(audio.filter(f => f.ext === 'm4a').at(-1) || audio.at(-1))
  return audioSize ? videoSize + audioSize : null
})
const downloadSizeText = computed(() => {
  const metrics = progress.value?.metrics
  const total = metrics?.totalBytes
  // An explicit unknown total from the downloader overrides a metadata estimate.
  const totalText = metrics
    ? (total ? `${metrics.totalIsEstimate ? '≈ ' : ''}${formatBytes(total)}` : 'Unknown size')
    : progress.value?.total || (estimatedDownloadBytes.value ? `≈ ${formatBytes(estimatedDownloadBytes.value)}` : 'Unknown size')
  return metrics?.downloadedBytes != null ? `${formatBytes(metrics.downloadedBytes)} / ${totalText}` : totalText
})
const remainingTimeText = computed(() => {
  switch (progress.value?.status) {
    case 'starting': return 'Estimating…'
    case 'processing': return 'Finishing…'
    case 'finished': return 'Done'
    case 'cancelled': case 'error': return '—'
    default: return progress.value?.eta ? `≈ ${progress.value.eta}` : 'Estimating…'
  }
})
const formatOptions = computed(() => [
  { value: 'best', label: 'Best available · prefer MP4' },
  { value: 'p720', label: 'Up to 720p · prefer MP4' },
  { value: 'bestaudio', label: 'Audio only · MP3' },
  ...(showDetailedFormats.value ? availableFormats.value : availableFormats.value.filter(format => format.formatId === selectedFormatId.value)).map(format => ({
    value: format.formatId,
    label: format.label,
  })),
])

function iconStyle(name: IconName) {
  return { '--icon-url': `url("${icons[name]}")` }
}

onMounted(() => {
  window.addEventListener('focus', checkClipboard)
  defaultDir.value = preferences.settings.defaultDir
  saveDir.value = defaultDir.value
  setAsDefault.value = Boolean(defaultDir.value)
  cookiesFilePath.value = preferences.settings.cookiesFilePath
  void Promise.all([
    onDownloadProgress(handleProgress),
    onToolInstallProgress(handleToolInstallProgress),
  ]).then(([downloadUnlisten, toolUnlisten]) => {
    unlistenDownloadProgress = downloadUnlisten
    unlistenToolProgress = toolUnlisten
  }).catch(() => {
    // Event listeners are optional until a task starts.
  })
  void refreshDependencies().then(() => {
    if (deps.value?.ytdlp_ok) void refreshYtdlpUpdate()
  })
})

watch(() => preferences.settings, (next, previous) => {
  if (downloading.value || queueActive.value) return
  if (!saveDir.value || saveDir.value === previous.defaultDir) saveDir.value = next.defaultDir
  defaultDir.value = next.defaultDir
  setAsDefault.value = Boolean(next.defaultDir && saveDir.value === next.defaultDir)
  cookiesFilePath.value = next.cookiesFilePath
  if (JSON.stringify(fileOptions.value) === JSON.stringify(previous.fileOptions)) fileOptions.value = readFileOptions(next.fileOptions)
  if (!next.clipboardSuggestions) showClipboardTip.value = false
})
watch(() => downloadQueue.running, running => {
  if (running) return
  const job = [...downloadQueue.jobs].reverse().find(job => normalizeQueueUrl(job.url) === normalizeQueueUrl(currentDownloadUrl.value))
  if (job && ['failed', 'cancelled'].includes(job.status)) {
    progress.value = { ...progress.value, status: job.status === 'failed' ? 'error' : 'cancelled' }
    if (job.error) errorMessage.value = job.error.message
  }
  else if (job?.status === 'completed') {
    progress.value = { ...progress.value, status: 'finished', percent: 100 }
    updateDisplayedProgress(100, true)
    completedFilePath.value = job.filePath || ''
  } else progress.value = null
})
onActivated(() => { if (deps.value && !downloading.value) void refreshDependencies() })

onBeforeUnmount(() => {
  resetLogs()
  unlistenDownloadProgress?.()
  unlistenToolProgress?.()
  window.removeEventListener('focus', checkClipboard)
})

async function refreshDependencies() {
  loadingDeps.value = true
  try {
    deps.value = await checkDependencies()
  } catch (error) {
    showError(error, 'Unable to check tools. Try again.')
  } finally {
    loadingDeps.value = false
  }
}

async function handleCheckTools() {
  clearError()
  await refreshDependencies()
  if (deps.value?.ytdlp_ok) {
    void refreshYtdlpUpdate()
  }
}

async function refreshYtdlpUpdate() {
  try {
    const update = await checkYtdlpUpdate()
    if (deps.value) {
      deps.value = { ...deps.value, ...update }
    }
  } catch {
    // Update checks should not block the main flow.
  }
}

async function checkClipboard() {
  if (downloadTab.value !== 'single') return
  if (!preferences.settings.clipboardSuggestions) return
  try {
    const text = await navigator.clipboard.readText()
    const trimmed = text.trim()
    if (/^https?:\/\/\S+/.test(trimmed)) {
      if (trimmed !== url.value && trimmed !== lastParsedUrl.value && !parsing.value && !downloading.value) {
        clipboardTipUrl.value = trimmed
        showClipboardTip.value = true
      }
    }
  } catch (e) {
    // Ignore clipboard permission restrictions.
  }
}

function selectedCookieSource(): CookieSource {
  const path = cookiesFilePath.value.trim()
  return path ? { type: 'file', path } : { type: 'none' }
}

async function handleSelectCookiesFile() {
  const selected = await selectCookiesFile()
  if (selected) {
    cookiesFilePath.value = selected
    preferences.update({ cookiesFilePath: selected })
  }
}

function clearCookiesFile() {
  cookiesFilePath.value = ''
  preferences.update({ cookiesFilePath: '' })
}

function handleUseClipboardUrl() {
  url.value = clipboardTipUrl.value
  showClipboardTip.value = false
  void handleParse()
}

async function scrollToVideoDetails() {
  await nextTick()
  if (downloadTab.value !== 'single') return
  detailsSection.value?.scrollIntoView({
    behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
    block: 'start',
  })
}

async function handleParse() {
  if (!canParse.value) return
  clearError()
  video.value = null
  currentDownloadUrl.value = ''
  completedFilePath.value = ''
  progress.value = null
  resetDisplayedProgress()
  parsing.value = true
  showClipboardTip.value = false
  selectedEntries.value = {}
  selectedFormatId.value = ''
  try {
    const result = await parseVideo({ url: url.value, options: { cookieSource: selectedCookieSource() } })
    video.value = result
    fileOptions.value.audioFormatId = null
    if (result.resolvedUrl && result.resolvedUrl !== url.value.trim()) {
      url.value = result.resolvedUrl
    }
    lastParsedUrl.value = url.value
    
    if (result.isPlaylist && result.entries) {
      result.entries.forEach(e => {
        selectedEntries.value[e.url] = true
      })
    }
    selectedFormatId.value = preferences.settings.defaultQuality === 'mp3' ? 'bestaudio' : preferences.settings.defaultQuality
    await scrollToVideoDetails()
  } catch (error) {
    showError(error, 'Unable to analyze this URL. Check that it is accessible and supported.')
  } finally {
    parsing.value = false
  }
}

async function handleSelectDir() {
  const selected = await selectDirectory()
  if (selected) {
    saveDir.value = selected
    if (setAsDefault.value) saveDefaultDir(selected)
  }
}

function handleDefaultToggle() {
  if (setAsDefault.value && saveDir.value) {
    saveDefaultDir(saveDir.value)
  } else {
    defaultDir.value = ''
    preferences.update({ defaultDir: '' })
  }
}

function queueOptions() {
  const selected = selectedFormatId.value
  const mode: DownloadMode = selected === 'bestaudio' ? 'mp3' : selected === 'p720' ? 'p720' : selected === 'best' ? 'best' : 'custom'
  return { dir: saveDir.value, mode, formatId: mode === 'custom' ? selected : null, cookieSource: video.value?.cookieSource || selectedCookieSource(), fileOptions: { ...fileOptions.value } }
}

async function handleDownload() {
  if (!canDownload.value) return
  clearError(); resetLogs(); completedFilePath.value = ''; resetDisplayedProgress()
  progress.value = { status: 'starting' }
  const target = video.value?.resolvedUrl || url.value
  currentDownloadUrl.value = normalizeQueueUrl(target) || target
  try {
    if (setAsDefault.value) saveDefaultDir(saveDir.value)
    const ids = downloadQueue.add([{ url: target, title: video.value?.title, extractor: video.value?.extractor }], queueOptions())
    const existing = downloadQueue.jobs.find(job => normalizeQueueUrl(job.url) === normalizeQueueUrl(target) && !['completed', 'cancelled'].includes(job.status))
    if (!ids.length && existing) {
      downloadQueue.changeOptions(existing.id, queueOptions())
      if (existing.status === 'failed') await downloadQueue.retry([existing.id])
      else await downloadQueue.run([existing.id])
    } else await downloadQueue.run(ids)
  } catch (error) { progress.value = { status: 'error' }; showError(error, 'Unable to save or start this download.') }
}

async function handleBatchDownload(retryFailed = false) {
  if (!canDownload.value || !video.value?.entries) return
  clearError(); resetLogs(); completedFilePath.value = ''; resetDisplayedProgress()
  progress.value = null
  try {
    if (setAsDefault.value) saveDefaultDir(saveDir.value)
    if (retryFailed) await downloadQueue.retry(failedEntries.value.map(job => job.id))
    else {
      const selected = video.value.entries.filter(entry => selectedEntries.value[entry.url])
      const ids = downloadQueue.add(selected, queueOptions())
      const selectedUrls = new Set(selected.map(entry => normalizeQueueUrl(entry.url)))
      const existing = downloadQueue.ready.filter(job => selectedUrls.has(normalizeQueueUrl(job.url))).map(job => job.id)
      await downloadQueue.run([...new Set([...ids, ...existing])])
    }
  } catch (error) { showError(error, 'Unable to save or start the download queue.') }
}

function selectAllEntries(val: boolean) {
  if (downloading.value || !video.value?.entries) return
  video.value.entries.forEach(entry => { selectedEntries.value[entry.url] = val })
}
async function cancelQueue() {
  try { await downloadQueue.stopQueue() }
  catch (error) { showError(error, 'Unable to cancel. Try again.') }
}
async function handleCancel() {
  if (cancelling.value || !downloadQueue.activeId) return
  try { await downloadQueue.cancel(downloadQueue.activeId) }
  catch (error) { showError(error, 'Unable to cancel. Try again.') }
}

async function handleInstallTools() {
  clearError()
  installing.value = true
  installEvents.value = {}
  try {
    await installMissingTools()
    await refreshDependencies()
    if (deps.value?.ytdlp_ok) void refreshYtdlpUpdate()
    showInstallConfirm.value = false
  } catch (error) {
    showError(error, 'Tool installation failed. See details.')
  } finally {
    installing.value = false
  }
}

async function handleUpdateYtdlp() {
  clearError()
  updatingYtdlp.value = true
  installEvents.value = {}
  try {
    await updateYtdlp()
    await refreshDependencies()
    if (deps.value?.ytdlp_ok) void refreshYtdlpUpdate()
  } catch (error) {
    showError(error, 'Unable to update yt-dlp. Check your connection.')
  } finally {
    updatingYtdlp.value = false
  }
}

async function openInstallConfirm() {
  if (!toolsDir.value) {
    try {
      toolsDir.value = await getToolsDirectory()
    } catch (error) {
      showError(error, 'Tool folder unavailable.')
      return
    }
  }
  showInstallConfirm.value = true
}

async function handleOpenFile() {
  if (!completedFilePath.value) return
  try {
    await openPath(completedFilePath.value)
  } catch (error) {
    showError(error, 'Unable to open file.')
  }
}

async function handleOpenParentFolder() {
  if (!completedFilePath.value) return
  try {
    await openParentFolder(completedFilePath.value)
  } catch (error) {
    showError(error, 'Unable to open folder.')
  }
}

function flushLogs() {
  if (logFlushTimer !== undefined) window.clearTimeout(logFlushTimer)
  logFlushTimer = undefined
  if (logsOpen.value) logs.value = [...logBuffer]
}

function resetLogs() {
  if (logFlushTimer !== undefined) window.clearTimeout(logFlushTimer)
  logFlushTimer = undefined
  logBuffer.length = 0
  logs.value = []
  hasLogs.value = false
  logsOpen.value = false
}

function handleLogsToggle(event: Event) {
  logsOpen.value = (event.target as HTMLDetailsElement).open
  flushLogs()
}

function handleProgress(event: DownloadProgressEvent) {
  if (event.requestId && event.requestId !== downloadQueue.attemptId) return
  if (downloadQueue.activeJob && normalizeQueueUrl(downloadQueue.activeJob.url) !== normalizeQueueUrl(currentDownloadUrl.value)) return
  if (event.line) {
    logBuffer.push(event.line)
    if (logBuffer.length > 1000) logBuffer.splice(0, logBuffer.length - 1000)
    if (!hasLogs.value) hasLogs.value = true
    // Collapsed logs do not trigger a render for every downloader message.
    if (logsOpen.value && logFlushTimer === undefined) {
      logFlushTimer = window.setTimeout(flushLogs, 200)
    }
  }
  if (event.status === 'log') return

  const nextEvent = event.status === 'finished' ? { ...event, percent: event.percent ?? 100 } : event
  progress.value = nextEvent.status === 'processing' || nextEvent.status === 'finished'
    ? { ...nextEvent, metrics: nextEvent.metrics ?? progress.value?.metrics, total: nextEvent.total ?? progress.value?.total }
    : nextEvent
  if (nextEvent.status === 'finished') {
    updateDisplayedProgress(100, true)
  } else if (typeof nextEvent.percent === 'number') {
    updateDisplayedProgress(nextEvent.percent)
  }
  if (event.filePath) completedFilePath.value = event.filePath


}

function handleDragOver(event: DragEvent) {
  event.preventDefault()
  dragActive.value = true
}

function handleDragLeave(event: DragEvent) {
  if (event.currentTarget === event.target) dragActive.value = false
}

function handleDrop(event: DragEvent) {
  event.preventDefault()
  dragActive.value = false
  if (parsing.value || downloading.value || queueActive.value) return
  const text = event.dataTransfer?.getData('text/uri-list') || event.dataTransfer?.getData('text/plain') || ''
  if (downloadTab.value === 'multiple') {
    queueWorkspace.value?.appendLinks(text)
    return
  }
  const found = text.match(/https?:\/\/\S+/)
  if (found) {
    url.value = found[0].trim()
  }
}

function handleToolInstallProgress(event: ToolInstallEvent) {
  installEvents.value = { ...installEvents.value, [event.tool]: event }
}

function saveDefaultDir(path: string) {
  defaultDir.value = path
  preferences.update({ defaultDir: path })
}

function showError(error: unknown, fallback: string) {
  const payload = typeof error === 'object' && error !== null ? error as { message?: string; detail?: string } : null
  const detail = payload?.detail || String(error || fallback)
  errorMessage.value = friendlyErrorMessage(payload?.message || fallback, detail)
  errorDetail.value = payload?.detail || fallback
}

function clearError() {
  errorMessage.value = ''
  errorDetail.value = ''
}

function friendlyErrorMessage(message: string, detail: string) {
  const failure = classifyQueueError({ message, detail })
  return failure.kind === 'unknown' ? message : failure.message
}

</script>

<template>
  <section class="download-page" :class="{ dragging: dragActive }" @dragover="handleDragOver" @dragleave="handleDragLeave" @drop="handleDrop">
    <div class="content-scroll">
    <section class="hero">
      <PageHeader title="Downloads" />
      <div class="download-tabs segmented-control" role="tablist" aria-label="Download input mode">
        <button v-for="(tab, index) in downloadTabs" :id="`downloads-tab-${tab.value}`" :key="tab.value"
          class="download-tab segmented-option" type="button" role="tab" :aria-selected="downloadTab === tab.value"
          :aria-controls="`downloads-panel-${tab.value}`" :tabindex="downloadTab === tab.value ? 0 : -1"
          @click="downloadTab = tab.value" @keydown="onDownloadTabKeydown($event, index)">
          {{ tab.label }}
        </button>
      </div>
      <Transition name="fade">
        <section v-if="depsChecked && hasMissingTools" class="download-readiness-card" aria-live="polite">
          <span class="download-readiness-mark"><span class="icon" aria-hidden="true" :style="iconStyle('settings')" /></span>
          <div>
            <strong>Missing {{ missingTools.join(', ') }}</strong>
          </div>
          <button class="button" type="button" :disabled="installing" @click="openInstallConfirm">
            {{ installing ? 'Installing…' : 'Install' }}
          </button>
        </section>
      </Transition>

      <div class="tool-disclosure">
        <button class="tool-disclosure-trigger" type="button" :aria-expanded="toolsOpen" aria-controls="download-tools-panel" @click="toolsOpen = !toolsOpen">
          <span class="status-square" :class="{ ok: deps?.ytdlp_ok && deps?.ffmpeg_ok, bad: depsChecked && hasMissingTools }" />
          <span>{{ toolStatusText }}</span>
          <span class="tool-summary-hint">Download tools</span>
          <DisclosureChevron :open="toolsOpen" />
        </button>
        <AppCollapsible :open="toolsOpen">
        <div id="download-tools-panel" class="tool-disclosure-content">
          <div class="env-row">
            <div class="env-item" :class="{ ok: deps?.ytdlp_ok, bad: deps && !deps.ytdlp_ok }">
              <span class="status-square" />
              <span :title="deps?.ytdlp_path || ''">yt-dlp {{ deps?.ytdlp_ok ? `Ready ${deps.ytdlp_version || ''}` : 'Missing' }}</span>
            </div>
            <div class="env-item" :class="{ ok: deps?.ffmpeg_ok, bad: deps && !deps.ffmpeg_ok }">
              <span class="status-square" />
              <span :title="deps?.ffmpeg_path || ''">ffmpeg {{ deps?.ffmpeg_ok ? 'Ready' : 'Missing' }}</span>
            </div>
            <button class="link-button" type="button" :disabled="loadingDeps || parsing || downloading" @click="handleCheckTools">
              <span class="icon" aria-hidden="true" :style="iconStyle('check')" />
              <span>{{ loadingDeps ? 'Checking…' : 'Check again' }}</span>
            </button>
            <button v-if="deps?.ytdlp_update_available" class="link-button" type="button" :disabled="updatingYtdlp" @click="handleUpdateYtdlp">
              <span class="icon" aria-hidden="true" :style="iconStyle('refresh')" />
              <span>{{ updatingYtdlp ? 'Updating…' : 'Update yt-dlp' }}</span>
            </button>
            <button v-if="hasMissingTools" class="link-button" type="button" @click="openInstallConfirm">
              <span class="icon" aria-hidden="true" :style="iconStyle('settings')" />
              <span>Install missing tools</span>
            </button>
          </div>
          <div v-if="deps?.ytdlp_ok || deps?.ffmpeg_ok" class="tool-paths">
            <span v-if="deps?.ytdlp_ok">yt-dlp: {{ deps.ytdlp_path }}</span>
            <span v-if="deps?.ffmpeg_ok">ffmpeg: {{ deps.ffmpeg_path }}</span>
          </div>
        </div>
        </AppCollapsible>
      </div>

    </section>

    <div id="downloads-panel-single" class="download-input-panel" role="tabpanel" aria-labelledby="downloads-tab-single" :hidden="downloadTab !== 'single'" tabindex="0">
      <div class="input-row-container">
        <div class="input-row">
          <input
            ref="urlInput"
            aria-label="Video or audio URL"
            v-model="url"
            class="url-input"
            type="url"
            placeholder="Paste a video URL"
            :disabled="parsing || downloading"
            @keydown.enter="handleParse"
          />
          <button
            class="button cookie-file-button icon-only"
            type="button"
            :disabled="parsing || downloading"
            :title="cookiesFilePath ? 'Replace cookies.txt' : 'Add cookies.txt for videos requiring sign-in'"
            :aria-label="cookiesFilePath ? 'Replace cookies' : 'Add cookies'"
            @click="handleSelectCookiesFile"
          >
            <span class="icon" aria-hidden="true" :style="iconStyle('cookie')" />
          </button>
          <button class="button primary parse-button" :class="{ parsing }" type="button" :disabled="!canParse" @click="handleParse">
            <span class="icon" aria-hidden="true" :style="iconStyle('mediaVideo')" />
            <span>{{ parsing ? 'Analyzing…' : 'Analyze' }}</span>
          </button>
        </div>
        <div v-if="cookiesFilePath" class="cookies-inline-status">
          <span :title="cookiesFilePath">cookies.txt: {{ cookiesFilePath }}</span>
          <button class="link-button compact" type="button" :disabled="parsing || downloading" @click="clearCookiesFile">Clear</button>
        </div>
        <Transition name="fade">
          <div v-if="showClipboardTip" class="clipboard-tip-bubble">
            <span class="tip-text" :title="clipboardTipUrl">Clipboard URL: {{ clipboardTipUrl }}</span>
            <div class="tip-actions">
              <button class="tip-btn primary" type="button" @click="handleUseClipboardUrl">Use</button>
              <button class="tip-btn" type="button" @click="showClipboardTip = false">Dismiss</button>
            </div>
          </div>
        </Transition>
      </div>



    <section v-if="!video && !parsing && !downloadQueue.jobs.length" class="download-empty" aria-label="Download workspace">
      <span class="empty-symbol"><span class="icon" aria-hidden="true" :style="iconStyle('mediaVideo')" /></span>
      <h2>Download from a link</h2>
      <p>Paste a video or audio URL above to choose a format and save it to your computer.</p>
    </section>

    <Transition name="reveal">
      <section v-if="video" ref="detailsSection" class="details">
        <div class="preview">
          <div class="thumb">
            <img v-if="video.thumbnail" :src="video.thumbnail" alt="" />
            <span v-else class="thumbnail-fallback"><span class="icon" aria-hidden="true" :style="iconStyle('mediaVideo')" /><span>No preview</span></span>
          </div>
          <div class="video-meta">
            <span class="label">{{ video.isPlaylist ? 'Playlist' : 'Video' }}<template v-if="video.extractor"> · {{ video.extractor }}</template></span>
            <h2>{{ video.title }}</h2>
            <p v-if="video.uploader || video.duration">{{ video.uploader || 'Unknown author' }} · {{ durationText }}</p>
          </div>
        </div>

        <div v-if="video.isPlaylist" class="playlist-panel">
          <div class="playlist-header">
            <span>Playlist {{ video.entries?.filter(e => selectedEntries[e.url]).length || 0 }}/{{ video.entries?.length || 0 }}</span>
            <div class="playlist-actions-quick">
              <button class="link-button compact" type="button" title="Select all" aria-label="Select all" @click="selectAllEntries(true)">
                <span class="icon" aria-hidden="true" :style="iconStyle('checkSquare')" />
              </button>
              <span class="divider">|</span>
              <button class="link-button compact" type="button" title="Deselect all" aria-label="Deselect all" @click="selectAllEntries(false)">
                <span class="icon" aria-hidden="true" :style="iconStyle('xmark')" />
              </button>
            </div>
          </div>
          <div class="playlist-list-scroll">
            <label v-for="entry in video.entries" :key="entry.url" class="playlist-item" :class="{ active: selectedEntries[entry.url] }">
              <input type="checkbox" v-model="selectedEntries[entry.url]" :disabled="downloading || queueActive" />
              <div class="entry-meta">
                <span class="entry-title" :title="entry.title">{{ entry.title }}</span>
                <span v-if="entry.duration" class="entry-duration">
                  {{ Math.floor(entry.duration / 60) }}:{{ (Math.floor(entry.duration % 60)).toString().padStart(2, '0') }}
                </span>
              </div>
            </label>
          </div>
        </div>

        <div class="download-block">
          <div v-if="formatOptions.length" class="format-select-row">
            <span>Format</span>
            <AppSelect
              v-model="selectedFormatId"
              :options="formatOptions"
              :disabled="downloading || queueActive"
              aria-label="Format"
            />
          </div>

          <div class="format-select-row save-location-row">
            <span>Save to</span>
          <div class="path-row">
            <div class="path-display" :title="saveDir || 'No folder selected'">{{ saveDir || 'Choose a folder' }}</div>
            <button class="button icon-button" type="button" title="Choose folder" aria-label="Choose folder" :disabled="downloading || queueActive" @click="handleSelectDir">
              <span class="icon" aria-hidden="true" :style="iconStyle('folder')" />
            </button>
          </div>
          </div>
          <label class="check-row">
            <input v-model="setAsDefault" type="checkbox" :disabled="downloading || queueActive" @change="handleDefaultToggle" />
            <span>Use as default folder</span>
          </label>

          <details class="file-options-details">
            <summary>More options <span class="options-summary-hint">Formats, audio and files</span></summary>
            <div class="download-options-content">
              <button v-if="video.formats?.length && !video.isPlaylist" class="link-button compact format-details-toggle" type="button" :disabled="downloading || queueActive" :aria-expanded="showDetailedFormats" @click="showDetailedFormats = !showDetailedFormats">{{ showDetailedFormats ? 'Use simple format choices' : 'Choose a specific format' }}</button>
              <div v-if="audioOptions.length > 1 && !video.isPlaylist" class="format-select-row">
                <span>Audio track</span>
                <AppSelect v-model="selectedAudioId" :options="audioOptions" :disabled="downloading || queueActive || (!!selectedFormat && selectedFormat.acodec !== 'none')" aria-label="Audio track" />
              </div>
              <p v-if="selectedFormat && selectedFormat.acodec !== 'none'" class="format-help">This format includes audio. Choose a video-only format to select a separate audio track.</p>
              <p v-if="selectedFormat" class="format-help">The selected format is required. If it becomes unavailable, reanalyze or choose another format.</p>
              <DownloadOptionsFields v-model="fileOptions" :disabled="downloading || queueActive" />
              <details v-if="video.parseStrategy || video.site" class="video-diagnostics"><summary>Source details</summary><p class="parse-note">{{ [video.parseStrategy, video.site].filter(Boolean).join(' · ') }}</p></details>
            </div>
          </details>
          <div v-if="progress" class="progress-panel" :class="{ 'is-finished': progress.status === 'finished' }" aria-live="polite">
            <div class="progress-head">
              <span class="progress-status"><i v-if="progressIndeterminate" class="progress-spinner" aria-hidden="true" />{{ progressStatusText }}</span>
              <strong v-if="progress.status === 'starting'">0%</strong>
              <strong v-else-if="!progressIndeterminate">{{ progressPercent.toFixed(1) }}%</strong>
            </div>
            <div v-if="progress.status !== 'processing'" class="progress-track" role="progressbar" aria-label="Download progress" :aria-valuenow="progress.status === 'starting' ? 0 : progressIndeterminate ? undefined : progressPercent" :aria-valuetext="progressStatusText" :aria-valuemin="0" :aria-valuemax="100">
              <div class="progress-fill" :style="{ width: `${progressTrackPercent}%` }" />
            </div>
            <dl class="progress-foot">
              <div><dt>Speed</dt><dd>{{ progress.status === 'downloading' ? progress.speed || 'Measuring…' : '—' }}</dd></div>
              <div><dt>Time remaining</dt><dd>{{ remainingTimeText }}</dd></div>
              <div><dt>{{ progress.metrics?.downloadedBytes != null ? 'Downloaded / Total' : 'Download size' }}</dt><dd>{{ downloadSizeText }}</dd></div>
            </dl>
          </div>

          <div class="download-actions">
            <button v-if="video.isPlaylist && failedEntries.length && !queueActive" class="button" type="button" :disabled="!canDownload" @click="handleBatchDownload(true)">Retry {{ failedEntries.length }} failed downloads</button>
            <template v-if="video.isPlaylist">
              <button v-if="!queueActive" class="button primary" type="button" :disabled="!canDownload || selectedCount === 0" @click="handleBatchDownload()">
                <span class="icon" aria-hidden="true" :style="iconStyle('download')" />
                <span>Download {{ video.entries?.filter(e => selectedEntries[e.url]).length || 0 }} videos</span>
              </button>
              <button v-else class="button danger" type="button" :disabled="cancelling" @click="cancelQueue">
                <span class="icon" aria-hidden="true" :style="iconStyle('xmark')" />
                <span>Cancel</span>
              </button>
            </template>
            <template v-else>
              <button v-if="!downloading && progress?.status !== 'finished'" class="button primary" type="button" :disabled="!canDownload" @click="handleDownload()">
                <span class="icon" aria-hidden="true" :style="iconStyle('download')" />
                <span>Download</span>
              </button>
              <button v-if="downloading" class="button" type="button" :disabled="cancelling" @click="handleCancel">
                <span class="icon" aria-hidden="true" :style="iconStyle('xmark')" />
                <span>{{ cancelling ? 'Cancelling…' : 'Cancel download' }}</span>
              </button>
            </template>

            <template v-if="!downloading && !queueActive && progress?.status === 'finished' && completedFilePath">
              <button class="button primary" type="button" title="Open folder" aria-label="Open folder" @click="handleOpenParentFolder">
                <span class="icon" aria-hidden="true" :style="iconStyle('folder')" />
                <span>Open folder</span>
              </button>
              <button class="button" type="button" @click="handleOpenFile">Open file</button>
            </template>
            <p v-if="!progress && !video.isPlaylist" class="download-size-preview">Estimated size <strong>{{ downloadSizeText }}</strong></p>
          </div>
        </div>
      </section>
    </Transition>

    <div v-if="errorMessage" class="error-box" role="alert">
      <strong>{{ errorMessage }}</strong>
      <details>
        <summary>
          <span class="icon disclosure-icon" aria-hidden="true" :style="iconStyle('navArrowRight')" />
          <span>Error details</span>
        </summary>
        <div class="details-content">
          <pre>{{ errorDetail }}</pre>
        </div>
      </details>
    </div>
    <details v-if="hasLogs" class="log-box" :open="logsOpen" @toggle="handleLogsToggle">
      <summary>
        <span class="icon disclosure-icon" aria-hidden="true" :style="iconStyle('navArrowRight')" />
        <span>Logs</span>
      </summary>
      <div class="details-content">
        <pre v-if="logsOpen">{{ logs.join('\n') }}</pre>
      </div>
    </details>
    </div>
    <DownloadQueue ref="queueWorkspace" :import-active="downloadTab === 'multiple'" :blocked="parsing || installing || updatingYtdlp" :tools-ready="!!deps?.ytdlp_ok && !!deps?.ffmpeg_ok" />
    </div>
    <AppDialog :open="showInstallConfirm" :title="installing ? 'Installing tools…' : 'Install missing tools?'" :busy="installing" @close="showInstallConfirm = false">
        <template v-if="!installing">
          <p class="modal-label">Required tools:</p>
          <ul>
            <li v-for="tool in missingTools" :key="tool">{{ tool }}</li>
          </ul>
          <p class="modal-label">Install location:</p>
          <code>{{ toolsDir }}</code>
          <div class="modal-actions">
            <button class="button" type="button" @click="showInstallConfirm = false">Cancel</button>
            <button class="button primary" type="button" @click="handleInstallTools">Install</button>
          </div>
        </template>
        <template v-else>
          <div class="install-list">
            <div v-for="tool in missingTools" :key="tool" class="install-row">
              <span>{{ tool }}</span>
              <span>{{ installEvents[tool]?.status === 'installed' ? 'Installed' : installEvents[tool]?.status === 'error' ? 'Installation failed' : installEvents[tool]?.status === 'extracting' ? 'Installing…' : installEvents[tool] ? 'Downloading…' : 'Waiting…' }}</span>
              <strong>{{ installEvents[tool]?.percent ? `${installEvents[tool]?.percent?.toFixed(0)}%` : '' }}</strong>
            </div>
          </div>
          <div class="progress-track">
            <div class="progress-fill" :style="{ width: `${Math.max(...missingTools.map((tool) => installEvents[tool]?.percent || 0), 0)}%` }" />
          </div>
        </template>
    </AppDialog>

    <div v-if="dragActive" class="drop-overlay">Drop a URL here</div>
  </section>
</template>

<style scoped>
.download-tab { min-inline-size: 126px; }
.file-options-details { margin-block-start: 20px; }
.file-options-details > summary { cursor: pointer; color: var(--muted); font-size: 14px; line-height: 20px; min-block-size: var(--control-height-compact); padding-block: 6px; }
.file-options-details > summary:active { color: var(--ink); }
.file-options-details[open] > summary { color: var(--ink); }
.options-summary-hint { color: var(--subtle); margin-inline-start: 8px; }
.file-options-details[open] .options-summary-hint { display: none; }
.download-options-content { display: grid; gap: 14px; padding-block: 14px 4px; }
.download-options-content > .format-select-row { margin-block: 0; }
.format-details-toggle { justify-self: start; }
.format-help { font-size: 14px; color: var(--muted); margin-block: 0; }
@media (hover: hover) and (pointer: fine) { .file-options-details > summary:hover { color: var(--ink); } }
.download-input-panel { width: 100%; max-width: 1040px; margin: 0 auto; }
.download-input-panel[hidden] { display: none; }
.hero { margin-bottom: 20px; }
.download-input-panel + :deep(.saved-queue) { margin-top: 24px; }
</style>
