<script setup lang="ts">
import { computed, nextTick, onActivated, onDeactivated, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import checkIcon from 'iconoir/icons/check.svg?url'
import folderIcon from 'iconoir/icons/folder.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import pageIcon from 'iconoir/icons/empty-page.svg?url'
import settingsIcon from 'iconoir/icons/settings.svg?url'
import AppSelect from './components/ui/select/AppSelect.vue'
import {
  analyzeSubtitleSource,
  burnSubtitles,
  cancelSubtitleTask,
  createSubtitleProject,
  downloadWhisperModel,
  downloadWhisperRuntime,
  exportSubtitles,
  getGeminiSettings,
  getSubtitleProject,
  importSubtitleTrack,
  listSubtitleProjects,
  listWhisperModels,
  listWhisperRuntimes,
  mediaUrl,
  onSubtitleProgress,
  openParentFolder,
  openPath,
  saveSubtitleProjectWorkspace,
  saveSubtitleSegments,
  selectMediaFile,
  selectOutputFile,
  startBingTranslation,
  startOpenAiCompatibleTranslation,
  startWhisperTranscription,
} from './api/tauri'
import { useActivityStore } from './stores/activity'
import type {
  GeminiSettings,
  MediaSubtitleAnalysis,
  SubtitleExportFormat,
  SubtitleProgressEvent,
  SubtitleProject,
  SubtitleContentMode,
  SubtitleSegment,
  SubtitleStyle,
  WhisperModelInfo,
  WhisperRuntimeInfo,
} from './types'

const props = defineProps<{ initialSourcePath?: string; initialProjectId?: string }>()
const emit = defineEmits<{ consumedInitial: []; consumedProject: [] }>()
const router = useRouter()
const activity = useActivityStore()

const sourcePath = ref('')
const analysis = ref<MediaSubtitleAnalysis | null>(null)
const project = ref<SubtitleProject | null>(null)
const settings = ref<GeminiSettings>({
  hasApiKey: false,
  defaultModel: 'gemini-3.1-flash-lite',
  defaultTargetLanguage: 'zh-CN',
  maxCostUsd: 2,
  maxConcurrency: 2,
  processingMode: 'local_free',
  whisperModel: 'large-v3-turbo-q5',
  whisperRuntime: 'cpu',
  hasOpenaiApiKey: false,
  openaiApiBase: 'https://api.openai.com/v1',
  openaiModel: '',
  hasGlmApiKey: false,
})
const burnVideo = ref(true)
type SubtitlePreset = 'classic' | 'contrast' | 'yellow' | 'custom'
type QaIssue = {
  segmentId: string
  severity: 'error' | 'warning'
  message: string
}
type ReadinessAction = 'install-model' | 'install-runtime' | 'open-settings'
type ReadinessCard = {
  title: string
  description: string
  action: ReadinessAction
  actionLabel: string
  setup?: 'custom' | 'local'
}

const subtitleStyleStorageKey = 'ydlite.subtitle-style.v1'
const showAdvanced = ref(false)
const stylePreset = ref<SubtitlePreset>('classic')
const contentMode = ref<SubtitleContentMode>('bilingual')
const syncTranslatedStyle = ref(true)
const advancedStyleOpen = ref(false)
const subtitleStyle = ref<SubtitleStyle>({
  fontFamily: 'Microsoft YaHei',
  fontSize: 48,
  translatedFontSize: 48,
  primaryColor: '#F8F8F8',
  translatedColor: '#F8F8F8',
  outlineColor: '#151515',
  backgroundColor: '#151515',
  backgroundOpacity: 0,
  outlineWidth: 3,
  shadow: 1,
  bold: false,
  boxed: false,
  position: 'bottom',
  marginVertical: 64,
  translatedFirst: true,
})

const subtitlePresets: Array<{ id: Exclude<SubtitlePreset, 'custom'>; name: string; note: string; color: string; boxed: boolean }> = [
  { id: 'classic', name: '经典白字', note: '清晰描边', color: '#F8F8F8', boxed: false },
  { id: 'contrast', name: '高对比', note: '半透明底', color: '#F8F8F8', boxed: true },
  { id: 'yellow', name: '黄色强调', note: '教程短视频', color: '#FFD95A', boxed: false },
]
const contentOptions = [
  { value: 'source', label: '仅源字幕' },
  { value: 'translated', label: '仅翻译字幕' },
  { value: 'bilingual', label: '双语字幕' },
]
const fontOptions = [
  { value: 'Microsoft YaHei', label: '微软雅黑' },
  { value: 'Microsoft YaHei UI', label: '微软雅黑 UI' },
  { value: 'SimHei', label: '黑体' },
  { value: 'SimSun', label: '宋体' },
  { value: 'Segoe UI', label: 'Segoe UI' },
]
const positionOptions = [
  { value: 'bottom', label: '底部' },
  { value: 'middle', label: '居中' },
  { value: 'top', label: '顶部' },
]
const availableContentOptions = computed(() =>
  analysis.value?.strategy === 'extract_chinese' ? contentOptions.slice(0, 1) : contentOptions,
)
const analyzing = ref(false)
const processing = ref(false)
const progress = ref<SubtitleProgressEvent | null>(null)
const error = ref('')
const notice = ref('')
const subtitleOutput = ref('')
const videoOutput = ref('')
const localModelReady = ref(false)
const localRuntimeReady = ref(false)
const localModels = ref<WhisperModelInfo[]>([])
const localRuntimes = ref<WhisperRuntimeInfo[]>([])
const preparingAsset = ref<'model' | 'runtime' | null>(null)
const resultPanel = ref<HTMLElement | null>(null)
const workbenchPanel = ref<HTMLElement | null>(null)
const previewVideo = ref<HTMLVideoElement | null>(null)
const previewTimeMs = ref(0)
const selectedSegmentId = ref('')
const savingSegments = ref(false)
const workspaceSaving = ref(false)
const exporting = ref(false)
const editorError = ref('')
const editorNotice = ref('')
const exportFormat = ref<SubtitleExportFormat>('srt')
const reviewStatus = ref<'draft' | 'reviewing' | 'approved'>('draft')
const undoStack = ref<SubtitleSegment[][]>([])
const redoStack = ref<SubtitleSegment[][]>([])
const lastSavedSegments = ref('')
const segmentQuery = ref('')
const replacementText = ref('')
let segmentSaveTimer: number | undefined
let workspaceSaveTimer: number | undefined
let unlisten: undefined | (() => void)

const needsLocalAssets = computed(() =>
  analysis.value?.strategy === 'transcribe_audio',
)
const needsCustomAi = computed(() =>
  settings.value.processingMode === 'local_custom' && analysis.value?.strategy !== 'extract_chinese',
)
const selectedWhisperModel = computed(() =>
  localModels.value.find(item => item.id === settings.value.whisperModel) ?? null,
)
const selectedWhisperRuntime = computed(() =>
  localRuntimes.value.find(item => item.id === settings.value.whisperRuntime) ?? null,
)
const canGenerate = computed(() =>
  Boolean(
    analysis.value
    && !analyzing.value
    && !processing.value
    && (!needsCustomAi.value || (settings.value.hasOpenaiApiKey && Boolean(settings.value.openaiModel)))
    && (!needsLocalAssets.value || (localModelReady.value && localRuntimeReady.value)),
  ),
)
const actionLabel = computed(() => {
  if (!analysis.value) return '选择视频'
  if (processing.value) return '正在自动生成…'
  return '开始自动字幕'
})
const stageLabel = computed(() => {
  if (!progress.value) return ''
  if (progress.value.stage === 'rate_limited' && progress.value.message) {
    return progress.value.message
  }
  const labels: Record<string, string> = {
    extracting: '正在识别语音',
    uploading: '正在识别语音',
    transcribing: '正在识别语音',
    whisper_transcription: '正在识别语音',
    bing_translation: '正在翻译字幕',
    custom_ai_translation: '正在翻译字幕',
    translate: '正在翻译字幕',
    polish: '正在自动校对',
    exporting: '正在生成结果',
    burning: '正在生成结果',
    completed: '处理完成',
  }
  return labels[progress.value.stage] ?? '正在自动处理'
})
const automationFlowLabel = computed(() =>
  analysis.value?.strategy === 'extract_chinese'
    ? '提取字幕 → 生成结果'
    : '识别语音 → 翻译 → 生成结果',
)
const processingPlan = computed(() => {
  if (!analysis.value) return '等待视频'
  if (analysis.value.strategy === 'extract_chinese') return '直接提取'
  if (settings.value.processingMode === 'local_custom') {
    return analysis.value.strategy === 'translate_subtitle' ? '自定义 AI 翻译' : 'Whisper + 自定义 AI'
  }
  return analysis.value.strategy === 'translate_subtitle' ? '必应翻译' : 'Whisper + 必应'
})
const processingTimeEstimate = computed(() => {
  if (!analysis.value) return ''
  if (analysis.value.strategy === 'extract_chinese') return '预计不到 1 分钟'
  const durationMinutes = Math.max(1, analysis.value.durationMs / 60_000)
  let low = 0.5
  let high = 2
  if (analysis.value.strategy === 'transcribe_audio') {
    if (settings.value.whisperRuntime === 'cuda') {
      low = durationMinutes * 0.2 + 0.5
      high = durationMinutes * 0.45 + 1
    } else {
      low = durationMinutes * 0.65 + 1
      high = durationMinutes * 1.4 + 2
    }
  }
  if (burnVideo.value) {
    low += durationMinutes * 0.35
    high += durationMinutes * 0.8
  }
  return `预计 ${Math.max(1, Math.ceil(low))}–${Math.max(2, Math.ceil(high))} 分钟`
})
const outputHeading = computed(() =>
  videoOutput.value || (!burnVideo.value && subtitleOutput.value) ? '处理完成' : '字幕已生成',
)
const generateHint = computed(() => {
  if (!analysis.value) return ''
  if (needsCustomAi.value && (!settings.value.hasOpenaiApiKey || !settings.value.openaiModel)) return '请先配置自定义 AI'
  if (needsLocalAssets.value && !localModelReady.value) return '请先下载本地模型'
  if (needsLocalAssets.value && !localRuntimeReady.value) return '请先下载运行组件'
  return ''
})
const readinessCard = computed<ReadinessCard | null>(() => {
  if (!analysis.value || canGenerate.value || processing.value || analyzing.value) return null
  if (needsCustomAi.value && (!settings.value.hasOpenaiApiKey || !settings.value.openaiModel)) {
    return {
      title: '需要配置自定义 AI',
      description: '填写 API 地址、模型名称和 Key 后，才会使用你的翻译服务。',
      action: 'open-settings',
      actionLabel: '配置自定义 AI',
      setup: 'custom',
    }
  }
  if (needsLocalAssets.value && !localModelReady.value) {
    const model = selectedWhisperModel.value
    return {
      title: `需要下载 ${model?.name ?? 'Whisper 模型'}`,
      description: model ? `${model.description} ${model.downloadSize}，只会下载这一个模型。` : '模型仅保存在本机，不会自动下载。',
      action: 'install-model',
      actionLabel: preparingAsset.value === 'model' ? '正在下载…' : `下载${model ? ` ${model.downloadSize}` : ''}`,
      setup: 'local',
    }
  }
  if (needsLocalAssets.value && !localRuntimeReady.value) {
    const runtime = selectedWhisperRuntime.value
    return {
      title: `需要安装 ${runtime?.name ?? 'Whisper 运行组件'}`,
      description: runtime ? `${runtime.description} ${runtime.downloadSize}，仅用于本机语音识别。` : '运行组件仅在开始本地识别前安装。',
      action: 'install-runtime',
      actionLabel: preparingAsset.value === 'runtime' ? '正在安装…' : `安装${runtime ? ` ${runtime.downloadSize}` : ''}`,
      setup: 'local',
    }
  }
  return null
})
const resolvedSubtitleStyle = computed<SubtitleStyle>(() => ({
  ...subtitleStyle.value,
  translatedFontSize: syncTranslatedStyle.value
    ? subtitleStyle.value.fontSize
    : subtitleStyle.value.translatedFontSize,
  translatedColor: syncTranslatedStyle.value
    ? subtitleStyle.value.primaryColor
    : subtitleStyle.value.translatedColor,
}))
const previewPositionClass = computed(() => `position-${subtitleStyle.value.position}`)
const previewBoxStyle = computed(() => ({
  backgroundColor: subtitleStyle.value.boxed
    ? `${subtitleStyle.value.backgroundColor}${Math.round(subtitleStyle.value.backgroundOpacity * 2.55).toString(16).padStart(2, '0')}`
    : 'transparent',
}))
const sourcePreviewStyle = computed(() => previewTextStyle(
  subtitleStyle.value.fontSize,
  subtitleStyle.value.primaryColor,
))
const translatedPreviewStyle = computed(() => previewTextStyle(
  resolvedSubtitleStyle.value.translatedFontSize,
  resolvedSubtitleStyle.value.translatedColor,
))
const previewSource = computed(() => sourcePath.value ? mediaUrl(sourcePath.value) : '')
const hasProjectSegments = computed(() => Boolean(project.value?.segments.length))
const selectedSegment = computed(() =>
  project.value?.segments.find(segment => segment.id === selectedSegmentId.value) ?? null,
)
const activePreviewSegment = computed(() => {
  const current = previewTimeMs.value
  return project.value?.segments.find(segment =>
    segment.startMs <= current && current < segment.endMs,
  ) ?? null
})
const hasUnsavedSegments = computed(() =>
  Boolean(project.value && segmentSignature(project.value.segments) !== lastSavedSegments.value),
)
const projectStatusText = computed(() => {
  const status = project.value?.status
  if (status === 'paused') return '任务已暂停，可继续处理'
  if (status === 'failed') return '上次处理失败，可重试'
  if (status === 'processing' || status === 'translate' || status === 'burning') return '项目正在处理'
  if (hasUnsavedSegments.value) return '有未保存的字幕修改'
  if (reviewStatus.value === 'approved') return '已确认，可交付'
  return hasProjectSegments.value ? '待校对' : '等待生成字幕'
})
const qaIssues = computed<QaIssue[]>(() => {
  const segments = project.value?.segments ?? []
  const issues: QaIssue[] = []
  let previousEnd = 0
  const requireTranslation = contentMode.value !== 'source'

  for (const [index, segment] of segments.entries()) {
    const source = segment.sourceText.trim()
    const translated = segment.translatedText?.trim() ?? ''
    const duration = segment.endMs - segment.startMs

    if (!source) {
      issues.push({ segmentId: segment.id, severity: 'error', message: `第 ${index + 1} 条缺少原文` })
    }
    if (segment.endMs <= segment.startMs) {
      issues.push({ segmentId: segment.id, severity: 'error', message: `第 ${index + 1} 条时间无效` })
    }
    if (index > 0 && segment.startMs < previousEnd) {
      issues.push({ segmentId: segment.id, severity: 'error', message: `第 ${index + 1} 条与上一条重叠` })
    }
    if (requireTranslation && !translated) {
      issues.push({ segmentId: segment.id, severity: 'error', message: `第 ${index + 1} 条缺少译文` })
    }
    if (duration > 0 && duration < 650) {
      issues.push({ segmentId: segment.id, severity: 'warning', message: `第 ${index + 1} 条停留不足 0.65 秒` })
    }

    const visibleText = contentMode.value === 'source'
      ? source
      : contentMode.value === 'translated'
        ? translated
        : `${source}${translated}`
    const characterCount = visibleText.replace(/\s/g, '').length
    const seconds = Math.max(duration / 1_000, 0.1)
    if (characterCount / seconds > 16) {
      issues.push({ segmentId: segment.id, severity: 'warning', message: `第 ${index + 1} 条阅读速度偏快` })
    }
    if (visibleText.split('\n').length > 2 || characterCount > 42) {
      issues.push({ segmentId: segment.id, severity: 'warning', message: `第 ${index + 1} 条文字较长，建议断句` })
    }
    previousEnd = Math.max(previousEnd, segment.endMs)
  }
  return issues
})
const qaErrors = computed(() => qaIssues.value.filter(issue => issue.severity === 'error'))
const qaWarnings = computed(() => qaIssues.value.filter(issue => issue.severity === 'warning'))
const issueMap = computed(() => {
  const result = new Map<string, QaIssue[]>()
  for (const issue of qaIssues.value) {
    const current = result.get(issue.segmentId) ?? []
    current.push(issue)
    result.set(issue.segmentId, current)
  }
  return result
})
const visibleSegments = computed(() => {
  const segments = project.value?.segments ?? []
  const query = segmentQuery.value.trim().toLocaleLowerCase()
  return segments
    .map((segment, index) => ({ segment, index }))
    .filter(({ segment }) => !query || `${segment.sourceText}\n${segment.translatedText ?? ''}`.toLocaleLowerCase().includes(query))
})

function previewTextStyle(fontSize: number, color: string) {
  const outline = Math.max(1, subtitleStyle.value.outlineWidth / 2)
  return {
    color,
    fontFamily: `"${subtitleStyle.value.fontFamily}", sans-serif`,
    fontSize: `${Math.max(12, fontSize * 0.34)}px`,
    fontWeight: subtitleStyle.value.bold ? '700' : '500',
    textShadow: subtitleStyle.value.boxed
      ? 'none'
      : `${outline}px ${outline}px 0 ${subtitleStyle.value.outlineColor}, -${outline}px -${outline}px 0 ${subtitleStyle.value.outlineColor}, ${outline}px -${outline}px 0 ${subtitleStyle.value.outlineColor}, -${outline}px ${outline}px 0 ${subtitleStyle.value.outlineColor}`,
  }
}

function cloneSegments(segments: SubtitleSegment[]) {
  return segments.map(segment => ({ ...segment }))
}

function segmentSignature(segments: SubtitleSegment[]) {
  return JSON.stringify(segments.map(segment => ({
    id: segment.id,
    startMs: segment.startMs,
    endMs: segment.endMs,
    sourceText: segment.sourceText,
    translatedText: segment.translatedText ?? null,
  })))
}

function parentDirectory(path: string) {
  const separator = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return separator >= 0 ? path.slice(0, separator) : ''
}

function sourceFileStem(path: string) {
  const filename = path.slice(Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/')) + 1)
  return filename.replace(/\.[^./\\]+$/, '') || '字幕项目'
}

function joinOutputPath(directory: string, filename: string) {
  if (!directory) return filename
  const separator = directory.includes('\\') ? '\\' : '/'
  return `${directory.replace(/[\\/]+$/, '')}${separator}${filename}`
}

function synchronizeProject(value: SubtitleProject, restoreWorkspace = false) {
  project.value = value
  restoreArtifacts(value)
  lastSavedSegments.value = segmentSignature(value.segments)
  selectedSegmentId.value = value.segments[0]?.id ?? ''
  undoStack.value = []
  redoStack.value = []
  if (restoreWorkspace) restoreProjectWorkspace(value)
}

function restoreProjectWorkspace(value: SubtitleProject) {
  const workspace = value.workspace
  if (!workspace) return
  Object.assign(subtitleStyle.value, workspace.style)
  stylePreset.value = 'custom'
  contentMode.value = workspace.exportContent
  burnVideo.value = workspace.burnVideo
  syncTranslatedStyle.value = workspace.syncTranslatedStyle
  exportFormat.value = workspace.exportFormat
  reviewStatus.value = workspace.reviewStatus
}

function workspacePayload() {
  return {
    reviewStatus: reviewStatus.value,
    style: { ...resolvedSubtitleStyle.value },
    exportContent: contentMode.value,
    exportFormat: exportFormat.value,
    burnVideo: burnVideo.value,
    syncTranslatedStyle: syncTranslatedStyle.value,
    outputDir: (project.value?.workspace.outputDir ?? parentDirectory(sourcePath.value)) || null,
  }
}

async function persistWorkspace(quiet = false) {
  if (!project.value || workspaceSaving.value) return
  workspaceSaving.value = true
  try {
    const saved = await saveSubtitleProjectWorkspace(project.value.id, workspacePayload())
    project.value = saved
    if (!quiet) notice.value = '项目设置已保存。'
  } catch (value) {
    error.value = errorText(value)
  } finally {
    workspaceSaving.value = false
  }
}

function scheduleWorkspaceSave() {
  if (!project.value) return
  if (workspaceSaveTimer) window.clearTimeout(workspaceSaveTimer)
  workspaceSaveTimer = window.setTimeout(() => void persistWorkspace(true), 650)
}

function rememberSegments() {
  if (!project.value) return
  const snapshot = cloneSegments(project.value.segments)
  const previous = undoStack.value.at(-1)
  if (previous && segmentSignature(previous) === segmentSignature(snapshot)) return
  undoStack.value = [...undoStack.value.slice(-49), snapshot]
  redoStack.value = []
}

function replaceSegments(segments: SubtitleSegment[]) {
  if (!project.value) return
  project.value = { ...project.value, segments }
  scheduleSegmentSave()
}

function scheduleSegmentSave() {
  if (segmentSaveTimer) window.clearTimeout(segmentSaveTimer)
  segmentSaveTimer = window.setTimeout(() => void saveSegmentEdits(true), 850)
}

async function saveSegmentEdits(quiet = false) {
  if (!project.value || savingSegments.value || !hasUnsavedSegments.value) return true
  savingSegments.value = true
  editorError.value = ''
  try {
    const saved = await saveSubtitleSegments(project.value.id, project.value.segments)
    project.value = saved
    lastSavedSegments.value = segmentSignature(saved.segments)
    if (reviewStatus.value === 'draft') reviewStatus.value = 'reviewing'
    scheduleWorkspaceSave()
    if (!quiet) editorNotice.value = '字幕修改已保存。'
    return true
  } catch (value) {
    editorError.value = errorText(value)
    return false
  } finally {
    savingSegments.value = false
  }
}

function undoSegmentEdit() {
  if (!project.value) return
  const snapshot = undoStack.value.at(-1)
  if (!snapshot) return
  redoStack.value = [...redoStack.value.slice(-49), cloneSegments(project.value.segments)]
  undoStack.value = undoStack.value.slice(0, -1)
  replaceSegments(cloneSegments(snapshot))
}

function redoSegmentEdit() {
  if (!project.value) return
  const snapshot = redoStack.value.at(-1)
  if (!snapshot) return
  undoStack.value = [...undoStack.value.slice(-49), cloneSegments(project.value.segments)]
  redoStack.value = redoStack.value.slice(0, -1)
  replaceSegments(cloneSegments(snapshot))
}

function updateSegmentText(segmentId: string, field: 'sourceText' | 'translatedText', event: Event) {
  if (!project.value) return
  const value = (event.target as HTMLTextAreaElement).value
  const index = project.value.segments.findIndex(segment => segment.id === segmentId)
  if (index < 0) return
  const segments = cloneSegments(project.value.segments)
  segments[index] = {
    ...segments[index],
    [field]: field === 'translatedText' ? value : value,
  }
  replaceSegments(segments)
}

function formatTimestamp(value: number) {
  const hours = Math.floor(value / 3_600_000)
  const minutes = Math.floor((value % 3_600_000) / 60_000)
  const seconds = Math.floor((value % 60_000) / 1_000)
  const milliseconds = value % 1_000
  return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}.${milliseconds.toString().padStart(3, '0')}`
}

function parseTimestamp(value: string) {
  const normalized = value.trim().replace(',', '.')
  const parts = normalized.split(':')
  if (parts.length !== 3) return null
  const [hours, minutes, seconds] = parts.map(Number)
  if (![hours, minutes, seconds].every(Number.isFinite) || hours < 0 || minutes < 0 || seconds < 0) return null
  return Math.round((hours * 3_600 + minutes * 60 + seconds) * 1_000)
}

function updateSegmentTime(segmentId: string, field: 'startMs' | 'endMs', event: Event) {
  if (!project.value) return
  const value = parseTimestamp((event.target as HTMLInputElement).value)
  if (value === null) {
    editorError.value = '时间请使用 00:00:00.000 格式。'
    return
  }
  const index = project.value.segments.findIndex(segment => segment.id === segmentId)
  if (index < 0) return
  const segments = cloneSegments(project.value.segments)
  segments[index] = { ...segments[index], [field]: value }
  replaceSegments(segments)
}

function splitText(value: string) {
  const trimmed = value.trim()
  if (trimmed.length < 2) return [trimmed, ''] as const
  const middle = Math.floor(trimmed.length / 2)
  const punctuation = /[，。！？、,.!?;；\s]/g
  let best = middle
  let distance = Number.POSITIVE_INFINITY
  for (const match of trimmed.matchAll(punctuation)) {
    const index = match.index ?? middle
    if (Math.abs(index - middle) < distance) {
      best = index + 1
      distance = Math.abs(index - middle)
    }
  }
  return [trimmed.slice(0, best).trim(), trimmed.slice(best).trim()] as const
}

function splitSelectedSegment() {
  if (!project.value || !selectedSegment.value) return
  const selected = selectedSegment.value
  const duration = selected.endMs - selected.startMs
  const cut = Math.min(
    selected.endMs - 250,
    Math.max(selected.startMs + 250, previewTimeMs.value || selected.startMs + Math.round(duration / 2)),
  )
  const [firstSource, secondSource] = splitText(selected.sourceText)
  if (!firstSource || !secondSource || cut <= selected.startMs || cut >= selected.endMs) {
    editorError.value = '当前字幕无法在这里拆分，请先补充文本或调整播放位置。'
    return
  }
  const [firstTranslated, secondTranslated] = splitText(selected.translatedText ?? '')
  const index = project.value.segments.findIndex(segment => segment.id === selected.id)
  rememberSegments()
  const segments = cloneSegments(project.value.segments)
  const first: SubtitleSegment = {
    ...selected,
    endMs: cut,
    sourceText: firstSource,
    translatedText: firstTranslated || null,
  }
  const second: SubtitleSegment = {
    ...selected,
    id: `${selected.id}-split-${Date.now()}`,
    startMs: cut,
    sourceText: secondSource,
    translatedText: secondTranslated || null,
  }
  segments.splice(index, 1, first, second)
  selectedSegmentId.value = second.id
  replaceSegments(segments)
}

function mergeSelectedWithNext() {
  if (!project.value || !selectedSegment.value) return
  const index = project.value.segments.findIndex(segment => segment.id === selectedSegment.value?.id)
  const next = project.value.segments[index + 1]
  if (!next) {
    editorError.value = '已经是最后一条字幕，无法继续合并。'
    return
  }
  rememberSegments()
  const segments = cloneSegments(project.value.segments)
  const current = segments[index]
  segments.splice(index, 2, {
    ...current,
    endMs: next.endMs,
    sourceText: `${current.sourceText.trim()} ${next.sourceText.trim()}`.trim(),
    translatedText: `${current.translatedText?.trim() ?? ''} ${next.translatedText?.trim() ?? ''}`.trim() || null,
  })
  replaceSegments(segments)
}

function insertSegmentAfterSelected() {
  if (!project.value) return
  const selectedIndex = project.value.segments.findIndex(segment => segment.id === selectedSegmentId.value)
  const previous = selectedIndex >= 0 ? project.value.segments[selectedIndex] : null
  const next = selectedIndex >= 0 ? project.value.segments[selectedIndex + 1] : null
  const startMs = previous?.endMs ?? Math.max(0, previewTimeMs.value)
  const latestEnd = next?.startMs ?? project.value.durationMs
  const endMs = Math.min(latestEnd, startMs + 1_500)
  if (endMs - startMs < 250) {
    editorError.value = '相邻字幕之间没有足够的空白时间，请先调整时间轴。'
    return
  }
  const segment: SubtitleSegment = {
    id: `manual-${Date.now()}`,
    startMs,
    endMs,
    sourceText: '请输入字幕原文',
    translatedText: contentMode.value === 'source' ? null : '请输入字幕译文',
  }
  rememberSegments()
  const segments = cloneSegments(project.value.segments)
  const insertAt = selectedIndex >= 0 ? selectedIndex + 1 : segments.length
  segments.splice(insertAt, 0, segment)
  selectedSegmentId.value = segment.id
  replaceSegments(segments)
}

function deleteSelectedSegment() {
  if (!project.value || !selectedSegment.value) return
  rememberSegments()
  const segments = project.value.segments.filter(segment => segment.id !== selectedSegment.value?.id)
  selectedSegmentId.value = segments.at(-1)?.id ?? ''
  replaceSegments(segments)
}

function replaceTextAcrossSegments() {
  if (!project.value) return
  const search = segmentQuery.value
  if (!search) {
    editorError.value = '请先输入要查找的文本。'
    return
  }
  let changed = 0
  const replace = replacementText.value
  const segments = project.value.segments.map(segment => {
    const sourceText = segment.sourceText.split(search).join(replace)
    const translatedText = segment.translatedText === null || segment.translatedText === undefined
      ? segment.translatedText
      : segment.translatedText.split(search).join(replace)
    if (sourceText !== segment.sourceText || translatedText !== segment.translatedText) changed += 1
    return { ...segment, sourceText, translatedText }
  })
  if (!changed) {
    editorNotice.value = '没有匹配到可替换的字幕文本。'
    return
  }
  rememberSegments()
  replaceSegments(segments)
  editorNotice.value = `已替换 ${changed} 条字幕。`
}

function seekToSegment(segment: SubtitleSegment) {
  selectedSegmentId.value = segment.id
  previewTimeMs.value = segment.startMs
  if (!previewVideo.value) return
  previewVideo.value.currentTime = segment.startMs / 1_000
  void previewVideo.value.play().catch(() => undefined)
}

function handlePreviewTime(event: Event) {
  previewTimeMs.value = Math.round((event.target as HTMLVideoElement).currentTime * 1_000)
  const active = activePreviewSegment.value
  if (active) selectedSegmentId.value = active.id
}

function handleWorkspaceShortcut(event: KeyboardEvent) {
  if (!hasProjectSegments.value || !(event.ctrlKey || event.metaKey)) return
  const key = event.key.toLowerCase()
  if (key === 's') {
    event.preventDefault()
    void saveSegmentEdits()
    return
  }
  const target = event.target
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return
  if (key === 'z') {
    event.preventDefault()
    if (event.shiftKey) redoSegmentEdit()
    else undoSegmentEdit()
  }
}

function focusIssue(issue: QaIssue) {
  const segment = project.value?.segments.find(item => item.id === issue.segmentId)
  if (segment) seekToSegment(segment)
}

function defaultOutputPath(format: SubtitleExportFormat, video = false) {
  const directory = project.value?.workspace.outputDir || parentDirectory(sourcePath.value)
  const suffix = video ? '.subtitled' : `.${contentMode.value}`
  return joinOutputPath(directory || parentDirectory(sourcePath.value), `${sourceFileStem(sourcePath.value)}${suffix}.${video ? 'mp4' : format}`)
}

async function chooseOutputPath(format: SubtitleExportFormat, video = false) {
  const selected = await selectOutputFile(defaultOutputPath(format, video), [video ? 'mp4' : format])
  return typeof selected === 'string' ? selected : null
}

async function exportCurrentSubtitles() {
  if (!project.value) return
  editorError.value = ''
  editorNotice.value = ''
  if (qaErrors.value.length) {
    editorError.value = '请先处理质检中的错误，再导出字幕。'
    return
  }
  if (!await saveSegmentEdits(true)) return
  const destination = await chooseOutputPath(exportFormat.value)
  if (!destination) return
  const taskProject = project.value
  activity.beginSubtitle({
    projectId: taskProject.id,
    title: subtitleTaskTitle(taskProject),
    detail: `正在导出 ${exportFormat.value.toUpperCase()} 字幕`,
  })
  exporting.value = true
  try {
    subtitleOutput.value = await exportSubtitles({
      projectId: project.value.id,
      outputPath: destination,
      format: exportFormat.value,
      content: contentMode.value,
    })
    const refreshed = await getSubtitleProject(project.value.id)
    project.value = refreshed
    project.value.workspace.outputDir = parentDirectory(destination) || project.value.workspace.outputDir
    synchronizeProject(await saveSubtitleProjectWorkspace(project.value.id, workspacePayload()))
    editorNotice.value = '字幕文件已导出。'
    activity.finishSubtitle(taskProject.id, { detail: `${exportFormat.value.toUpperCase()} 字幕已导出` })
  } catch (value) {
    editorError.value = errorText(value)
    activity.finishSubtitle(taskProject.id, { state: taskFailureState(value), detail: '字幕导出未完成' })
  } finally {
    exporting.value = false
  }
}

async function burnCurrentVideo() {
  if (!project.value) return
  editorError.value = ''
  editorNotice.value = ''
  if (qaErrors.value.length) {
    editorError.value = '请先处理质检中的错误，再生成字幕视频。'
    return
  }
  if (!await saveSegmentEdits(true)) return
  const destination = await chooseOutputPath('srt', true)
  if (!destination) return
  const taskProject = project.value
  activity.beginSubtitle({
    projectId: taskProject.id,
    title: subtitleTaskTitle(taskProject),
    detail: '正在生成字幕视频',
  })
  exporting.value = true
  try {
    videoOutput.value = await burnSubtitles({
      projectId: project.value.id,
      outputPath: destination,
      content: contentMode.value,
      style: resolvedSubtitleStyle.value,
    })
    const refreshed = await getSubtitleProject(project.value.id)
    project.value = refreshed
    project.value.workspace.outputDir = parentDirectory(destination) || project.value.workspace.outputDir
    synchronizeProject(await saveSubtitleProjectWorkspace(project.value.id, workspacePayload()))
    editorNotice.value = '带字幕视频已生成。'
    activity.finishSubtitle(taskProject.id, { detail: '字幕视频已生成' })
  } catch (value) {
    editorError.value = errorText(value)
    activity.finishSubtitle(taskProject.id, { state: taskFailureState(value), detail: '字幕视频未生成' })
  } finally {
    exporting.value = false
  }
}

function projectNeedsTranslation() {
  if (!project.value) return false
  if (analysis.value) return analysis.value.strategy !== 'extract_chinese'
  return !['zh', 'zho', 'chi', 'cmn', 'zh-cn', 'zh-hans', 'chs'].includes(
    project.value.sourceLanguage?.toLowerCase() ?? '',
  )
}

async function translateProject(projectId: string, targetLanguage: string) {
  if (settings.value.processingMode === 'local_free') {
    return startBingTranslation({ projectId, targetLanguage })
  }
  if (settings.value.processingMode === 'local_custom') {
    return startOpenAiCompatibleTranslation({ projectId, targetLanguage })
  }
  return startBingTranslation({ projectId, targetLanguage })
}

async function continueProject() {
  if (!project.value) return
  if (!project.value.segments.length) {
    await generateChineseSubtitles()
    return
  }
  const needsTranslation = projectNeedsTranslation()
  const hasTranslations = project.value.segments.every(segment => segment.translatedText?.trim())
  if (!needsTranslation || hasTranslations) {
    notice.value = '字幕已具备继续处理的条件。'
    return
  }
  const taskProject = project.value
  activity.beginSubtitle({
    projectId: taskProject.id,
    title: subtitleTaskTitle(taskProject),
    detail: '正在继续处理字幕',
  })
  processing.value = true
  error.value = ''
  try {
    let current = project.value
    if (!hasTranslations) {
      current = await translateProject(current.id, current.targetLanguage ?? 'zh-CN')
    }
    synchronizeProject(current)
    notice.value = '字幕已继续处理完成。'
    activity.finishSubtitle(taskProject.id, { detail: '字幕已继续处理完成' })
  } catch (value) {
    error.value = errorText(value)
    activity.finishSubtitle(taskProject.id, { state: taskFailureState(value), detail: '继续处理未完成' })
  } finally {
    processing.value = false
  }
}

function applyPreset(preset: Exclude<SubtitlePreset, 'custom'>) {
  const values: Record<Exclude<SubtitlePreset, 'custom'>, Partial<SubtitleStyle>> = {
    classic: { primaryColor: '#F8F8F8', outlineColor: '#151515', outlineWidth: 3, shadow: 1, boxed: false, backgroundOpacity: 0, bold: false },
    contrast: { primaryColor: '#F8F8F8', outlineColor: '#151515', outlineWidth: 5, shadow: 0, boxed: true, backgroundColor: '#151515', backgroundOpacity: 68, bold: true },
    yellow: { primaryColor: '#FFD95A', outlineColor: '#151515', outlineWidth: 3, shadow: 1, boxed: false, backgroundOpacity: 0, bold: true },
  }
  Object.assign(subtitleStyle.value, values[preset])
  stylePreset.value = preset
}

function markCustom() {
  stylePreset.value = 'custom'
}

function restoreSubtitleStyle() {
  try {
    const saved = localStorage.getItem(subtitleStyleStorageKey)
    if (!saved) return
    const value = JSON.parse(saved) as {
      preset?: SubtitlePreset
      contentMode?: SubtitleContentMode
      syncTranslatedStyle?: boolean
      style?: Partial<SubtitleStyle>
    }
    if (value.style) Object.assign(subtitleStyle.value, value.style)
    if (value.preset && ['classic', 'contrast', 'yellow', 'custom'].includes(value.preset)) {
      stylePreset.value = value.preset
    } else {
      applyPreset('classic')
    }
    if (value.contentMode) contentMode.value = value.contentMode
    if (typeof value.syncTranslatedStyle === 'boolean') syncTranslatedStyle.value = value.syncTranslatedStyle
  } catch {
    localStorage.removeItem(subtitleStyleStorageKey)
  }
}

function restoreArtifacts(value: SubtitleProject | null) {
  if (!value) return
  const subtitles = value.artifacts?.filter(item => item.kind === 'subtitle') ?? []
  const videos = value.artifacts?.filter(item => item.kind === 'video') ?? []
  subtitleOutput.value = subtitles.at(-1)?.path ?? ''
  videoOutput.value = videos.at(-1)?.path ?? ''
}

function iconStyle(url: string) {
  return { '--icon-url': `url("${url}")` }
}

async function openAdvancedWorkspace() {
  showAdvanced.value = true
  await nextTick()
  workbenchPanel.value?.scrollIntoView({ behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth', block: 'start' })
}

async function refreshLocalAssets() {
  const [models, runtimes] = await Promise.all([listWhisperModels(), listWhisperRuntimes()])
  localModels.value = models
  localRuntimes.value = runtimes
  localModelReady.value = Boolean(models.find(item => item.id === settings.value.whisperModel)?.installed)
  localRuntimeReady.value = Boolean(runtimes.find(item => item.id === settings.value.whisperRuntime)?.installed)
}

async function prepareRequiredAsset(action: Extract<ReadinessAction, 'install-model' | 'install-runtime'>) {
  error.value = ''
  notice.value = ''
  if (action === 'install-model') {
    const model = selectedWhisperModel.value
    if (!model) {
      error.value = '未找到当前选择的 Whisper 模型，请到设置中重新选择。'
      return
    }
    preparingAsset.value = 'model'
    try {
      await downloadWhisperModel(model.id)
      await refreshLocalAssets()
      notice.value = `${model.name} 已准备好，可以开始自动字幕。`
    } catch (value) {
      error.value = errorText(value)
    } finally {
      preparingAsset.value = null
    }
    return
  }

  const runtime = selectedWhisperRuntime.value
  if (!runtime) {
    error.value = '未找到当前选择的 Whisper 运行组件，请到设置中重新选择。'
    return
  }
  preparingAsset.value = 'runtime'
  try {
    await downloadWhisperRuntime(runtime.id)
    await refreshLocalAssets()
    notice.value = `${runtime.name} 已准备好，可以开始自动字幕。`
  } catch (value) {
    error.value = errorText(value)
  } finally {
    preparingAsset.value = null
  }
}

function handleReadinessAction() {
  const card = readinessCard.value
  if (!card) return
  if (card.action === 'open-settings') {
    void router.push({ name: 'settings', query: card.setup ? { setup: card.setup } : undefined })
    return
  }
  void prepareRequiredAsset(card.action)
}

async function refreshProcessingSettings() {
  settings.value = await getGeminiSettings()
  if (!['local_free', 'local_custom'].includes(settings.value.processingMode)) {
    settings.value.processingMode = 'local_free'
  }
  await refreshLocalAssets()
}
const durationLabel = computed(() => {
  const seconds = Math.floor((analysis.value?.durationMs ?? 0) / 1000)
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  return hours ? `${hours} 小时 ${minutes} 分钟` : `${minutes || 1} 分钟`
})

function errorText(value: unknown) {
  if (typeof value === 'object' && value && 'message' in value) {
    const payload = value as { message: unknown; detail?: unknown }
    return payload.detail ? `${String(payload.message)}\n${String(payload.detail)}` : String(payload.message)
  }
  return String(value)
}

function taskFailureState(value: unknown): 'cancelled' | 'failed' {
  const message = errorText(value).toLowerCase()
  return message.includes('取消') || message.includes('cancelled') ? 'cancelled' : 'failed'
}

function subtitleTaskTitle(value: SubtitleProject) {
  return sourceFileStem(value.sourcePath)
}

function outputPath(extension: string) {
  const directory = project.value?.workspace.outputDir || parentDirectory(sourcePath.value)
  return joinOutputPath(directory, `${sourceFileStem(sourcePath.value)}${extension}`)
}

function languageName(code?: string | null) {
  const value = code?.toLowerCase() ?? ''
  const names: Record<string, string> = {
    zh: '中文', zho: '中文', chi: '中文', cmn: '中文', 'zh-cn': '中文',
    en: '英文', eng: '英文', ja: '日文', jpn: '日文', ko: '韩文', kor: '韩文',
    fr: '法文', fra: '法文', fre: '法文', de: '德文', deu: '德文', ger: '德文',
    es: '西班牙文', spa: '西班牙文', ru: '俄文', rus: '俄文',
  }
  return names[value] ?? (value && value !== 'und' ? value.toUpperCase() : '自动识别')
}

async function chooseProject(projectId: string) {
  showAdvanced.value = false
  analyzing.value = true
  error.value = ''
  notice.value = ''
  subtitleOutput.value = ''
  videoOutput.value = ''
  try {
    const existing = await getSubtitleProject(projectId)
    sourcePath.value = existing.sourcePath
    synchronizeProject(existing, true)
    try {
      analysis.value = await analyzeSubtitleSource(existing.sourcePath)
    } catch (value) {
      analysis.value = null
      notice.value = `项目已打开，但无法重新分析源视频：${errorText(value)}`
    }
  } catch (value) {
    error.value = errorText(value)
  } finally {
    analyzing.value = false
    emit('consumedProject')
  }
}

async function chooseVideo(path?: string) {
  const selected = path ?? await selectMediaFile()
  if (!selected) return
  showAdvanced.value = false
  sourcePath.value = selected
  analysis.value = null
  project.value = null
  selectedSegmentId.value = ''
  lastSavedSegments.value = ''
  subtitleOutput.value = ''
  videoOutput.value = ''
  error.value = ''
  notice.value = ''
  analyzing.value = true
  try {
    analysis.value = await analyzeSubtitleSource(selected)
    const previous = (await listSubtitleProjects()).find(item => item.sourcePath === selected)
    if (previous) {
      synchronizeProject(previous, true)
    }
  } catch (value) {
    error.value = errorText(value)
  } finally {
    analyzing.value = false
    emit('consumedInitial')
  }
}

async function ensureProject() {
  if (project.value && project.value.sourcePath === sourcePath.value) return project.value
  const created = await createSubtitleProject({
    sourcePath: sourcePath.value,
    durationMs: analysis.value?.durationMs || undefined,
    model: settings.value.whisperModel,
    targetLanguage: 'zh-CN',
  })
  synchronizeProject(created)
  await persistWorkspace(true)
  return project.value ?? created
}

async function generateChineseSubtitles() {
  if (!analysis.value) {
    await chooseVideo()
    return
  }
  processing.value = true
  error.value = ''
  notice.value = ''
  let activityProjectId = ''
  try {
    const automaticContent = analysis.value.strategy === 'extract_chinese'
      ? 'source'
      : showAdvanced.value
        ? contentMode.value
        : 'bilingual'
    const automaticBurn = showAdvanced.value ? burnVideo.value : true
    contentMode.value = automaticContent
    burnVideo.value = automaticBurn
    let current = await ensureProject()
    activityProjectId = current.id
    activity.beginSubtitle({
      projectId: current.id,
      title: subtitleTaskTitle(current),
      detail: '正在生成自动字幕',
    })
    const track = analysis.value.recommendedTrack
    if (track) {
      current = await importSubtitleTrack({
        projectId: current.id,
        streamIndex: track.streamIndex,
        language: track.language,
      })
      if (analysis.value.strategy === 'translate_subtitle') {
        current = await translateProject(current.id, 'zh-CN')
      }
    } else {
      current = await startWhisperTranscription({
        projectId: current.id,
        model: settings.value.whisperModel,
        runtime: settings.value.whisperRuntime,
      })
      current = await translateProject(current.id, 'zh-CN')
    }
    project.value = current
    const content = analysis.value.strategy === 'extract_chinese' ? 'source' : contentMode.value
    const srtPath = outputPath('.zh-CN.srt')
    subtitleOutput.value = await exportSubtitles({
      projectId: current.id,
      outputPath: srtPath,
      format: 'srt',
      content,
    })
    if (automaticBurn) {
      const mp4Path = outputPath('.subtitled.mp4')
      videoOutput.value = await burnSubtitles({
        projectId: current.id,
        outputPath: mp4Path,
        content,
        style: resolvedSubtitleStyle.value,
      })
    }
    synchronizeProject((await listSubtitleProjects()).find(item => item.id === current.id) ?? current)
    await persistWorkspace(true)
    notice.value = automaticBurn
      ? '中文字幕和带字幕视频都已生成。'
      : '中文字幕文件已生成。'
    activity.finishSubtitle(current.id, {
      detail: automaticBurn ? '中文字幕和字幕视频已生成' : '中文字幕已生成',
    })
    await nextTick()
    resultPanel.value?.scrollIntoView({ behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth', block: 'nearest' })
  } catch (value) {
    error.value = errorText(value)
    if (activityProjectId) {
      activity.finishSubtitle(activityProjectId, {
        state: taskFailureState(value),
        detail: '自动字幕未完成',
      })
    }
  } finally {
    processing.value = false
  }
}

async function cancel() {
  if (!project.value) return
  activity.updateSubtitle(project.value.id, { detail: '正在取消字幕任务…' })
  await cancelSubtitleTask(project.value.id)
}

watch(() => props.initialSourcePath, path => {
  if (path) void chooseVideo(path)
})
watch(() => props.initialProjectId, projectId => {
  if (projectId) void chooseProject(projectId)
})
watch(analysis, value => {
  if (value?.strategy === 'extract_chinese') contentMode.value = 'source'
  else if (value) contentMode.value = 'bilingual'
})
watch(
  [subtitleStyle, stylePreset, contentMode, syncTranslatedStyle],
  () => {
    localStorage.setItem(subtitleStyleStorageKey, JSON.stringify({
      preset: stylePreset.value,
      contentMode: contentMode.value,
      syncTranslatedStyle: syncTranslatedStyle.value,
      style: subtitleStyle.value,
    }))
    scheduleWorkspaceSave()
  },
  { deep: true },
)
watch([burnVideo, exportFormat, reviewStatus], scheduleWorkspaceSave)

onMounted(async () => {
  try {
    restoreSubtitleStyle()
    await refreshProcessingSettings()
    unlisten = await onSubtitleProgress(event => {
      if (!project.value || event.projectId === project.value.id) progress.value = event
    })
    if (props.initialProjectId) await chooseProject(props.initialProjectId)
    else if (props.initialSourcePath) await chooseVideo(props.initialSourcePath)
  } catch (value) {
    error.value = errorText(value)
  }
})
onActivated(() => {
  window.addEventListener('keydown', handleWorkspaceShortcut)
  void refreshProcessingSettings().catch(value => {
    error.value = errorText(value)
  })
})
onDeactivated(() => window.removeEventListener('keydown', handleWorkspaceShortcut))
onBeforeUnmount(() => {
  if (segmentSaveTimer) window.clearTimeout(segmentSaveTimer)
  if (workspaceSaveTimer) window.clearTimeout(workspaceSaveTimer)
  window.removeEventListener('keydown', handleWorkspaceShortcut)
  unlisten?.()
})
</script>

<template>
  <section class="simple-subtitle">
    <header class="simple-header">
      <div>
        <h1>自动字幕</h1>
        <p>选择视频后，自动识别、翻译、校对并生成结果。</p>
      </div>
      <button class="settings-button" type="button" @click="router.push({ name: 'settings' })">
        <i class="icon" :style="iconStyle(settingsIcon)" />
        设置
      </button>
    </header>

    <div class="subtitle-layout">
      <main class="source-column">
        <div class="section-caption">
          <div>
            <strong>视频来源</strong>
          </div>
        </div>

        <button class="file-card" type="button" :disabled="analyzing || processing" @click="chooseVideo()">
          <span class="file-icon"><i class="icon" :style="iconStyle(mediaVideoIcon)" /></span>
          <span class="file-copy">
            <strong>{{ sourcePath ? sourcePath.replace(/^.*[\\/]/, '') : '选择一个视频' }}</strong>
            <small>{{ sourcePath || '支持 MP4、MKV、MOV、WebM 等常见格式' }}</small>
          </span>
          <span class="replace-label">{{ sourcePath ? '更换视频' : '浏览文件' }}</span>
        </button>

        <div v-if="analyzing" class="analysis-card loading">
          <i />
          <div><strong>正在检查视频</strong><span>分析音轨、字幕轨道和语言信息…</span></div>
        </div>

        <div v-else-if="analysis" class="analysis-card">
          <span class="result-mark" :class="analysis.strategy"><i class="icon" :style="iconStyle(checkIcon)" /></span>
          <div class="analysis-copy">
            <strong>{{ analysis.recommendedTrack ? '发现可用字幕' : '未发现字幕' }}</strong>
            <span>
              {{ durationLabel }}
              <template v-if="analysis.recommendedTrack">
                · {{ languageName(analysis.recommendedTrack.language) }}
                · {{ analysis.recommendedTrack.codec.toUpperCase() }}
              </template>
              <template v-else> · 自动识别语言</template>
            </span>
          </div>
          <span class="route-tag">
            {{ analysis.strategy === 'extract_chinese' ? '直接提取' : analysis.strategy === 'translate_subtitle' ? '只翻译文本' : '识别音频' }}
          </span>
        </div>
      </main>

      <aside v-if="analysis || analyzing" class="action-column">
        <div class="section-caption">
          <div>
              <strong>自动处理</strong>
          </div>
        </div>

        <section class="action-panel">
          <div v-if="analysis" class="route-summary">
            <span>处理流程</span>
            <strong>{{ processingPlan }}</strong>
            <small>
              {{ automationFlowLabel }} · {{ processingTimeEstimate }}
            </small>
          </div>
          <div v-else class="route-summary pending">
            <span>自动流程</span>
            <strong>等待选择视频</strong>
            <small>选择后自动识别、翻译、校对并生成结果</small>
          </div>

          <label v-if="analysis && showAdvanced" class="burn-option">
            <input v-model="burnVideo" type="checkbox" />
            <span><strong>生成字幕视频</strong></span>
          </label>

          <fieldset v-if="analysis && showAdvanced && burnVideo" class="subtitle-style-editor" :disabled="processing">
            <div class="style-field compact-field">
              <span>字幕内容</span>
              <AppSelect
                v-model="contentMode"
                :options="availableContentOptions"
                aria-label="字幕内容"
              />
            </div>

            <div class="style-heading">
              <span>字幕样式</span>
              <small v-if="stylePreset === 'custom'">已自定义</small>
            </div>
            <div class="preset-grid" role="radiogroup" aria-label="字幕样式预设">
              <button
                v-for="preset in subtitlePresets"
                :key="preset.id"
                type="button"
                class="preset-option"
                :class="{ active: stylePreset === preset.id }"
                role="radio"
                :aria-checked="stylePreset === preset.id"
                @click="applyPreset(preset.id)"
              >
                <i :style="{ color: preset.color, backgroundColor: preset.boxed ? '#34312f' : 'transparent' }">字</i>
                <span><strong>{{ preset.name }}</strong><small>{{ preset.note }}</small></span>
              </button>
            </div>

            <div class="subtitle-preview" :class="previewPositionClass">
              <div class="preview-caption" :style="previewBoxStyle">
                <template v-if="contentMode === 'bilingual' && subtitleStyle.translatedFirst">
                  <span :style="translatedPreviewStyle">你好，欢迎使用 YDLite</span>
                  <span :style="sourcePreviewStyle">Hello, welcome to YDLite</span>
                </template>
                <template v-else>
                  <span v-if="contentMode !== 'translated'" :style="sourcePreviewStyle">Hello, welcome to YDLite</span>
                  <span v-if="contentMode !== 'source'" :style="translatedPreviewStyle">你好，欢迎使用 YDLite</span>
                </template>
              </div>
            </div>

            <div v-if="contentMode === 'bilingual'" class="translation-link">
              <label>
                <input v-model="syncTranslatedStyle" type="checkbox" />
                <span><strong>译文跟随源文</strong><small>字体、颜色和字号保持一致</small></span>
              </label>
            </div>

            <button class="advanced-toggle" type="button" :aria-expanded="advancedStyleOpen" @click="advancedStyleOpen = !advancedStyleOpen">
              <span>{{ advancedStyleOpen ? '收起高级设置' : '高级设置' }}</span>
              <i :class="{ open: advancedStyleOpen }">⌄</i>
            </button>

            <div class="advanced-wrap" :class="{ open: advancedStyleOpen }" :inert="!advancedStyleOpen" :aria-hidden="!advancedStyleOpen">
              <div class="advanced-inner">
                <div class="advanced-grid primary-style-row">
                  <div class="style-field font-field">
                    <span>字体</span>
                    <AppSelect
                      v-model="subtitleStyle.fontFamily"
                      :options="fontOptions"
                      aria-label="字幕字体"
                      @update:model-value="markCustom"
                    />
                  </div>
                  <div class="style-field">
                    <span>位置</span>
                    <AppSelect
                      v-model="subtitleStyle.position"
                      :options="positionOptions"
                      aria-label="字幕位置"
                      @update:model-value="markCustom"
                    />
                  </div>
                </div>

                <section class="advanced-section">
                  <span class="advanced-section-title">尺寸与位置</span>
                  <div class="advanced-grid slider-grid">
                    <label class="style-field range-field">
                      <span>源文字号 <b>{{ subtitleStyle.fontSize }}</b></span>
                      <input v-model.number="subtitleStyle.fontSize" type="range" min="24" max="80" step="1" @input="markCustom" />
                    </label>
                    <label class="style-field range-field">
                      <span>底部间距 <b>{{ subtitleStyle.marginVertical }}</b></span>
                      <input v-model.number="subtitleStyle.marginVertical" type="range" min="20" max="180" step="4" @input="markCustom" />
                    </label>
                  </div>
                </section>

                <section class="advanced-section">
                  <span class="advanced-section-title">颜色与描边</span>
                  <div class="color-grid">
                    <label class="color-control">
                      <span>文字</span>
                      <input v-model="subtitleStyle.primaryColor" type="color" aria-label="文字颜色" @input="markCustom" />
                    </label>
                    <label class="color-control">
                      <span>描边</span>
                      <input v-model="subtitleStyle.outlineColor" type="color" aria-label="描边颜色" @input="markCustom" />
                    </label>
                  </div>
                  <div class="outline-row">
                    <label class="style-field range-field">
                      <span>描边粗细 <b>{{ subtitleStyle.outlineWidth }}</b></span>
                      <input v-model.number="subtitleStyle.outlineWidth" type="range" min="0" max="8" step="0.5" @input="markCustom" />
                    </label>
                    <label class="style-check compact-check">
                      <input v-model="subtitleStyle.bold" type="checkbox" @change="markCustom" />
                      <span>粗体</span>
                    </label>
                  </div>
                </section>

                <section v-if="contentMode === 'bilingual'" class="advanced-section bilingual-section">
                  <span class="advanced-section-title">双语排列</span>
                  <div class="bilingual-controls">
                    <template v-if="!syncTranslatedStyle">
                      <label class="style-field range-field">
                        <span>译文字号 <b>{{ subtitleStyle.translatedFontSize }}</b></span>
                        <input v-model.number="subtitleStyle.translatedFontSize" type="range" min="24" max="80" step="1" @input="markCustom" />
                      </label>
                      <label class="color-control translated-color">
                        <span>译文</span>
                        <input v-model="subtitleStyle.translatedColor" type="color" aria-label="译文颜色" @input="markCustom" />
                      </label>
                    </template>
                    <label class="style-check order-check">
                      <input v-model="subtitleStyle.translatedFirst" type="checkbox" @change="markCustom" />
                      <span>翻译字幕显示在上方</span>
                    </label>
                  </div>
                </section>
              </div>
            </div>
          </fieldset>

          <section v-if="!hasProjectSegments && readinessCard" class="readiness-card" aria-live="polite">
            <span class="readiness-mark">!</span>
            <div class="readiness-copy">
              <span>开始前准备</span>
              <strong>{{ readinessCard.title }}</strong>
              <p>{{ readinessCard.description }}</p>
            </div>
            <button
              type="button"
              :disabled="preparingAsset !== null"
              @click="handleReadinessAction"
            >
              {{ readinessCard.actionLabel }}
            </button>
          </section>

          <button
            v-if="project && (project.status === 'paused' || project.status === 'failed')"
            class="generate-button"
            type="button"
            :disabled="processing || analyzing"
            @click="continueProject"
          >
            <span>{{ processing ? '正在继续处理…' : '继续上次任务' }}</span>
          </button>
          <button v-else-if="!hasProjectSegments || (project && !subtitleOutput && !videoOutput)" class="generate-button" type="button" :disabled="analysis ? !canGenerate : analyzing" @click="generateChineseSubtitles">
            <span>{{ actionLabel }}</span>
          </button>
          <p v-else-if="!hasProjectSegments && generateHint" class="action-hint">{{ generateHint }}</p>

          <div v-if="processing" class="task-progress">
            <div><strong>{{ stageLabel || '正在处理…' }}</strong><span>{{ Math.round(progress?.percent ?? 0) }}%</span></div>
            <div class="progress-track"><i :style="{ width: `${progress?.percent ?? 0}%` }" /></div>
            <button type="button" @click="cancel">取消任务</button>
          </div>

          <p v-if="error" class="message error">{{ error }}</p>
          <p v-if="notice" class="message success">{{ notice }}</p>

          <div v-if="(subtitleOutput || videoOutput) && !processing" ref="resultPanel" class="output-panel">
            <div class="output-heading"><span class="result-mark done"><i class="icon" :style="iconStyle(checkIcon)" /></span><strong>{{ outputHeading }}</strong></div>
            <div class="output-actions">
              <button v-if="videoOutput" class="dark" type="button" @click="openPath(videoOutput)"><i class="icon" :style="iconStyle(mediaVideoIcon)" />播放字幕视频</button>
              <button v-if="subtitleOutput" type="button" @click="openPath(subtitleOutput)"><i class="icon" :style="iconStyle(pageIcon)" />打开字幕</button>
              <button type="button" @click="openParentFolder(subtitleOutput || videoOutput)"><i class="icon" :style="iconStyle(folderIcon)" />打开文件夹</button>
            </div>
          </div>
        </section>
      </aside>
    </div>

  </section>
</template>

<style scoped>
.simple-subtitle {
  height: 100%;
  overflow: auto;
  background: transparent;
  color: var(--workspace-ink);
}

.simple-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 20px;
  max-width: 1040px;
  margin: 0 auto 26px;
}

.simple-header h1 {
  margin: 0;
  font-family: inherit;
  font-size: 24px;
  font-weight: 700;
  line-height: 1.25;
  letter-spacing: -0.02em;
}

.simple-header p {
  max-width: 650px;
  margin: 0;
  color: var(--workspace-muted);
  font-size: 13px;
  line-height: 1.65;
}

.settings-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  gap: 7px;
  min-height: 40px;
  padding: 0 14px;
  border: 1px solid var(--workspace-border);
  border-radius: 9px;
  background: var(--workspace-surface);
  color: var(--workspace-ink);
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: border-color 140ms ease-out, background-color 140ms ease-out, transform 140ms var(--workspace-ease);
}

.settings-button:hover {
  border-color: var(--workspace-border-strong);
  background: var(--workspace-surface-muted);
  transform: translateY(-1px);
}

.subtitle-layout {
  display: grid;
  grid-template-columns: 1fr;
  gap: 24px;
  max-width: 1040px;
  margin: 0 auto;
  align-items: start;
}

.source-column,
.action-column {
  min-width: 0;
}

.section-caption {
  display: flex;
  align-items: center;
  gap: 0;
  min-height: auto;
  margin-bottom: 10px;
}

.section-caption strong,
.section-caption small {
  display: block;
}

.section-caption strong {
  color: var(--workspace-ink);
  font-size: 13px;
}

.section-caption small {
  margin-top: 3px;
  color: var(--workspace-subtle);
  font-size: 10px;
}

.file-card {
  display: grid;
  grid-template-columns: 48px minmax(0, 1fr) auto;
  align-items: center;
  gap: 15px;
  width: 100%;
  min-height: 80px;
  padding: 14px;
  border: 1px dashed var(--workspace-border-strong);
  border-radius: 10px;
  background: var(--workspace-surface);
  color: var(--workspace-ink);
  text-align: left;
  cursor: pointer;
  transition: border-color 140ms ease-out, background-color 140ms ease-out;
}

.file-card:hover:not(:disabled) {
  border-color: var(--workspace-accent);
  background: var(--workspace-accent-soft);
}

.file-icon {
  display: grid;
  place-items: center;
  width: 46px;
  height: 46px;
  border-radius: 13px;
  background: var(--workspace-ink);
  color: white;
  font-size: 11px;
}

.file-copy {
  min-width: 0;
}

.file-copy strong,
.file-copy small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-copy strong {
  font-size: 14px;
}

.file-copy small {
  margin-top: 6px;
  color: var(--workspace-subtle);
  font-size: 11px;
}

.replace-label {
  padding: 7px 10px;
  border-radius: 8px;
  background: var(--workspace-accent-soft);
  color: var(--workspace-accent);
  font-size: 11px;
  font-weight: 700;
}

.analysis-card {
  display: grid;
  grid-template-columns: 38px minmax(0, 1fr) auto;
  align-items: center;
  gap: 14px;
  margin-top: 14px;
  padding: 16px 17px;
  border: 1px solid var(--workspace-border);
  border-radius: 12px;
  background: var(--workspace-surface);
  animation: panel-in 260ms var(--workspace-ease);
}

.result-mark {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border-radius: 10px;
  background: var(--workspace-accent-soft);
  color: var(--workspace-accent);
  font-weight: 700;
}

.result-mark .icon {
  width: 16px;
  height: 16px;
}

.analysis-copy {
  min-width: 0;
}

.analysis-copy strong,
.analysis-copy span {
  display: block;
}

.analysis-copy strong {
  font-size: 13px;
}

.analysis-copy span {
  margin-top: 5px;
  color: var(--workspace-subtle);
  font-size: 11px;
}

.route-tag {
  padding: 6px 9px;
  border-radius: 7px;
  background: var(--workspace-surface-muted);
  color: var(--workspace-muted);
  font-size: 10px;
  font-weight: 700;
}

.loading i {
  width: 28px;
  height: 28px;
  border: 2px solid var(--workspace-border);
  border-top-color: var(--workspace-accent);
  border-radius: 50%;
  animation: spin 800ms linear infinite;
}

.loading strong,
.loading span {
  display: block;
}

.loading span {
  margin-top: 4px;
  color: var(--workspace-subtle);
  font-size: 11px;
}

.action-panel {
  padding: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  box-shadow: none;
}

.route-summary {
  padding-bottom: 16px;
  border-bottom: 1px solid var(--workspace-border);
}

.route-summary > span,
.route-summary strong,
.route-summary small {
  display: block;
}

.route-summary > span {
  color: var(--workspace-subtle);
  font-size: 10px;
  font-weight: 700;
}

.route-summary strong {
  margin-top: 6px;
  color: var(--workspace-ink);
  font-size: 14px;
}

.route-summary small {
  margin-top: 5px;
  color: var(--workspace-muted);
  font-size: 11px;
  line-height: 1.5;
}

.route-summary.pending strong {
  color: var(--workspace-muted);
}

.burn-option {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin-top: 16px;
  cursor: pointer;
}

.burn-option input {
  width: 16px;
  height: 16px;
  margin-top: 2px;
  accent-color: var(--workspace-accent);
}

.burn-option span,
.burn-option strong,
.burn-option small {
  display: block;
}

.subtitle-style-editor {
  min-width: 0;
  margin-top: 16px;
  padding-right: 0;
  padding-left: 0;
  padding-top: 16px;
  border-top: 1px solid var(--workspace-border);
  border-right: 0;
  border-bottom: 0;
  border-left: 0;
}

.subtitle-style-editor:disabled {
  opacity: 0.62;
}

.style-heading,
.style-field > span {
  color: var(--workspace-subtle);
  font-size: 10px;
  font-weight: 700;
}

.compact-field {
  display: grid;
  grid-template-columns: 78px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
}

.style-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 16px;
}

.style-heading small {
  color: var(--workspace-accent);
  font-size: 9px;
}

.preset-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 6px;
  margin-top: 8px;
}

.preset-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 5px;
  min-width: 0;
  min-height: 70px;
  padding: 8px 5px 7px;
  border: 1px solid var(--workspace-border);
  border-radius: 8px;
  background: transparent;
  color: var(--workspace-ink);
  text-align: center;
  cursor: pointer;
  transition: border-color 140ms ease-out, background-color 140ms ease-out;
}

.preset-option:hover,
.preset-option.active {
  border-color: var(--workspace-accent);
  background: var(--workspace-accent-soft);
}

.preset-option.active {
  background: transparent;
}

.preset-option i {
  display: grid;
  place-items: center;
  flex: 0 0 27px;
  width: 27px;
  height: 27px;
  border-radius: 4px;
  font-size: 13px;
  font-style: normal;
  font-weight: 800;
  text-shadow: 1px 1px 0 #171615, -1px -1px 0 #171615;
}

.preset-option span,
.preset-option strong,
.preset-option small {
  display: block;
  min-width: 0;
}

.preset-option strong {
  overflow: hidden;
  font-size: 10px;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preset-option small {
  margin-top: 3px;
  color: var(--workspace-subtle);
  font-size: 8px;
  line-height: 1.2;
}

.subtitle-preview {
  position: relative;
  display: flex;
  justify-content: center;
  min-height: 126px;
  margin-top: 10px;
  overflow: hidden;
  border-radius: 9px;
  background:
    linear-gradient(180deg, transparent 0 62%, rgba(20, 18, 16, 0.34)),
    radial-gradient(circle at 72% 22%, rgba(222, 194, 150, 0.34), transparent 22%),
    linear-gradient(135deg, #777b76, #313735 58%, #202423);
}

.subtitle-preview::before {
  position: absolute;
  inset: 16px 18px auto auto;
  width: 54px;
  height: 22px;
  border: 1px solid rgba(247, 241, 232, 0.22);
  border-radius: 99px;
  content: '';
}

.preview-caption {
  position: absolute;
  display: grid;
  justify-items: center;
  max-width: calc(100% - 28px);
  padding: 3px 7px;
  border-radius: 3px;
  line-height: 1.28;
  text-align: center;
}

.position-bottom .preview-caption { bottom: 12px; }
.position-middle .preview-caption { top: 50%; transform: translateY(-50%); }
.position-top .preview-caption { top: 12px; }

.preview-caption span {
  display: block;
  white-space: nowrap;
}

.translation-link {
  margin-top: 11px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--workspace-border);
}

.translation-link > label:first-child {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  cursor: pointer;
}

.translation-link input[type='checkbox'],
.style-check input {
  width: 15px;
  height: 15px;
  margin: 1px 0 0;
  accent-color: var(--workspace-accent);
}

.translation-link strong,
.translation-link small {
  display: block;
}

.translation-link strong {
  font-size: 11px;
}

.translation-link small {
  margin-top: 2px;
  color: var(--workspace-subtle);
  font-size: 9px;
  line-height: 1.45;
}

.advanced-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  min-height: 38px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--workspace-muted);
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}

.advanced-toggle i {
  font-size: 16px;
  font-style: normal;
  transition: transform 160ms var(--workspace-ease);
}

.advanced-toggle i.open {
  transform: rotate(180deg);
}

.advanced-wrap {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 180ms var(--workspace-ease);
}

.advanced-wrap.open {
  grid-template-rows: 1fr;
}

.advanced-wrap.open .advanced-inner {
  overflow: visible;
}

.advanced-inner {
  min-height: 0;
  overflow: hidden;
}

.advanced-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.primary-style-row {
  grid-template-columns: minmax(0, 1.25fr) minmax(104px, 0.75fr);
  padding: 7px 0 5px;
}

.advanced-section {
  margin-top: 13px;
  padding-top: 12px;
  border-top: 1px solid var(--workspace-border);
}

.advanced-section-title {
  display: block;
  margin-bottom: 10px;
  color: var(--workspace-subtle);
  font-size: 9px;
  font-weight: 750;
  letter-spacing: 0.02em;
}

.style-field {
  display: grid;
  gap: 6px;
  min-width: 0;
}

.style-field b {
  color: var(--workspace-ink);
  font-size: 9px;
}

.range-field > span {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.slider-grid {
  padding: 0;
}

.range-field input[type='range'] {
  width: 100%;
  min-height: 20px;
  margin: 0;
  accent-color: var(--workspace-accent);
}

.color-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.color-control {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  min-height: 42px;
  padding: 5px 6px 5px 10px;
  border: 1px solid var(--workspace-border);
  border-radius: 8px;
  color: var(--workspace-muted);
  font-size: 10px;
  font-weight: 650;
  cursor: pointer;
}

.color-control input {
  flex: 0 0 31px;
  width: 31px;
  height: 31px;
  padding: 2px;
  border: 1px solid var(--workspace-border);
  border-radius: 6px;
  background: var(--workspace-surface);
  cursor: pointer;
}

.outline-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: end;
  gap: 12px;
  margin-top: 11px;
}

.style-check {
  display: flex;
  align-items: center;
  gap: 7px;
  min-height: 32px;
  color: var(--workspace-muted);
  font-size: 10px;
  cursor: pointer;
}

.compact-check {
  min-height: 38px;
  padding: 0 2px 0 12px;
  border-left: 1px solid var(--workspace-border);
}

.bilingual-section {
  padding-bottom: 2px;
}

.bilingual-controls {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: end;
  gap: 11px;
}

.order-check {
  grid-column: 1 / -1;
  min-height: 28px;
}

.translated-color {
  min-width: 92px;
}

.burn-option strong {
  color: var(--workspace-ink);
  font-size: 12px;
}

.burn-option small {
  margin-top: 4px;
  color: var(--workspace-subtle);
  font-size: 10px;
}

.readiness-card {
  display: grid;
  grid-template-columns: 28px minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  margin: 14px 0;
  padding: 11px 12px;
  border: 1px solid color-mix(in oklch, var(--workspace-accent) 26%, var(--workspace-border));
  border-radius: 10px;
  background: color-mix(in oklch, var(--workspace-accent-soft) 62%, var(--workspace-surface));
}

.readiness-mark {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border-radius: 8px;
  background: var(--workspace-accent);
  color: var(--workspace-surface);
  font-size: 13px;
  font-weight: 800;
}

.readiness-copy {
  min-width: 0;
}

.readiness-copy > span,
.readiness-copy strong,
.readiness-copy p {
  display: block;
}

.readiness-copy > span {
  color: var(--workspace-accent);
  font-size: 10px;
  font-weight: 700;
}

.readiness-copy strong {
  margin-top: 2px;
  color: var(--workspace-ink);
  font-size: 12px;
}

.readiness-copy p {
  margin: 3px 0 0;
  color: var(--workspace-muted);
  font-size: 10px;
  line-height: 1.5;
}

.readiness-card button {
  min-height: 30px;
  padding: 0 10px;
  border: 1px solid var(--workspace-accent);
  border-radius: 7px;
  background: var(--workspace-surface);
  color: var(--workspace-accent);
  font-size: 11px;
  font-weight: 700;
  white-space: nowrap;
  cursor: pointer;
  transition: background-color 140ms ease-out, color 140ms ease-out, opacity 140ms ease-out;
}

.readiness-card button:hover:not(:disabled) {
  background: var(--workspace-accent);
  color: var(--workspace-surface);
}

.readiness-card button:disabled {
  opacity: 0.55;
  cursor: wait;
}

.generate-button {
  display: flex;
  align-items: center;
  justify-content: center;
  place-items: center;
  width: 100%;
  min-height: 44px;
  margin-top: 18px;
  border: 0;
  border-radius: 6px;
  background: var(--workspace-accent);
  color: white;
  cursor: pointer;
  box-shadow: none;
  transition: background-color 140ms ease-out, opacity 140ms ease-out;
}

.generate-button:hover:not(:disabled) {
  background: var(--workspace-accent-hover);
}

.generate-button span {
  font-size: 14px;
  font-weight: 700;
}

.generate-button:disabled {
  background: var(--workspace-border-strong);
  box-shadow: none;
  cursor: not-allowed;
}

.action-hint {
  margin: 8px 0 0;
  color: var(--workspace-subtle);
  font-size: 10px;
  text-align: center;
}

.task-progress {
  margin-top: 14px;
  padding: 14px;
  border: 1px solid var(--workspace-border);
  border-radius: 10px;
  background: var(--workspace-surface-muted);
}

.task-progress > div:first-child {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  color: var(--workspace-muted);
  font-size: 11px;
}

.progress-track {
  height: 6px;
  margin-top: 11px;
  overflow: hidden;
  border-radius: 99px;
  background: var(--workspace-border);
}

.progress-track i {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--workspace-accent);
  transition: width 300ms var(--workspace-ease);
}

.task-progress button {
  margin-top: 10px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--workspace-danger);
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}

.message {
  margin: 14px 0 0;
  padding: 12px 13px;
  border-radius: 9px;
  font-size: 11px;
  line-height: 1.55;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.message.error {
  background: oklch(0.94 0.035 28);
  color: var(--workspace-danger);
}

.message.success {
  background: var(--workspace-accent-soft);
  color: var(--workspace-accent);
}

.output-panel {
  margin-top: 14px;
  padding-top: 16px;
  border-top: 1px solid var(--workspace-border);
  animation: panel-in 300ms var(--workspace-ease);
}

.output-heading {
  display: flex;
  align-items: center;
  gap: 11px;
}

.output-heading strong,
.output-heading small {
  display: block;
}

.output-heading strong {
  font-size: 13px;
}

.output-heading small {
  margin-top: 4px;
  color: var(--workspace-subtle);
  font-size: 10px;
}

.output-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  margin-top: 14px;
}

.output-actions button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 1 1 82px;
  gap: 6px;
  min-height: 38px;
  border: 1px solid var(--workspace-border);
  border-radius: 8px;
  background: var(--workspace-surface);
  color: var(--workspace-ink);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  transition: border-color 140ms ease-out, background-color 140ms ease-out, transform 140ms var(--workspace-ease);
}

.output-actions button:hover {
  border-color: var(--workspace-border-strong);
  background: var(--workspace-surface-muted);
  transform: translateY(-1px);
}

.output-actions .dark {
  border-color: var(--workspace-accent);
  background: var(--workspace-accent);
  color: white;
}

.output-actions .dark:hover {
  border-color: var(--workspace-accent-hover);
  background: var(--workspace-accent-hover);
  color: white;
}

.advanced-entry {
  display: block;
  width: 100%;
  margin-top: 10px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--workspace-accent);
  font-size: 10px;
  font-weight: 700;
  text-align: center;
  cursor: pointer;
}

.advanced-entry:hover {
  color: var(--workspace-accent-hover);
  text-decoration: underline;
}

.advanced-entry.page-entry {
  max-width: 220px;
  margin: 18px auto 0;
}

.project-workbench {
  max-width: 1040px;
  margin: 38px auto 0;
  padding: 22px;
  border: 1px solid var(--workspace-border);
  border-radius: 16px;
  background: color-mix(in oklab, var(--workspace-surface) 94%, white);
  box-shadow: 0 18px 44px rgb(41 50 65 / 7%);
}

.workbench-header,
.delivery-heading,
.segments-review-header,
.qa-summary,
.video-review-meta,
.video-review-tools,
.delivery-controls,
.artifact-list {
  display: flex;
  align-items: center;
  gap: 10px;
}

.workbench-header,
.delivery-heading,
.segments-review-header,
.qa-summary {
  justify-content: space-between;
}

.workbench-kicker {
  display: block;
  margin-bottom: 4px;
  color: var(--workspace-accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.workbench-header h2,
.delivery-heading h3 {
  margin: 0;
  color: var(--workspace-ink);
  font-size: 18px;
  letter-spacing: -0.02em;
}

.workbench-header p,
.delivery-heading p,
.segments-review-header small,
.qa-summary span,
.video-review-meta,
.artifact-list > span {
  margin: 5px 0 0;
  color: var(--workspace-muted);
  font-size: 11px;
  line-height: 1.45;
}

.workbench-header-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 7px;
}

.workbench-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 34px;
  padding: 0 11px;
  border: 1px solid var(--workspace-border);
  border-radius: 8px;
  background: var(--workspace-surface);
  color: var(--workspace-ink);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  transition: border-color 140ms ease-out, background-color 140ms ease-out, color 140ms ease-out;
}

.workbench-button:hover:not(:disabled) {
  border-color: var(--workspace-border-strong);
  background: var(--workspace-surface-muted);
}

.workbench-button:disabled {
  cursor: not-allowed;
  opacity: 0.48;
}

.workbench-button.primary {
  border-color: var(--workspace-accent);
  background: var(--workspace-accent);
  color: white;
}

.workbench-button.primary:hover:not(:disabled) {
  border-color: var(--workspace-accent-hover);
  background: var(--workspace-accent-hover);
}

.workbench-button.dark {
  border-color: #252c38;
  background: #252c38;
  color: white;
}

.workbench-button.danger {
  color: var(--workspace-danger);
}

.workbench-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.08fr) minmax(340px, 0.92fr);
  gap: 18px;
  margin-top: 20px;
}

.video-review-panel,
.segments-review-panel,
.delivery-panel {
  min-width: 0;
  border: 1px solid var(--workspace-border);
  border-radius: 12px;
  background: var(--workspace-surface-muted);
}

.video-review-panel {
  overflow: hidden;
}

.video-review-stage {
  position: relative;
  aspect-ratio: 16 / 9;
  overflow: hidden;
  background: #171b22;
}

.video-review-stage video {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.video-caption-preview {
  position: absolute;
  right: 5%;
  left: 5%;
  display: grid;
  justify-items: center;
  gap: 2px;
  padding: 7px 11px;
  text-align: center;
  pointer-events: none;
}

.video-review-stage.position-bottom .video-caption-preview { bottom: 9%; }
.video-review-stage.position-middle .video-caption-preview { top: 50%; transform: translateY(-50%); }
.video-review-stage.position-top .video-caption-preview { top: 9%; }

.video-caption-preview span {
  display: block;
  max-width: 100%;
  line-height: 1.35;
  overflow-wrap: anywhere;
}

.video-review-meta,
.video-review-tools {
  padding: 11px 13px;
}

.video-review-meta {
  justify-content: space-between;
  margin: 0;
  border-top: 1px solid var(--workspace-border);
}

.video-review-tools {
  flex-wrap: wrap;
  border-top: 1px solid var(--workspace-border);
}

.segments-review-panel {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.segments-review-header,
.qa-summary {
  padding: 13px;
}

.segments-review-header {
  border-bottom: 1px solid var(--workspace-border);
}

.segments-review-header strong,
.qa-summary strong {
  display: block;
  color: var(--workspace-ink);
  font-size: 12px;
}

.segments-review-header small {
  display: block;
}

.segment-filter,
.replace-tools {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 9px 13px;
  border-bottom: 1px solid var(--workspace-border);
}

.segment-filter input,
.replace-tools input {
  box-sizing: border-box;
  min-width: 0;
  height: 31px;
  flex: 1;
  padding: 0 9px;
  border: 1px solid var(--workspace-border);
  border-radius: 7px;
  background: var(--workspace-surface);
  color: var(--workspace-ink);
  font: inherit;
  font-size: 11px;
}

.segment-filter span {
  flex: 0 0 auto;
  color: var(--workspace-subtle);
  font-size: 10px;
  font-weight: 700;
}

.replace-tools {
  background: color-mix(in oklab, var(--workspace-accent-soft) 35%, transparent);
}

.save-state {
  flex: 0 0 auto;
  color: var(--workspace-accent);
  font-size: 10px;
  font-weight: 800;
}

.save-state.unsaved { color: #a66a00; }

.qa-summary {
  background: color-mix(in oklab, var(--workspace-accent-soft) 55%, transparent);
}

.qa-summary.clear {
  background: color-mix(in oklab, #e7f6ed 75%, transparent);
}

.qa-summary span { display: block; }

.qa-issues {
  display: grid;
  gap: 5px;
  padding: 9px 13px;
  border-bottom: 1px solid var(--workspace-border);
}

.qa-issues button {
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--workspace-muted);
  font-size: 10px;
  text-align: left;
  cursor: pointer;
}

.qa-issues button:hover { color: var(--workspace-ink); }
.qa-issues button > span,
.segment-issue-tags span {
  display: inline-block;
  margin-right: 5px;
  padding: 1px 4px;
  border-radius: 4px;
  font-size: 9px;
  font-weight: 800;
}

.qa-issues button.error > span,
.segment-issue-tags .error {
  background: oklch(0.94 0.035 28);
  color: var(--workspace-danger);
}

.qa-issues button.warning > span,
.segment-issue-tags .warning {
  background: oklch(0.96 0.04 82);
  color: #885a00;
}

.qa-issues > span {
  color: var(--workspace-subtle);
  font-size: 10px;
}

.segment-list {
  min-height: 280px;
  max-height: 510px;
  overflow: auto;
}

.segment-row {
  display: grid;
  grid-template-columns: 29px minmax(0, 1fr);
  gap: 9px;
  padding: 10px 12px;
  border-bottom: 1px solid color-mix(in oklab, var(--workspace-border) 72%, transparent);
  cursor: pointer;
}

.segment-row:hover,
.segment-row.selected {
  background: color-mix(in oklab, var(--workspace-accent-soft) 58%, transparent);
}

.segment-row.selected {
  background: transparent;
}

.segment-row.active {
  background: transparent;
  box-shadow: none;
}
.segment-row.invalid {
  background: color-mix(in oklab, var(--danger-soft) 70%, transparent);
  box-shadow: none;
}

.segment-index {
  align-self: start;
  width: 27px;
  height: 27px;
  padding: 0;
  border: 1px solid var(--workspace-border);
  border-radius: 7px;
  background: var(--workspace-surface);
  color: var(--workspace-muted);
  font-size: 10px;
  font-weight: 800;
  cursor: pointer;
}

.segment-editor { min-width: 0; }

.segment-time-fields {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: 5px;
  margin-bottom: 6px;
  color: var(--workspace-subtle);
  font-size: 10px;
}

.segment-time-fields input,
.segment-editor textarea,
.review-status-control select {
  box-sizing: border-box;
  width: 100%;
  border: 1px solid var(--workspace-border);
  border-radius: 6px;
  background: var(--workspace-surface);
  color: var(--workspace-ink);
  font: inherit;
}

.segment-time-fields input {
  min-width: 0;
  height: 27px;
  padding: 0 6px;
  font-size: 10px;
}

.segment-editor textarea {
  display: block;
  min-height: 43px;
  margin-top: 5px;
  padding: 7px;
  resize: vertical;
  font-size: 11px;
  line-height: 1.45;
}

.segment-issue-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 6px;
}

.segment-issue-tags span {
  margin: 0;
  line-height: 1.35;
}

.delivery-panel {
  margin-top: 18px;
  padding: 16px;
  background: var(--workspace-surface);
}

.delivery-heading { align-items: flex-end; }
.delivery-heading p { max-width: 450px; }

.review-status-control,
.delivery-controls > label {
  display: grid;
  gap: 5px;
  color: var(--workspace-muted);
  font-size: 10px;
  font-weight: 700;
}

.review-status-control { min-width: 118px; }
.review-status-control select { height: 34px; padding: 0 7px; font-size: 11px; }

.delivery-controls {
  flex-wrap: wrap;
  margin-top: 15px;
}

.delivery-controls > label { min-width: 150px; }
.delivery-controls .workbench-button { align-self: end; }

.workbench-message {
  margin: 12px 0 0;
  padding: 9px 11px;
  border-radius: 8px;
  font-size: 11px;
  line-height: 1.5;
}

.workbench-message.error { background: oklch(0.94 0.035 28); color: var(--workspace-danger); }
.workbench-message.success { background: var(--workspace-accent-soft); color: var(--workspace-accent); }

.artifact-list {
  flex-wrap: wrap;
  margin-top: 14px;
  padding-top: 13px;
  border-top: 1px solid var(--workspace-border);
}

.artifact-list > span { margin: 0 4px 0 0; }

.artifact-list button {
  max-width: 260px;
  padding: 0;
  overflow: hidden;
  border: 0;
  background: transparent;
  color: var(--workspace-accent);
  font-size: 10px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
  cursor: pointer;
}

.workbench-button:focus-visible,
.segment-index:focus-visible,
.qa-issues button:focus-visible,
.segment-filter input:focus-visible,
.replace-tools input:focus-visible,
.segment-time-fields input:focus-visible,
.segment-editor textarea:focus-visible,
.review-status-control select:focus-visible,
.artifact-list button:focus-visible {
  outline: 2px solid var(--workspace-accent);
  outline-offset: 2px;
}

.settings-button:focus-visible,
.file-card:focus-visible,
.preset-option:focus-visible,
.advanced-toggle:focus-visible,
.advanced-entry:focus-visible,
.generate-button:focus-visible,
.task-progress button:focus-visible,
.output-actions button:focus-visible {
  outline: 2px solid var(--workspace-accent);
  outline-offset: 3px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@keyframes panel-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (max-width: 900px) {
  .action-column {
    max-width: none;
  }

  .workbench-layout {
    grid-template-columns: 1fr;
  }

  .segment-list {
    max-height: 430px;
  }
}

@container workspace (min-width: 900px) {
  .subtitle-layout {
    grid-template-columns: minmax(0, 1.35fr) minmax(330px, 0.82fr);
    gap: clamp(28px, 4vw, 48px);
  }

  .action-column {
    position: sticky;
    top: 0;
  }
}

@container workspace (max-width: 620px) {
  .simple-header {
    align-items: stretch;
    flex-direction: column;
    margin-bottom: 22px;
  }

  .settings-button {
    align-self: flex-start;
  }

  .file-card {
    grid-template-columns: 42px minmax(0, 1fr);
    gap: 12px;
  }

  .file-icon {
    width: 42px;
    height: 42px;
  }

  .replace-label {
    grid-column: 2;
    justify-self: start;
  }

  .analysis-card {
    grid-template-columns: 34px minmax(0, 1fr);
  }

  .route-tag {
    grid-column: 2;
    justify-self: start;
  }

  .project-workbench {
    margin-top: 26px;
    padding: 14px;
    border-radius: 12px;
  }

  .workbench-header,
  .delivery-heading,
  .segments-review-header,
  .qa-summary,
  .video-review-meta {
    align-items: flex-start;
    flex-direction: column;
  }

  .workbench-header-actions {
    justify-content: flex-start;
  }

  .video-review-meta {
    gap: 3px;
  }

  .segment-time-fields {
    grid-template-columns: 1fr;
    gap: 3px;
  }

  .segment-time-fields span {
    display: none;
  }

  .delivery-controls > label {
    width: 100%;
  }

  .delivery-controls .workbench-button {
    flex: 1 1 135px;
  }
}

@media (prefers-reduced-motion: reduce) {
  .settings-button:hover,
  .output-actions button:hover { transform: none; }
}
</style>
