<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
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
import { useTasksStore } from '@/stores/tasks'
import { useActivityStore } from '@/stores/activity'
import type { CookieSource, DependencyStatus, DownloadHistoryItem, DownloadProgressEvent, ToolInstallEvent, VideoInfo, DownloadMode } from '../types'

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
const parsing = ref(false)
const downloading = ref(false)
const progress = ref<DownloadProgressEvent | null>(null)
const errorMessage = ref('')
const errorDetail = ref('')
const logs = ref<string[]>([])
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
let progressAnimationFrame: number | undefined
let pendingDisplayPercent: number | undefined
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
  if (loadingDeps.value) return '检查中'
  if (!depsChecked.value) return '等待检查'
  if (deps.value?.ytdlp_ok && deps.value?.ffmpeg_ok) return '工具已就绪'
  return `需要安装 ${missingTools.value.join('、')}`
})

const progressPercent = computed(() => {
  return Math.max(0, Math.min(100, displayPercent.value))
})

function resetDisplayedProgress() {
  if (progressAnimationFrame !== undefined) {
    window.cancelAnimationFrame(progressAnimationFrame)
    progressAnimationFrame = undefined
  }
  pendingDisplayPercent = undefined
  displayPercent.value = 0
}

function updateDisplayedProgress(percent: number, terminal = false) {
  const normalized = Math.max(0, Math.min(100, percent))
  const target = terminal ? 100 : Math.min(normalized, 99.8)

  if (terminal) {
    if (progressAnimationFrame !== undefined) {
      window.cancelAnimationFrame(progressAnimationFrame)
      progressAnimationFrame = undefined
    }
    pendingDisplayPercent = undefined
    displayPercent.value = 100
    return
  }

  pendingDisplayPercent = Math.max(displayPercent.value, pendingDisplayPercent ?? 0, target)
  if (progressAnimationFrame !== undefined) return

  progressAnimationFrame = window.requestAnimationFrame(() => {
    displayPercent.value = Math.max(displayPercent.value, pendingDisplayPercent ?? 0)
    pendingDisplayPercent = undefined
    progressAnimationFrame = undefined
  })
}

const queueProgressPercent = computed(() => {
  if (!queueTotal.value) return 0
  const settledItems = queueCompleted.value + queueFailed.value
  const terminal = ['finished', 'cancelled', 'error'].includes(progress.value?.status || '')
  const completedThroughCurrent = terminal ? Math.max(settledItems, queueCurrentIndex.value) : settledItems
  const currentItemProgress = queueActive.value && !terminal && queueCurrentIndex.value > completedThroughCurrent
    ? progressPercent.value / 100
    : 0
  return Math.max(0, Math.min(100, ((completedThroughCurrent + currentItemProgress) / queueTotal.value) * 100))
})

const progressStatusText = computed(() => {
  if (cancelling.value) return '正在取消'
  switch (progress.value?.status) {
    case 'starting':
      return '正在准备'
    case 'downloading':
      return '正在下载'
    case 'processing':
      return '正在合并或转换音视频'
    case 'finished':
      return '已完成'
    case 'cancelled':
      return '已取消'
    case 'error':
      return '下载失败'
    default:
      return '等待开始'
  }
})

const durationText = computed(() => {
  if (!video.value?.duration) return '时长未知'
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
    label: '仅音频 · MP3',
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
  if (copiedHistoryTimer) window.clearTimeout(copiedHistoryTimer)
  if (progressAnimationFrame !== undefined) window.cancelAnimationFrame(progressAnimationFrame)
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
    showError(error, '工具检查失败，请稍后重试。')
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
    showError(error, '无法解析该链接，请确认链接可访问且受 yt-dlp 支持。')
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
  logs.value = []
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
    activity.beginDownload({ title: video.value?.title || '视频下载' })
    const activeCookieSource = video.value?.cookieSource || selectedCookieSource()
    await startDownload({ url: downloadUrl, dir: saveDir.value, mode, formatId, options: { cookieSource: activeCookieSource } })
  } catch (error) {
    if (progress.value?.status !== 'cancelled') {
      showError(error, '下载失败，请查看详情后重试。')
      activity.finishDownload({ state: 'failed', detail: '下载失败，请查看详情后重试。' })
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
    showError(error, '未能提取站点字幕，请查看详情。')
  } finally {
    extractingSubtitles.value = false
  }
}

async function downloadPromise(targetUrl: string, title: string) {
  logs.value = []
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
    showError(new Error('请至少选择一个视频。'), '播放列表中没有选中的视频。')
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
      showError(e, '队列中的视频下载失败，请查看详情。')
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
      ? '下载队列已取消'
      : queueFailed.value
        ? `队列已完成：${queueCompleted.value} 个成功，${queueFailed.value} 个失败`
        : `下载队列已完成：${queueCompleted.value} 个视频`,
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
    showError(error, '取消失败，请重试。')
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
    showError(error, '工具安装失败，请查看详情后重试。')
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
    showError(error, '无法更新 yt-dlp，请检查网络后重试。')
  } finally {
    updatingYtdlp.value = false
  }
}

