<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import checkIcon from 'iconoir/icons/check.svg?url'
import checkSquareIcon from 'iconoir/icons/check-square.svg?url'
import copyIcon from 'iconoir/icons/copy.svg?url'
import cookieIcon from 'iconoir/icons/cookie.svg?url'
import downloadIcon from 'iconoir/icons/download.svg?url'
import emptyPageIcon from 'iconoir/icons/empty-page.svg?url'
import folderIcon from 'iconoir/icons/folder.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import navArrowRightIcon from 'iconoir/icons/nav-arrow-right.svg?url'
import openNewWindowIcon from 'iconoir/icons/open-new-window.svg?url'
import refreshIcon from 'iconoir/icons/refresh.svg?url'
import settingsIcon from 'iconoir/icons/settings.svg?url'
import trashIcon from 'iconoir/icons/trash.svg?url'
import xmarkIcon from 'iconoir/icons/xmark.svg?url'
import {
  cancelDownload,
  checkDependencies,
  checkYtdlpUpdate,
  extractSubtitles,
  getToolsDirectory,
  installMissingTools,
  onDownloadProgress,
  onToolInstallProgress,
  openParentFolder,
  openPath,
  parseVideo,
  selectCookiesFile,
  selectDirectory,
  startDownload,
  updateYtdlp,
} from '../api/tauri'
import { AppSelect } from '@/components/ui/select'
import AppDialog from '@/components/ui/AppDialog.vue'
import AppCollapsible from '@/components/ui/AppCollapsible.vue'
import DisclosureChevron from '@/components/ui/DisclosureChevron.vue'
import PageHeader from '@/components/ui/PageHeader.vue'
import { useTasksStore } from '@/stores/tasks'
import { useActivityStore } from '@/stores/activity'
import type { CookieSource, DependencyStatus, DownloadHistoryItem, DownloadProgressEvent, ToolInstallEvent, VideoInfo, VideoFormat, DownloadMode } from '../types'