async function openInstallConfirm() {
  if (!toolsDir.value) {
    try {
      toolsDir.value = await getToolsDirectory()
    } catch (error) {
      showError(error, '无法定位工具目录。')
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
    showError(error, '无法打开文件。')
  }
}

async function handleOpenParentFolder() {
  if (!completedFilePath.value) return
  try {
    await openParentFolder(completedFilePath.value)
  } catch (error) {
    showError(error, '无法打开文件夹。')
  }
}

function handleProgress(event: DownloadProgressEvent) {
  if (event.line) {
    logs.value.push(event.line)
    if (logs.value.length > 1000) logs.value.splice(0, logs.value.length - 1000)
  }
  if (event.status === 'log') return

  const nextEvent = event.status === 'finished' ? { ...event, percent: event.percent ?? 100 } : event
  progress.value = nextEvent
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
    showError(error, '无法复制链接。')
  }
}

async function openHistoryFolder(item: DownloadHistoryItem) {
  try {
    await openParentFolder(item.filePath)
  } catch (error) {
    showError(error, '无法打开历史任务文件夹。')
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
      ? '未发现站点字幕。可点击“创建字幕”使用语音识别生成字幕。'
      : '未发现站点字幕。该视频未提供人工或自动字幕。'
  }
  if (text.includes('未检测到 yt-dlp') || (text.includes('yt-dlp') && (text.includes('not found') || text.includes('no such file') || text.includes('unable to execute')))) {
    return '无法启动 yt-dlp。请先检查工具，缺失时再安装。'
  }
  if (text.includes('ffmpeg') && (text.includes('not found') || text.includes('no such file') || text.includes('unable to execute'))) {
    return '缺少 ffmpeg，无法完成视频合并或音频转换。'
  }
  if (text.includes('cloudflare') || text.includes('anti-bot challenge') || text.includes('http error 403')) {
    return '网站拦截了请求。请尝试添加 cookies.txt，或稍后重试。'
  }
  if (text.includes('bilibili') && (text.includes('http error 412') || text.includes('precondition failed'))) {
    return 'Bilibili rejected the metadata request. Add cookies.txt and try again.'
  }
  if (text.includes('older than 90 days') || text.includes('use that to update')) {
    return deps.value?.ytdlp_update_available
      ? 'yt-dlp is outdated. Update it, then try again.'
      : 'yt-dlp 版本较旧，但没有找到可用更新。请检查工具路径或稍后重试。'
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
    return '网络请求失败，请检查网络或代理后重试。'
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
  catch (error) { showError(error, '文件已下载，但无法保存历史记录。') }
}
function downloadNext() {
  url.value = ''
  video.value = null
  progress.value = null
  completedFilePath.value = ''
  extractedSubtitlePath.value = ''
  logs.value = []
  clearError()
  void nextTick(() => urlInput.value?.focus())
}

function formatHistoryTime(value: string) {
  return new Date(value).toLocaleString()
}</script>

<template>
  <section class="download-page" :class="{ dragging: dragActive }" @dragover="handleDragOver" @dragleave="handleDragLeave" @drop="handleDrop">
    <div class="content-scroll">
    <section class="hero">
      <header class="download-intro">
        <h1>视频下载</h1>
        <p>粘贴链接，选择清晰度和保存位置。</p>
      </header>
      <div class="input-row-container">
        <div class="input-row">
          <input
            ref="urlInput"
            aria-label="视频或音频链接"
            v-model="url"
            class="url-input"
            type="url"
            placeholder="粘贴视频链接"
            :disabled="parsing || downloading"
            @keydown.enter="handleParse"
          />
          <button
            class="button cookie-file-button icon-only"
            type="button"
            :disabled="parsing || downloading"
            :title="cookiesFilePath || '添加 cookies.txt'"
            :aria-label="cookiesFilePath ? '更换 cookies.txt' : '添加 cookies.txt'"
            @click="handleSelectCookiesFile"
          >
            <span class="icon" aria-hidden="true" :style="iconStyle('cookie')" />
          </button>
          <button class="button primary parse-button" :class="{ parsing }" type="button" :disabled="!canParse" @click="handleParse">
            <span class="icon" aria-hidden="true" :style="iconStyle('mediaVideo')" />
            <span>{{ parsing ? '正在解析' : '解析视频' }}</span>
          </button>
        </div>
        <div v-if="cookiesFilePath" class="cookies-inline-status">
          <span :title="cookiesFilePath">cookies.txt: {{ cookiesFilePath }}</span>
          <button class="link-button compact" type="button" :disabled="parsing || downloading || extractingSubtitles" @click="clearCookiesFile">清除</button>
        </div>
        <Transition name="fade">
          <div v-if="showClipboardTip" class="clipboard-tip-bubble">
            <span class="tip-text" :title="clipboardTipUrl">检测到剪贴板链接：{{ clipboardTipUrl }}</span>
            <div class="tip-actions">
              <button class="tip-btn primary" type="button" @click="handleUseClipboardUrl">使用</button>
              <button class="tip-btn" type="button" @click="showClipboardTip = false">忽略</button>
            </div>
          </div>
        </Transition>
      </div>

      <Transition name="fade">
        <section v-if="depsChecked && hasMissingTools" class="download-readiness-card" aria-live="polite">
          <span class="download-readiness-mark"><span class="icon" aria-hidden="true" :style="iconStyle('settings')" /></span>
          <div>
            <strong>下载前需要安装 {{ missingTools.join('、') }}</strong>
            <p>仅安装当前下载所需的工具；不会额外安装其他下载器。</p>
          </div>
          <button class="button" type="button" :disabled="installing" @click="openInstallConfirm">
            {{ installing ? '正在安装' : '去安装' }}
          </button>
        </section>
      </Transition>

      <details class="tool-disclosure">
        <summary>
          <span class="status-square" :class="{ ok: deps?.ytdlp_ok && deps?.ffmpeg_ok, bad: depsChecked && hasMissingTools }" />
          <span>{{ toolStatusText }}</span>
          <span class="tool-summary-hint">下载工具</span>
        </summary>
        <div class="tool-disclosure-content">
          <div class="env-row">
            <div class="env-item" :class="{ ok: deps?.ytdlp_ok, bad: deps && !deps.ytdlp_ok }">
              <span class="status-square" />
              <span :title="deps?.ytdlp_path || ''">yt-dlp {{ deps?.ytdlp_ok ? `可用 ${deps.ytdlp_version || ''}` : '缺失' }}</span>
            </div>
            <div class="env-item" :class="{ ok: deps?.ffmpeg_ok, bad: deps && !deps.ffmpeg_ok }">
              <span class="status-square" />
              <span :title="deps?.ffmpeg_path || ''">ffmpeg {{ deps?.ffmpeg_ok ? '可用' : '缺失' }}</span>
            </div>
            <button class="link-button" type="button" :disabled="loadingDeps || parsing || downloading || extractingSubtitles" @click="handleCheckTools">
              <span class="icon" aria-hidden="true" :style="iconStyle('check')" />
              <span>{{ loadingDeps ? '正在检查' : '重新检查' }}</span>
            </button>
            <button v-if="deps?.ytdlp_update_available" class="link-button" type="button" :disabled="updatingYtdlp" @click="handleUpdateYtdlp">
              <span class="icon" aria-hidden="true" :style="iconStyle('refresh')" />
              <span>{{ updatingYtdlp ? '正在更新' : '更新 yt-dlp' }}</span>
            </button>
            <button v-if="hasMissingTools" class="link-button" type="button" @click="openInstallConfirm">
              <span class="icon" aria-hidden="true" :style="iconStyle('settings')" />
              <span>安装缺失工具</span>
            </button>
          </div>
          <div v-if="deps?.ytdlp_ok || deps?.ffmpeg_ok" class="tool-paths">
            <span v-if="deps?.ytdlp_ok">yt-dlp: {{ deps.ytdlp_path }}</span>
            <span v-if="deps?.ffmpeg_ok">ffmpeg: {{ deps.ffmpeg_path }}</span>
          </div>
        </div>
      </details>

    </section>

    <Transition name="reveal">
      <section v-if="video" ref="detailsSection" class="details">
        <div class="preview">
          <div class="thumb">
            <img v-if="video.thumbnail" :src="video.thumbnail" alt="" />
          </div>
          <div class="video-meta">
            <span class="label">{{ video.extractor || 'video' }}</span>
            <h2>{{ video.title }}</h2>
            <p v-if="video.uploader || video.duration">{{ video.uploader || '作者未知' }} · {{ durationText }}</p>
            <p class="parse-note">{{ video.parseStrategy }} · {{ video.site }}</p>
          </div>
        </div>

        <div v-if="video.isPlaylist" class="playlist-panel">
          <div class="playlist-header">
            <span>播放列表 {{ video.entries?.filter(e => selectedEntries[e.url]).length || 0 }}/{{ video.entries?.length || 0 }}</span>
            <div class="playlist-actions-quick">
              <button class="link-button compact" type="button" title="全选" aria-label="全选" @click="selectAllEntries(true)">
                <span class="icon" aria-hidden="true" :style="iconStyle('checkSquare')" />
              </button>
              <span class="divider">|</span>
              <button class="link-button compact" type="button" title="取消全选" aria-label="取消全选" @click="selectAllEntries(false)">
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
            <span>下载格式</span>
            <AppSelect
              v-model="selectedFormatId"
              :options="formatOptions"
              :disabled="downloading || queueActive || extractingSubtitles"
              aria-label="下载格式"
            />
          </div>

          <div class="path-row">
            <div class="path-display" :title="saveDir || '尚未选择文件夹'">{{ saveDir || '选择保存文件夹' }}</div>
            <button class="button icon-button" type="button" title="选择文件夹" aria-label="选择文件夹" :disabled="downloading || queueActive || extractingSubtitles" @click="handleSelectDir">
              <span class="icon" aria-hidden="true" :style="iconStyle('folder')" />
            </button>
          </div>
          <label class="check-row">
            <input v-model="setAsDefault" type="checkbox" :disabled="downloading || queueActive || extractingSubtitles" @change="handleDefaultToggle" />
            <span>设为默认文件夹</span>
          </label>

          <div v-if="queueActive" class="queue-status-panel">
            <div class="queue-status-head">
              <span>队列 {{ queueCurrentIndex }}/{{ queueTotal }}</span>
              <span>完成 {{ queueCompleted }} · 失败 {{ queueFailed }}</span>
            </div>
            <div class="queue-progress-track">
              <div class="queue-progress-fill" :style="{ width: `${queueProgressPercent}%` }" />
            </div>
            <div class="queue-status-title" :title="currentDownloadTitle">正在下载：{{ currentDownloadTitle }}</div>
          </div>

          <div v-if="progress" class="progress-panel">
            <div class="progress-head">
              <span>{{ progressStatusText }}</span>
              <strong>{{ progressPercent.toFixed(1) }}%</strong>
            </div>
            <div class="progress-track" role="progressbar" aria-label="下载进度" :aria-valuenow="progress.status === 'processing' ? undefined : progressPercent" :aria-valuetext="progressStatusText" :aria-valuemin="0" :aria-valuemax="100">
              <div class="progress-fill" :style="{ width: `${progressPercent}%` }" />
            </div>
            <div v-if="progress.status === 'downloading'" class="progress-foot">
              <span>速度 {{ progress.speed || '--' }}</span>
              <span>剩余 {{ progress.eta || '--' }}</span>
              <span>大小 {{ progress.total || '--' }}</span>
            </div>
          </div>

          <div class="download-actions">
            <template v-if="video.isPlaylist">
              <button v-if="!queueActive" class="button primary" type="button" :disabled="!canDownload || selectedCount === 0" @click="handleBatchDownload">
                <span class="icon" aria-hidden="true" :style="iconStyle('download')" />
                <span>下载 {{ video.entries?.filter(e => selectedEntries[e.url]).length || 0 }} 个视频</span>
              </button>
              <button v-else class="button danger" type="button" :disabled="cancelling" @click="cancelQueue">
                <span class="icon" aria-hidden="true" :style="iconStyle('xmark')" />
                <span>取消</span>
              </button>
            </template>
            <template v-else>
              <button v-if="!downloading && progress?.status !== 'finished'" class="button primary" type="button" :disabled="!canDownload" @click="handleDownload()">
                <span class="icon" aria-hidden="true" :style="iconStyle('download')" />
                <span>开始下载</span>
              </button>
              <button v-if="downloading" class="button" type="button" :disabled="cancelling" @click="handleCancel">
                <span class="icon" aria-hidden="true" :style="iconStyle('xmark')" />
                <span>{{ cancelling ? '正在取消…' : '取消下载' }}</span>
              </button>
            </template>

            <template v-if="!video.isPlaylist && progress?.status !== 'finished'">
              <button class="button" type="button" title="提取视频页面提供的人工或自动字幕；没有时可下载视频后创建字幕" :disabled="!saveDir || downloading || extractingSubtitles" @click="handleExtractSubtitles">
                <span class="icon" aria-hidden="true" :style="iconStyle('emptyPage')" />
                <span>{{ extractingSubtitles ? '正在提取' : extractedSubtitlePath ? '重新提取站点字幕' : '提取站点字幕' }}</span>
              </button>
              <button v-if="extractedSubtitlePath" class="button icon-button" type="button" title="打开 Markdown 字幕" aria-label="打开 Markdown 字幕" @click="openPath(extractedSubtitlePath)">
                <span class="icon" aria-hidden="true" :style="iconStyle('openNewWindow')" />
              </button>
            </template>

            <template v-if="!downloading && !queueActive && progress?.status === 'finished' && completedFilePath">
              <button class="button primary" type="button" title="打开文件夹" aria-label="打开文件夹" @click="handleOpenParentFolder">
                <span class="icon" aria-hidden="true" :style="iconStyle('folder')" />
                <span>打开文件夹</span>
              </button>
              <button class="button" type="button" @click="downloadNext">下载下一个</button>
              <button class="button" type="button" @click="handleOpenFile">打开文件</button>
              <details class="completion-more">
                <summary>更多操作</summary>
                <button class="button" type="button" :disabled="!canDownload" @click="handleDownload()">重新下载</button>
                <button class="button" type="button" @click="openSubtitleWorkspace(completedFilePath)">创建字幕</button>
              </details>
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
          <span>错误详情</span>
        </summary>
        <div class="details-content">
          <pre>{{ errorDetail }}</pre>
        </div>
      </details>
    </div>
    <details v-if="logs.length" class="log-box" @toggle="logsOpen = ($event.target as HTMLDetailsElement).open">
      <summary>
        <span class="icon disclosure-icon" aria-hidden="true" :style="iconStyle('navArrowRight')" />
        <span>运行日志</span>
      </summary>
      <div class="details-content">
        <pre v-if="logsOpen">{{ logs.join('\n') }}</pre>
      </div>
    </details>
    </div>
    <AppDialog :open="showInstallConfirm" :title="installing ? '正在安装工具' : '安装缺失工具？'" :busy="installing" @close="showInstallConfirm = false">
        <template v-if="!installing">
          <p class="modal-label">需要安装：</p>
          <ul>
            <li v-for="tool in missingTools" :key="tool">{{ tool }}</li>
          </ul>
          <p class="modal-label">安装位置：</p>
          <code>{{ toolsDir }}</code>
          <div class="modal-actions">
            <button class="button" type="button" @click="showInstallConfirm = false">取消</button>
            <button class="button primary" type="button" @click="handleInstallTools">开始安装</button>
          </div>
        </template>
        <template v-else>
          <div class="install-list">
            <div v-for="tool in missingTools" :key="tool" class="install-row">
              <span>{{ tool }}</span>
              <span>{{ installEvents[tool]?.message || '等待中' }}</span>
              <strong>{{ installEvents[tool]?.percent ? `${installEvents[tool]?.percent?.toFixed(0)}%` : '' }}</strong>
            </div>
          </div>
          <div class="progress-track">
            <div class="progress-fill" :style="{ width: `${Math.max(...missingTools.map((tool) => installEvents[tool]?.percent || 0), 0)}%` }" />
          </div>
        </template>
    </AppDialog>

    <div v-if="dragActive" class="drop-overlay">拖放链接到这里</div>
  </section>
</template>