const router = useRouter()
const activity = useActivityStore()
const icons = {
  check: checkIcon,
  checkSquare: checkSquareIcon,
  copy: copyIcon,
  cookie: cookieIcon,
  download: downloadIcon,
  emptyPage: emptyPageIcon,
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
const downloading = ref(false)
const progress = ref<DownloadProgressEvent | null>(null)
const errorMessage = ref('')
const errorDetail = ref('')
const logs = shallowRef<string[]>([])
const hasLogs = ref(false)
const logBuffer: string[] = []
let logFlushTimer: number | undefined
const completedFilePath = ref('')
const extractedSubtitlePath = ref('')
const extractingSubtitles = ref(false)
const displayPercent = ref(0)
const tasks = useTasksStore()
const history = computed(() => tasks.downloads)
const cancelling = ref(false)
const logsOpen = ref(false)
const urlInput = ref<HTMLInputElement | null>(null)
const dragActive = ref(false)
const toolsDir = ref('')
const showInstallConfirm = ref(false)
const installing = ref(false)
const updatingYtdlp = ref(false)
const installEvents = ref<Record<string, ToolInstallEvent>>({})
const copiedHistoryId = ref('')
const detailsSection = ref<HTMLElement | null>(null)
let copiedHistoryTimer: number | undefined
let unlistenDownloadProgress: (() => void) | undefined
let unlistenToolProgress: (() => void) | undefined
let unregisterDownloadCanceller: (() => void) | undefined
const lastParsedUrl = ref('')
const clipboardTipUrl = ref('')
const showClipboardTip = ref(false)

function openSubtitleWorkspace(path = '') {
  void router.push({ name: 'subtitles', query: path ? { source: path } : undefined })
}

const selectedFormatId = ref<string>('')

const selectedEntries = ref<Record<string, boolean>>({})

const queueActive = ref(false)
const queueTotal = ref(0)
const queueCurrentIndex = ref(0)
const queueCompleted = ref(0)
const queueFailed = ref(0)
const currentDownloadTitle = ref('')
const currentDownloadUrl = ref('')

let queueStopRequested = false

const missingTools = computed(() => {
  const missing: string[] = []
  if (deps.value && !deps.value.ytdlp_ok) missing.push('yt-dlp')
  if (deps.value && !deps.value.ffmpeg_ok) missing.push('ffmpeg')
  return missing
})

const canParse = computed(() => Boolean(url.value.trim() && !parsing.value && !downloading.value && !extractingSubtitles.value))
const canDownload = computed(() => Boolean(video.value && saveDir.value && !parsing.value && !downloading.value && !extractingSubtitles.value && !hasMissingTools.value))
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

const queueProgressPercent = computed(() => {
  if (!queueTotal.value) return 0
  const settledItems = queueCompleted.value + queueFailed.value
  const terminal = ['finished', 'cancelled', 'error'].includes(progress.value?.status || '')
  const completedThroughCurrent = terminal ? Math.max(settledItems, queueCurrentIndex.value) : settledItems
  const currentItemProgress = queueActive.value && !terminal && !progressIndeterminate.value && queueCurrentIndex.value > completedThroughCurrent
    ? progressPercent.value / 100
    : 0
  return Math.max(0, Math.min(100, ((completedThroughCurrent + currentItemProgress) / queueTotal.value) * 100))
})

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

const availableFormats = computed(() => {
  if (!video.value?.formats) return []
  
  const seenHeights = new Set<number>()
  const formatsList: Array<{ label: string; formatId: string; height: number; type: 'video' | 'audio' }> = []
  
  for (const fmt of video.value.formats) {
    if (fmt.vcodec && fmt.vcodec !== 'none') {
      const height = fmt.height ?? 0
      if (height > 0 && !seenHeights.has(height)) {
        seenHeights.add(height)
        let label = `${height}p`
        if (fmt.note && fmt.note.trim().toLowerCase() !== `${height}p`) {
          label += ` (${fmt.note})`
        }
        if (fmt.ext) {
          label += ` - ${fmt.ext}`
        }
        formatsList.push({
          label,
          formatId: fmt.formatId,
          height,
          type: 'video'
        })
      }
    }
  }
  
  formatsList.sort((a, b) => b.height - a.height)
  
  formatsList.push({
    label: 'Audio only · MP3',
    formatId: 'bestaudio',
    height: 0,
    type: 'audio'
  })
  
  return formatsList
})

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
  const audio = video.value?.formats?.filter(f => f.vcodec === 'none' && f.acodec && f.acodec !== 'none') || []
  if (selectedFormatId.value === 'bestaudio') return formatSize(audio.at(-1))
  const chosen = selectedFormat.value
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
const formatOptions = computed(() =>
  availableFormats.value.map(format => ({
    value: format.formatId,
    label: format.label,
  })),
)

function iconStyle(name: IconName) {
  return { '--icon-url': `url("${icons[name]}")` }
}

onMounted(() => {
  window.addEventListener('focus', checkClipboard)
  defaultDir.value = localStorage.getItem('ydlite.defaultDir') || ''
  saveDir.value = defaultDir.value
  setAsDefault.value = Boolean(defaultDir.value)
  cookiesFilePath.value = localStorage.getItem('ydlite.cookiesFilePath') || ''
  unregisterDownloadCanceller = activity.registerDownloadCanceller(handleGlobalDownloadCancel)
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

onBeforeUnmount(() => {
  resetLogs()
  if (copiedHistoryTimer) window.clearTimeout(copiedHistoryTimer)
  unlistenDownloadProgress?.()
  unlistenToolProgress?.()
  unregisterDownloadCanceller?.()
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
    localStorage.setItem('ydlite.cookiesFilePath', selected)
  }
}

function clearCookiesFile() {
  cookiesFilePath.value = ''
  localStorage.removeItem('ydlite.cookiesFilePath')
}

function handleUseClipboardUrl() {
  url.value = clipboardTipUrl.value
  showClipboardTip.value = false
  void handleParse()
}

async function scrollToVideoDetails() {
  await nextTick()
  detailsSection.value?.scrollIntoView({
    behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
    block: 'start',
  })
}

async function handleParse() {
  if (!canParse.value) return
  clearError()
  video.value = null
  completedFilePath.value = ''
  extractedSubtitlePath.value = ''
  progress.value = null
  resetDisplayedProgress()
  parsing.value = true
  showClipboardTip.value = false
  selectedEntries.value = {}
  selectedFormatId.value = ''
  try {
    const result = await parseVideo({ url: url.value, options: { cookieSource: selectedCookieSource() } })
    video.value = result
    if (result.resolvedUrl && result.resolvedUrl !== url.value.trim()) {
      url.value = result.resolvedUrl
    }
    lastParsedUrl.value = url.value
    
    if (result.isPlaylist && result.entries) {
      result.entries.forEach(e => {
        selectedEntries.value[e.url] = true
      })
    } else if (availableFormats.value.length > 0) {
      selectedFormatId.value = availableFormats.value[0].formatId
    }
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
    localStorage.removeItem('ydlite.defaultDir')
  }
}

async function handleDownload(targetUrl?: string) {
  if (!canDownload.value) return
  clearError()
  resetLogs()
  completedFilePath.value = ''
  extractedSubtitlePath.value = ''
  resetDisplayedProgress()
  progress.value = { status: 'starting' }
  downloading.value = true
  if (setAsDefault.value && saveDir.value) saveDefaultDir(saveDir.value)
  
  const downloadUrl = targetUrl || video.value?.resolvedUrl || url.value
  currentDownloadUrl.value = downloadUrl
  let mode: DownloadMode = 'best'
  let formatId: string | null = null

  if (selectedFormatId.value) {
    if (selectedFormatId.value === 'bestaudio') {
      mode = 'mp3'
    } else {
      mode = 'custom'
      formatId = selectedFormatId.value
    }
  }

  try {
    activity.beginDownload({ title: video.value?.title || 'Downloads' })
    const activeCookieSource = video.value?.cookieSource || selectedCookieSource()
    await startDownload({ url: downloadUrl, dir: saveDir.value, mode, formatId, options: { cookieSource: activeCookieSource } })
  } catch (error) {
    if (progress.value?.status !== 'cancelled') {
      progress.value = { ...progress.value, status: 'error' }
      showError(error, 'Download failed. See details and try again.')
      activity.finishDownload({ state: 'failed', detail: 'Download failed. See details and try again.' })
    }
  } finally {
    if (!queueActive.value) {
      downloading.value = false
      cancelling.value = false
    }
  }
}

async function handleExtractSubtitles() {
  if (!video.value || video.value.isPlaylist || !saveDir.value || extractingSubtitles.value) return

  clearError()
  extractedSubtitlePath.value = ''
  extractingSubtitles.value = true
  try {
    extractedSubtitlePath.value = await extractSubtitles({
      url: video.value.resolvedUrl || url.value,
      dir: saveDir.value,
      title: video.value.title,
      options: { cookieSource: video.value.cookieSource || selectedCookieSource() },
    })
  } catch (error) {
    showError(error, 'Unable to extract site subtitles. See details.')
  } finally {
    extractingSubtitles.value = false
  }
}

async function downloadPromise(targetUrl: string, title: string) {
  resetLogs()
  completedFilePath.value = ''
  resetDisplayedProgress()
  progress.value = { status: 'starting' }
  downloading.value = true
  currentDownloadTitle.value = title
  currentDownloadUrl.value = targetUrl

  let mode: DownloadMode = 'best'
  let formatId: string | null = null

  if (selectedFormatId.value) {
    if (selectedFormatId.value === 'bestaudio') {
      mode = 'mp3'
    } else {
      mode = 'custom'
      formatId = selectedFormatId.value
    }
  }

  activity.beginDownload({
    title,
    queue: {
      total: queueTotal.value,
      current: queueCurrentIndex.value,
      completed: queueCompleted.value,
      failed: queueFailed.value,
    },
  })
  const activeCookieSource = video.value?.cookieSource || selectedCookieSource()
  // IPC completion owns queue advancement; progress events can arrive before rejection.
  await startDownload({ url: targetUrl, dir: saveDir.value, mode, formatId, options: { cookieSource: activeCookieSource } })
}

async function handleBatchDownload() {
  if (!canDownload.value || !video.value?.entries || queueActive.value) return
  
  const selectedList = video.value.entries.filter(e => selectedEntries.value[e.url])
  if (selectedList.length === 0) {
    showError(new Error('Select at least one video.'), 'No videos selected.')
    return
  }

  clearError()
  queueStopRequested = false
  queueActive.value = true
  if (setAsDefault.value && saveDir.value) saveDefaultDir(saveDir.value)
  queueTotal.value = selectedList.length
  queueCurrentIndex.value = 0
  queueCompleted.value = 0
  queueFailed.value = 0

  for (let i = 0; i < selectedList.length; i++) {
    if (queueStopRequested) break
    queueCurrentIndex.value = i + 1
    const item = selectedList[i]
    
    try {
      await downloadPromise(item.url, item.title)
      if (progress.value?.status === 'cancelled') queueStopRequested = true
      else queueCompleted.value++
    } catch (e) {
      queueFailed.value++
      showError(e, 'A queued download failed. See details.')
    }
    activity.updateDownloadQueue({
      total: queueTotal.value,
      current: queueCurrentIndex.value,
      completed: queueCompleted.value,
      failed: queueFailed.value,
      active: queueActive.value,
    }, currentDownloadTitle.value || item.title)
  }

  const cancelled = queueStopRequested
  queueActive.value = false
  downloading.value = false
  currentDownloadTitle.value = ''
  cancelling.value = false
  activity.finishDownload({
    state: cancelled ? 'cancelled' : queueFailed.value > 0 ? 'failed' : 'completed',
    detail: cancelled
      ? 'Download queue cancelled'
      : queueFailed.value
        ? `Queue complete: ${queueCompleted.value} succeeded, ${queueFailed.value} failed`
        : `Download queue complete: ${queueCompleted.value} videos`,
  })
}

function cancelQueue() {
  queueStopRequested = true
  void handleCancel()
}

async function handleGlobalDownloadCancel() {
  if (queueActive.value) queueStopRequested = true
  await handleCancel()
}

function selectAllEntries(val: boolean) {
  if (downloading.value || queueActive.value) return
  if (!video.value?.entries) return
  video.value.entries.forEach(e => {
    selectedEntries.value[e.url] = val
  })
}


async function handleCancel() {
  if (cancelling.value) return
  cancelling.value = true
  try { await cancelDownload() }
  catch (error) {
    cancelling.value = false
    showError(error, 'Unable to cancel. Try again.')
  }
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
  if (event.status === 'finished' && event.filePath && video.value) {
    addHistory({
      id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
      title: currentDownloadTitle.value || video.value.title,
      extractor: video.value.extractor,
      filePath: event.filePath,
      url: currentDownloadUrl.value || url.value,
      completedAt: new Date().toISOString(),
    })
  }

}

async function copyHistoryUrl(item: DownloadHistoryItem) {
  try {
    await navigator.clipboard.writeText(item.url)
    copiedHistoryId.value = item.id
    if (copiedHistoryTimer) window.clearTimeout(copiedHistoryTimer)
    copiedHistoryTimer = window.setTimeout(() => {
      if (copiedHistoryId.value === item.id) copiedHistoryId.value = ''
      copiedHistoryTimer = undefined
    }, 1200)
  } catch (error) {
    showError(error, 'Unable to copy URL.')
  }
}

async function openHistoryFolder(item: DownloadHistoryItem) {
  try {
    await openParentFolder(item.filePath)
  } catch (error) {
    showError(error, 'Unable to open task folder.')
  }
}

function removeHistoryItem(item: DownloadHistoryItem) {
  tasks.removeDownload(item.id)
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
  const text = event.dataTransfer?.getData('text/uri-list') || event.dataTransfer?.getData('text/plain') || ''
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
  localStorage.setItem('ydlite.defaultDir', path)
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
  const text = `${message}\n${detail}`.toLowerCase()
  if (text.includes('没有可提取的站点字幕') || text.includes('did not return any subtitle files')) {
    return completedFilePath.value
      ? 'No site subtitles found. Use Create subtitles to transcribe the video.'
      : 'No manual or automatic subtitles available.'
  }
  if (text.includes('未检测到 yt-dlp') || (text.includes('yt-dlp') && (text.includes('not found') || text.includes('no such file') || text.includes('unable to execute')))) {
    return 'Unable to start yt-dlp. Check your tools and install missing components.'
  }
  if (text.includes('ffmpeg') && (text.includes('not found') || text.includes('no such file') || text.includes('unable to execute'))) {
    return 'FFmpeg is required to merge video or convert audio.'
  }
  if (text.includes('cloudflare') || text.includes('anti-bot challenge') || text.includes('http error 403')) {
    return 'Request blocked. Add cookies.txt or try again later.'
  }
  if (text.includes('bilibili') && (text.includes('http error 412') || text.includes('precondition failed'))) {
    return 'Bilibili rejected the metadata request. Add cookies.txt and try again.'
  }
  if (text.includes('older than 90 days') || text.includes('use that to update')) {
    return deps.value?.ytdlp_update_available
      ? 'yt-dlp is outdated. Update it, then try again.'
      : 'No yt-dlp update found. Check the tool path or try again later.'
  }
  if (text.includes('impersonate') || text.includes('impersonation')) {
    return 'This link needs yt-dlp browser impersonation. The site may require extra dependencies or stricter access.'
  }
  if (text.includes('private') || text.includes('login') || text.includes('sign in') || text.includes('cookies')) {
    return 'This link may need login cookies. Add cookies.txt, or close the browser that owns the cookies and retry.'
  }
  if (text.includes('unsupported url') || text.includes('no suitable extractor')) {
    return 'yt-dlp does not support this URL, or the URL format is invalid.'
  }
  if (text.includes('timed out') || text.includes('connection') || text.includes('network') || text.includes('http error')) {
    return 'Connection failed. Check your network or proxy.'
  }
  if (text.includes('requested format is not available') || text.includes('format is not available')) {
    return 'The selected format is not available for this video.'
  }
  if (text.includes('permission denied') || text.includes('access is denied')) {
    return 'No write permission for this folder. Choose another folder.'
  }
  if (text.includes('invalid argument') || text.includes('filename') || text.includes('path')) {
    return 'The file path or name is invalid. Choose another folder and retry.'
  }
  return message
}

function addHistory(item: DownloadHistoryItem) {
  try { tasks.addDownload(item) }
  catch (error) { showError(error, 'Download complete. Unable to save history.') }
}
function formatHistoryTime(value: string) {
  return new Date(value).toLocaleString()
}</script>

<template>
  <section class="download-page" :class="{ dragging: dragActive }" @dragover="handleDragOver" @dragleave="handleDragLeave" @drop="handleDrop">
    <div class="content-scroll">
    <section class="hero">
      <PageHeader title="Downloads" />
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
          <button class="link-button compact" type="button" :disabled="parsing || downloading || extractingSubtitles" @click="clearCookiesFile">Clear</button>
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
            <button class="link-button" type="button" :disabled="loadingDeps || parsing || downloading || extractingSubtitles" @click="handleCheckTools">
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

    <section v-if="!video && !parsing" class="download-empty" aria-label="Download workspace">
      <span class="empty-symbol"><span class="icon" aria-hidden="true" :style="iconStyle('mediaVideo')" /></span>
      <h2>Download from a link</h2>
      <p>Paste a video or audio URL above to choose a format and save it to your computer.</p>
      <div class="local-source-link"><button class="link-button" type="button" @click="router.push({ name: 'subtitles' })">Subtitles for a local video <span aria-hidden="true">→</span></button></div>
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
            <details class="video-diagnostics"><summary>Details</summary><p class="parse-note">{{ video.parseStrategy }} · {{ video.site }}</p></details>
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
          <div v-if="!video.isPlaylist && availableFormats.length" class="format-select-row">
            <span>Format</span>
            <AppSelect
              v-model="selectedFormatId"
              :options="formatOptions"
              :disabled="downloading || queueActive || extractingSubtitles"
              aria-label="Format"
            />
          </div>

          <div class="format-select-row save-location-row">
            <span>Save to</span>
          <div class="path-row">
            <div class="path-display" :title="saveDir || 'No folder selected'">{{ saveDir || 'Choose a folder' }}</div>
            <button class="button icon-button" type="button" title="Choose folder" aria-label="Choose folder" :disabled="downloading || queueActive || extractingSubtitles" @click="handleSelectDir">
              <span class="icon" aria-hidden="true" :style="iconStyle('folder')" />
            </button>
          </div>
          </div>
          <label class="check-row">
            <input v-model="setAsDefault" type="checkbox" :disabled="downloading || queueActive || extractingSubtitles" @change="handleDefaultToggle" />
            <span>Use as default folder</span>
          </label>

          <div v-if="queueActive" class="queue-status-panel">
            <div class="queue-status-head">
              <span>Queue {{ queueCurrentIndex }}/{{ queueTotal }}</span>
              <span>Completed {{ queueCompleted }} · Failed {{ queueFailed }}</span>
            </div>
            <div class="queue-progress-track">
              <div class="queue-progress-fill" :style="{ width: `${queueProgressPercent}%` }" />
            </div>
            <div class="queue-status-title" :title="currentDownloadTitle">Downloading: {{ currentDownloadTitle }}</div>
          </div>

          <p v-if="!progress && !video.isPlaylist" class="download-size-preview">Estimated download size <strong>{{ downloadSizeText }}</strong></p>
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
            <template v-if="video.isPlaylist">
              <button v-if="!queueActive" class="button primary" type="button" :disabled="!canDownload || selectedCount === 0" @click="handleBatchDownload">
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

            <template v-if="!video.isPlaylist && progress?.status !== 'finished'">
              <button class="button" type="button" title="Extract manual or automatic subtitles from the site" :disabled="!saveDir || downloading || extractingSubtitles" @click="handleExtractSubtitles">
                <span class="icon" aria-hidden="true" :style="iconStyle('emptyPage')" />
                <span>{{ extractingSubtitles ? 'Extracting…' : extractedSubtitlePath ? 'Extract again' : 'Extract subtitles' }}</span>
              </button>
              <button v-if="extractedSubtitlePath" class="button icon-button" type="button" title="Open Markdown subtitles" aria-label="Open Markdown subtitles" @click="openPath(extractedSubtitlePath)">
                <span class="icon" aria-hidden="true" :style="iconStyle('openNewWindow')" />
              </button>
            </template>

            <template v-if="!downloading && !queueActive && progress?.status === 'finished' && completedFilePath">
              <button class="button primary" type="button" title="Open folder" aria-label="Open folder" @click="handleOpenParentFolder">
                <span class="icon" aria-hidden="true" :style="iconStyle('folder')" />
                <span>Open folder</span>
              </button>
              <button class="button" type="button" @click="handleOpenFile">Open file</button>
              <button class="button" type="button" @click="openSubtitleWorkspace(completedFilePath)">
                <span class="icon" aria-hidden="true" :style="iconStyle('emptyPage')" />Create subtitles
              </button>
            </template>
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
