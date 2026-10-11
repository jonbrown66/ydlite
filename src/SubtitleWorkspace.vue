<script setup lang="ts">
import { computed, nextTick, onActivated, onDeactivated, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import checkIcon from 'iconoir/icons/check.svg?url'
import folderIcon from 'iconoir/icons/folder.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import pageIcon from 'iconoir/icons/empty-page.svg?url'
import AppSelect from './components/ui/select/AppSelect.vue'
import { modelDescription, runtimeDescription, runtimeName } from '@/lib/assetLabels'
import PageHeader from './components/ui/PageHeader.vue'
import DisclosureChevron from './components/ui/DisclosureChevron.vue'
import AppCollapsible from './components/ui/AppCollapsible.vue'
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
const outputSettingsConfigured = ref(false)
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
  { id: 'classic', name: 'Classic', note: 'Outlined', color: '#F8F8F8', boxed: false },
  { id: 'contrast', name: 'Contrast', note: 'Background', color: '#F8F8F8', boxed: true },
  { id: 'yellow', name: 'Yellow', note: 'Accent', color: '#FFD95A', boxed: false },
]
const contentOptions = [
  { value: 'source', label: 'Source only' },
  { value: 'translated', label: 'Translated only' },
  { value: 'bilingual', label: 'Bilingual subtitles' },
]
const fontOptions = [
  { value: 'Microsoft YaHei', label: 'Microsoft YaHei' },
  { value: 'Microsoft YaHei UI', label: 'Microsoft YaHei UI' },
  { value: 'SimHei', label: 'SimHei' },
  { value: 'SimSun', label: 'SimSun' },
  { value: 'Segoe UI', label: 'Segoe UI' },
]
const positionOptions = [
  { value: 'bottom', label: 'Bottom' },
  { value: 'middle', label: 'Center' },
  { value: 'top', label: 'Top' },
]
const availableContentOptions = computed(() =>
  analysis.value?.strategy === 'extract_chinese' ? contentOptions.slice(0, 1) : contentOptions,
)
const outputSummary = computed(() => {
  const mode = analysis.value?.strategy === 'extract_chinese' ? 'source' : outputSettingsConfigured.value ? contentMode.value : 'bilingual'
  const label = mode === 'source' ? 'Source subtitles' : mode === 'translated' ? 'Chinese subtitles' : 'Bilingual subtitles'
  return `${label}${!outputSettingsConfigured.value || burnVideo.value ? ' + video' : ' only'}`
})
function toggleOutputSettings() {
  showAdvanced.value = !showAdvanced.value
  outputSettingsConfigured.value = true
  if (analysis.value?.strategy === 'extract_chinese') contentMode.value = 'source'
}
async function handlePresetKey(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const buttons = Array.from((event.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>('[role="radio"]'))
  const current = buttons.indexOf(document.activeElement as HTMLButtonElement)
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (current + (['ArrowRight', 'ArrowDown'].includes(event.key) ? 1 : -1) + buttons.length) % buttons.length
  applyPreset(subtitlePresets[next].id)
  await nextTick()
  buttons[next]?.focus()
}
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
  if (!analysis.value) return 'Choose video'
  if (processing.value) return 'Generating…'
  return 'Generate subtitles'
})
const stageLabel = computed(() => {
  if (!progress.value) return ''
  const labels: Record<string, string> = {
    extracting: 'Transcribing…',
    uploading: 'Transcribing…',
    transcribing: 'Transcribing…',
    whisper_transcription: 'Transcribing…',
    bing_translation: 'Translating…',
    custom_ai_translation: 'Translating…',
    translate: 'Translating…',
    polish: 'Reviewing…',
    exporting: 'Exporting…',
    burning: 'Exporting…',
    completed: 'Completed',
    rate_limited: 'Waiting for service…',
  }
  return labels[progress.value.stage] ?? 'Processing…'
})
const automationFlowLabel = computed(() =>
  analysis.value?.strategy === 'extract_chinese'
    ? 'Extract → Export'
    : analysis.value?.strategy === 'translate_subtitle'
      ? 'Extract → Translate → Export'
      : 'Transcribe → Translate → Export',
)
const processingPlan = computed(() => {
  if (!analysis.value) return 'Choose a video'
  if (analysis.value.strategy === 'extract_chinese') return 'Extract subtitles'
  if (settings.value.processingMode === 'local_custom') {
    return analysis.value.strategy === 'translate_subtitle' ? 'Custom AI' : 'Whisper + Custom AI'
  }
  return analysis.value.strategy === 'translate_subtitle' ? 'Bing Translate' : 'Whisper + Bing'
})
const processingTimeEstimate = computed(() => {
  if (!analysis.value) return ''
  if (analysis.value.strategy === 'extract_chinese') return 'Less than 1 min'
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
  return `Estimated ${Math.max(1, Math.ceil(low))}–${Math.max(2, Math.ceil(high))} min`
})
const outputHeading = computed(() =>
  videoOutput.value || (!burnVideo.value && subtitleOutput.value) ? 'Completed' : 'Subtitles ready',
)
const generateHint = computed(() => {
  if (!analysis.value) return ''
  if (needsCustomAi.value && (!settings.value.hasOpenaiApiKey || !settings.value.openaiModel)) return 'Configure your AI provider'
  if (needsLocalAssets.value && !localModelReady.value) return 'Install a speech model'
  if (needsLocalAssets.value && !localRuntimeReady.value) return 'Install a runtime'
  return ''
})
const readinessCard = computed<ReadinessCard | null>(() => {
  if (!analysis.value || canGenerate.value || processing.value || analyzing.value) return null
  if (needsCustomAi.value && (!settings.value.hasOpenaiApiKey || !settings.value.openaiModel)) {
    return {
      title: 'AI provider required',
      description: 'Add your endpoint, model and API key.',
      action: 'open-settings',
      actionLabel: 'Configure provider',
      setup: 'custom',
    }
  }
  if (needsLocalAssets.value && !localModelReady.value) {
    const model = selectedWhisperModel.value
    return {
      title: `Install ${model?.name ?? 'Whisper model'}`,
      description: model ? `${modelDescription(model.id)} ${model.downloadSize}.` : 'Install a local speech model to continue.',
      action: 'install-model',
      actionLabel: preparingAsset.value === 'model' ? 'Downloading…' : `Download${model ? ` ${model.downloadSize}` : ''}`,
      setup: 'local',
    }
  }
  if (needsLocalAssets.value && !localRuntimeReady.value) {
    const runtime = selectedWhisperRuntime.value
    return {
      title: `Install ${runtime ? runtimeName(runtime.id) : 'Whisper runtime'}`,
      description: runtime ? `${runtimeDescription(runtime.id)} ${runtime.downloadSize}.` : 'Install a runtime for local transcription.',
      action: 'install-runtime',
      actionLabel: preparingAsset.value === 'runtime' ? 'Installing…' : `Install${runtime ? ` ${runtime.downloadSize}` : ''}`,
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
  if (status === 'paused') return 'Paused. Ready to resume.'
  if (status === 'failed') return 'Failed. Ready to retry.'
  if (status === 'processing' || status === 'translate' || status === 'burning') return 'Processing project…'
  if (hasUnsavedSegments.value) return 'Unsaved subtitle changes'
  if (reviewStatus.value === 'approved') return 'Approved'
  return hasProjectSegments.value ? 'Needs review' : 'Ready to generate'
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
      issues.push({ segmentId: segment.id, severity: 'error', message: `Segment ${index + 1}: missing source text` })
    }
    if (segment.endMs <= segment.startMs) {
      issues.push({ segmentId: segment.id, severity: 'error', message: `Segment ${index + 1}: invalid timing` })
    }
    if (index > 0 && segment.startMs < previousEnd) {
      issues.push({ segmentId: segment.id, severity: 'error', message: `Segment ${index + 1}: overlaps previous segment` })
    }
    if (requireTranslation && !translated) {
      issues.push({ segmentId: segment.id, severity: 'error', message: `Segment ${index + 1}: missing translation` })
    }
    if (duration > 0 && duration < 650) {
      issues.push({ segmentId: segment.id, severity: 'warning', message: `Segment ${index + 1}: duration below 0.65 s` })
    }

    const visibleText = contentMode.value === 'source'
      ? source
      : contentMode.value === 'translated'
        ? translated
        : `${source}${translated}`
    const characterCount = visibleText.replace(/\s/g, '').length
    const seconds = Math.max(duration / 1_000, 0.1)
    if (characterCount / seconds > 16) {
      issues.push({ segmentId: segment.id, severity: 'warning', message: `Segment ${index + 1}: reading speed too fast` })
    }
    if (visibleText.split('\n').length > 2 || characterCount > 42) {
      issues.push({ segmentId: segment.id, severity: 'warning', message: `Segment ${index + 1}: consider a line break` })
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
  return filename.replace(/\.[^./\\]+$/, '') || 'Subtitle projects'
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
    if (!quiet) notice.value = 'Project saved.'
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
    if (!quiet) editorNotice.value = 'Subtitles saved.'
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
    editorError.value = 'Use the time format 00:00:00.000.'
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
  const punctuation = /[，.！？、,.!?;；\s]/g
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
    editorError.value = 'Add text or adjust the playhead before splitting.'
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
    editorError.value = 'No next segment to merge.'
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
    editorError.value = 'Adjust segment timing to create space.'
    return
  }
  const segment: SubtitleSegment = {
    id: `manual-${Date.now()}`,
    startMs,
    endMs,
    sourceText: 'Enter source text',
    translatedText: contentMode.value === 'source' ? null : 'Enter translation',
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
    editorError.value = 'Enter text to find.'
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
    editorNotice.value = 'No matching text.'
    return
  }
  rememberSegments()
  replaceSegments(segments)
  editorNotice.value = `Replaced text in ${changed} segments.`
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
    editorError.value = 'Resolve review errors before exporting.'
    return
  }
  if (!await saveSegmentEdits(true)) return
  const destination = await chooseOutputPath(exportFormat.value)
  if (!destination) return
  const taskProject = project.value
  activity.beginSubtitle({
    projectId: taskProject.id,
    title: subtitleTaskTitle(taskProject),
    detail: `Exporting ${exportFormat.value.toUpperCase()} subtitles…`,
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
    editorNotice.value = 'Subtitles exported.'
    activity.finishSubtitle(taskProject.id, { detail: `${exportFormat.value.toUpperCase()} subtitles exported` })
  } catch (value) {
    editorError.value = errorText(value)
    activity.finishSubtitle(taskProject.id, { state: taskFailureState(value), detail: 'Subtitle export failed' })
  } finally {
    exporting.value = false
  }
}

async function burnCurrentVideo() {
  if (!project.value) return
  editorError.value = ''
  editorNotice.value = ''
  if (qaErrors.value.length) {
    editorError.value = 'Resolve review errors before creating the video.'
    return
  }
  if (!await saveSegmentEdits(true)) return
  const destination = await chooseOutputPath('srt', true)
  if (!destination) return
  const taskProject = project.value
  activity.beginSubtitle({
    projectId: taskProject.id,
    title: subtitleTaskTitle(taskProject),
    detail: 'Rendering video…',
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
    editorNotice.value = 'Video ready.'
    activity.finishSubtitle(taskProject.id, { detail: 'Video ready' })
  } catch (value) {
    editorError.value = errorText(value)
    activity.finishSubtitle(taskProject.id, { state: taskFailureState(value), detail: 'Video export failed' })
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
    notice.value = 'Ready to resume.'
    return
  }
  const taskProject = project.value
  activity.beginSubtitle({
    projectId: taskProject.id,
    title: subtitleTaskTitle(taskProject),
    detail: 'Resuming subtitles…',
  })
  processing.value = true
  error.value = ''
  try {
    let current = project.value
    if (!hasTranslations) {
      current = await translateProject(current.id, current.targetLanguage ?? 'zh-CN')
    }
    synchronizeProject(current)
    notice.value = 'Subtitles completed.'
    activity.finishSubtitle(taskProject.id, { detail: 'Subtitles completed' })
  } catch (value) {
    error.value = errorText(value)
    activity.finishSubtitle(taskProject.id, { state: taskFailureState(value), detail: 'Unable to resume' })
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
      error.value = 'Speech model unavailable. Choose another in Settings.'
      return
    }
    preparingAsset.value = 'model'
    try {
      await downloadWhisperModel(model.id)
      await refreshLocalAssets()
      notice.value = `${model.name} is ready.`
    } catch (value) {
      error.value = errorText(value)
    } finally {
      preparingAsset.value = null
    }
    return
  }

  const runtime = selectedWhisperRuntime.value
  if (!runtime) {
    error.value = 'Runtime unavailable. Choose another in Settings.'
    return
  }
  preparingAsset.value = 'runtime'
  try {
    await downloadWhisperRuntime(runtime.id)
    await refreshLocalAssets()
    notice.value = `${runtimeName(runtime.id)} is ready.`
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
  return hours ? `${hours} hr ${minutes} min` : `${minutes || 1} min`
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
    zh: 'Chinese', zho: 'Chinese', chi: 'Chinese', cmn: 'Chinese', 'zh-cn': 'Chinese',
    en: 'English', eng: 'English', ja: 'Japanese', jpn: 'Japanese', ko: 'Korean', kor: 'Korean',
    fr: 'French', fra: 'French', fre: 'French', de: 'German', deu: 'German', ger: 'German',
    es: 'Spanish', spa: 'Spanish', ru: 'Russian', rus: 'Russian',
  }
  return names[value] ?? (value && value !== 'und' ? value.toUpperCase() : 'Auto-detect')
}

async function chooseProject(projectId: string) {
  showAdvanced.value = false
  outputSettingsConfigured.value = false
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
      notice.value = `Project opened. Unable to analyze source: ${errorText(value)}`
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
  outputSettingsConfigured.value = false
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
      : outputSettingsConfigured.value
        ? contentMode.value
        : 'bilingual'
    const automaticBurn = outputSettingsConfigured.value ? burnVideo.value : true
    contentMode.value = automaticContent
    burnVideo.value = automaticBurn
    let current = await ensureProject()
    activityProjectId = current.id
    activity.beginSubtitle({
      projectId: current.id,
      title: subtitleTaskTitle(current),
      detail: 'Generating subtitles…',
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
      ? 'Subtitles and video ready.'
      : 'Subtitles ready.'
    activity.finishSubtitle(current.id, {
      detail: automaticBurn ? 'Subtitles and video ready' : 'Subtitles ready',
    })
    await nextTick()
    resultPanel.value?.scrollIntoView({ behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth', block: 'nearest' })
  } catch (value) {
    error.value = errorText(value)
    if (activityProjectId) {
      activity.finishSubtitle(activityProjectId, {
        state: taskFailureState(value),
        detail: 'Subtitle generation failed',
      })
    }
  } finally {
    processing.value = false
  }
}

async function cancel() {
  if (!project.value) return
  activity.updateSubtitle(project.value.id, { detail: 'Cancelling…' })
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
    <PageHeader title="Subtitles" />

    <div class="subtitle-layout">
      <main class="source-column">
        <div class="section-caption">
          <div>
            <strong>Video</strong>
          </div>
        </div>

        <button class="file-card" :class="{ 'has-source': sourcePath }" type="button" :disabled="analyzing || processing" @click="chooseVideo()">
          <span class="file-icon"><i class="icon" :style="iconStyle(mediaVideoIcon)" /></span>
          <span class="file-copy">
            <strong :title="sourcePath">{{ sourcePath ? sourcePath.replace(/^.*[\\/]/, '') : 'Choose a video' }}</strong>
          </span>
          <span class="replace-label">{{ sourcePath ? 'Replace' : 'Browse' }}</span>
        </button>

        <div v-if="analyzing" class="analysis-card loading">
          <i />
          <div><strong>Analyzing video…</strong></div>
        </div>

        <div v-else-if="analysis" class="analysis-card">
          <span class="result-mark" :class="analysis.strategy"><i class="icon" :style="iconStyle(checkIcon)" /></span>
          <div class="analysis-copy">
            <strong>{{ analysis.recommendedTrack ? 'Subtitles found' : 'No embedded subtitles' }}</strong>
            <span>
              {{ durationLabel }}
              <template v-if="analysis.recommendedTrack">
                · {{ languageName(analysis.recommendedTrack.language) }}
                · {{ analysis.recommendedTrack.codec.toUpperCase() }}
              </template>
              <template v-else> · Detect language</template>
            </span>
          </div>
          <span class="route-tag">
            {{ analysis.strategy === 'extract_chinese' ? 'Extract subtitles' : analysis.strategy === 'translate_subtitle' ? 'Translate subtitles' : 'Transcribe audio' }}
          </span>
        </div>
        <section v-if="analysis && showAdvanced && burnVideo" class="preview-section" aria-label="Subtitle preview">
          <div class="style-heading"><span>Style</span><small v-if="stylePreset === 'custom'">Custom</small></div>
          <div class="preset-grid" role="radiogroup" aria-label="Style presets" @keydown="handlePresetKey">
            <button v-for="preset in subtitlePresets" :key="preset.id" type="button" class="preset-option" :class="{ active: stylePreset === preset.id }" role="radio" :aria-checked="stylePreset === preset.id" :tabindex="stylePreset === preset.id || (stylePreset === 'custom' && preset.id === 'classic') ? 0 : -1" :disabled="processing" @click="applyPreset(preset.id)">
              <i :style="{ color: preset.color }">Aa</i><strong>{{ preset.name }}</strong>
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
          <p class="preview-disclaimer" title="Styles apply to video. SRT is plain text.">Preview</p>
        </section>
      </main>

      <aside class="action-column">
        <div class="section-caption">
          <div>
              <strong>Output</strong>
          </div>
        </div>

        <section class="action-panel">
          <div class="action-options">
          <div v-if="analysis" class="route-summary">
            <strong>{{ processingPlan }}</strong>
            <small>
              {{ processingTimeEstimate }}
            </small>
          </div>
          <div v-else class="route-summary pending">
            <strong>Choose a video to begin</strong>
          </div>

          <button v-if="analysis" class="advanced-entry page-entry" type="button" :aria-label="showAdvanced ? 'Hide output options' : 'Customize output'" :aria-expanded="showAdvanced" :disabled="processing" @click="toggleOutputSettings">
            {{ showAdvanced ? 'Hide options' : 'Customize' }}
            <DisclosureChevron :open="showAdvanced" />
          </button>

          <p v-if="analysis && !showAdvanced" class="action-hint" title="Your original video is preserved">{{ outputSummary }}</p>

          <AppCollapsible :open="Boolean(analysis && showAdvanced)">
          <label class="burn-option">
            <input v-model="burnVideo" type="checkbox" :disabled="processing || !analysis" />
            <span><strong>Burn subtitles into video</strong></span>
          </label>

          <fieldset class="subtitle-style-editor" :disabled="processing || !analysis">
            <div class="style-field compact-field">
              <span>Content</span>
              <AppSelect
                v-model="contentMode"
                :options="availableContentOptions"
                aria-label="Content"
              />
            </div>

            <template v-if="burnVideo">


            <div v-if="contentMode === 'bilingual'" class="translation-link">
              <label>
                <input v-model="syncTranslatedStyle" type="checkbox" />
                <span><strong>Match source style</strong></span>
              </label>
            </div>

            <button class="advanced-toggle" type="button" :aria-expanded="advancedStyleOpen" @click="advancedStyleOpen = !advancedStyleOpen">
              <span>{{ advancedStyleOpen ? 'Hide advanced options' : 'Advanced options' }}</span>
              <DisclosureChevron :open="advancedStyleOpen" />
            </button>

            <div class="advanced-wrap" :class="{ open: advancedStyleOpen }" :inert="!advancedStyleOpen" :aria-hidden="!advancedStyleOpen">
              <div class="advanced-inner">
                <div class="advanced-grid primary-style-row">
                  <div class="style-field font-field">
                    <span>Font</span>
                    <AppSelect
                      v-model="subtitleStyle.fontFamily"
                      :options="fontOptions"
                      aria-label="Subtitle font"
                      @update:model-value="markCustom"
                    />
                  </div>
                  <div class="style-field">
                    <span>Position</span>
                    <AppSelect
                      v-model="subtitleStyle.position"
                      :options="positionOptions"
                      aria-label="Subtitle position"
                      @update:model-value="markCustom"
                    />
                  </div>
                </div>

                <section class="advanced-section">
                  <span class="advanced-section-title">Size and position</span>
                  <div class="advanced-grid slider-grid">
                    <label class="style-field range-field">
                      <span>Source size <b>{{ subtitleStyle.fontSize }}</b></span>
                      <input v-model.number="subtitleStyle.fontSize" type="range" min="24" max="80" step="1" @input="markCustom" />
                    </label>
                    <label class="style-field range-field">
                      <span>Vertical margin <b>{{ subtitleStyle.marginVertical }}</b></span>
                      <input v-model.number="subtitleStyle.marginVertical" type="range" min="20" max="180" step="4" @input="markCustom" />
                    </label>
                  </div>
                </section>

                <section class="advanced-section">
                  <span class="advanced-section-title">Color and outline</span>
                  <div class="color-grid">
                    <label class="color-control">
                      <span>Text</span>
                      <input v-model="subtitleStyle.primaryColor" type="color" aria-label="Text color" @input="markCustom" />
                    </label>
                    <label class="color-control">
                      <span>Outline</span>
                      <input v-model="subtitleStyle.outlineColor" type="color" aria-label="Outline color" @input="markCustom" />
                    </label>
                  </div>
                  <div class="outline-row">
                    <label class="style-field range-field">
                      <span>Outline width <b>{{ subtitleStyle.outlineWidth }}</b></span>
                      <input v-model.number="subtitleStyle.outlineWidth" type="range" min="0" max="8" step="0.5" @input="markCustom" />
                    </label>
                    <label class="style-check compact-check">
                      <input v-model="subtitleStyle.bold" type="checkbox" @change="markCustom" />
                      <span>Bold</span>
                    </label>
                  </div>
                </section>

                <section v-if="contentMode === 'bilingual'" class="advanced-section bilingual-section">
                  <span class="advanced-section-title">Bilingual layout</span>
                  <div class="bilingual-controls">
                    <template v-if="!syncTranslatedStyle">
                      <label class="style-field range-field">
                        <span>Translation size <b>{{ subtitleStyle.translatedFontSize }}</b></span>
                        <input v-model.number="subtitleStyle.translatedFontSize" type="range" min="24" max="80" step="1" @input="markCustom" />
                      </label>
                      <label class="color-control translated-color">
                        <span>Translation</span>
                        <input v-model="subtitleStyle.translatedColor" type="color" aria-label="Translation color" @input="markCustom" />
                      </label>
                    </template>
                    <label class="style-check order-check">
                      <input v-model="subtitleStyle.translatedFirst" type="checkbox" @change="markCustom" />
                      <span>Translation above source</span>
                    </label>
                  </div>
                </section>
              </div>
            </div>
            </template>
          </fieldset>
          </AppCollapsible>

          <section v-if="!hasProjectSegments && readinessCard" class="readiness-card" aria-live="polite">
            <span class="readiness-mark">!</span>
            <div class="readiness-copy">
              <span>Setup required</span>
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
          </div>
          <div v-if="processing || error || notice || subtitleOutput || videoOutput" class="action-execute">


          <div v-if="processing" class="task-progress">
            <div><strong>{{ stageLabel || 'Processing…' }}</strong><span>{{ Math.round(progress?.percent ?? 0) }}%</span></div>
            <div class="progress-track" role="progressbar" aria-label="Subtitle progress" :aria-valuenow="progress?.percent ?? 0" :aria-valuemin="0" :aria-valuemax="100"><i :style="{ width: `${progress?.percent ?? 0}%` }" /></div>
            <button type="button" @click="cancel">Cancel task</button>
          </div>

          <p v-if="error" class="message error">{{ error }}</p>
          <p v-if="notice" class="message success">{{ notice }}</p>

          <div v-if="(subtitleOutput || videoOutput) && !processing" ref="resultPanel" class="output-panel">
            <div class="output-heading"><span class="result-mark done"><i class="icon" :style="iconStyle(checkIcon)" /></span><strong>{{ outputHeading }}</strong></div>
            <div class="output-actions">
              <button v-if="videoOutput" class="dark" type="button" @click="openPath(videoOutput)"><i class="icon" :style="iconStyle(mediaVideoIcon)" />Play video</button>
              <button v-if="subtitleOutput" type="button" @click="openPath(subtitleOutput)"><i class="icon" :style="iconStyle(pageIcon)" />Open subtitles</button>
              <button type="button" @click="openParentFolder(subtitleOutput || videoOutput)"><i class="icon" :style="iconStyle(folderIcon)" />Open folder</button>
            </div>
          </div>
          </div>
        </section>
      </aside>
    </div>
    <footer v-if="!hasProjectSegments || (project && !subtitleOutput && !videoOutput) || (project && ['paused', 'failed'].includes(project.status))" class="subtitle-actions">
      <button v-if="project && ['paused', 'failed'].includes(project.status)" class="generate-button" type="button" :disabled="processing || analyzing" @click="continueProject">{{ processing ? 'Resuming…' : 'Resume task' }}</button>
      <button v-else class="generate-button" type="button" :disabled="analysis ? !canGenerate : analyzing" @click="generateChineseSubtitles">{{ actionLabel }}</button>
      <p v-if="!hasProjectSegments && generateHint" class="action-hint">{{ generateHint }}</p>
    </footer>
  </section>
</template>

<style scoped>

.simple-subtitle { display: flex; flex-direction: column; overflow: hidden; padding-bottom: 20px; color: var(--ink); }
.simple-subtitle > .page-header { flex-shrink: 0; }
.subtitle-layout { display: grid; grid-template-columns: repeat(auto-fit,minmax(min(100%,18rem),1fr)); gap: 16px; flex: 1; min-block-size: 0; inline-size: 100%; max-inline-size: 1040px; margin-inline: auto; padding: 1px 1px 16px; overflow-y: auto; overscroll-behavior: contain; scrollbar-gutter: stable; scrollbar-width: thin; align-items: start; align-content: start; }
.source-column, .action-column { min-width: 0; border-radius: var(--radius-panel); background: var(--surface); box-shadow: var(--panel-shadow); }
.section-caption { padding: 14px 20px; border-bottom: 1px solid var(--border); border-radius: var(--radius-panel) var(--radius-panel) 0 0; background: var(--surface-subtle); }
.section-caption strong { font-size: 16px; font-weight: 600; }
.file-card { display: flex; align-items: center; gap: 12px; width: calc(100% - 32px); margin: 16px; padding: 12px 16px; border: 1px dashed var(--border-strong); border-radius: var(--radius-control); background: var(--surface-subtle); color: var(--ink); text-align: left; }
.file-card:hover:not(:disabled) { background: var(--surface-subtle); border-color: var(--primary); }
.file-card.has-source { border-style: solid; border-color: var(--border); background: var(--surface); }
.file-icon { display: grid; place-items: center; flex-shrink: 0; width: 36px; height: 36px; border-radius: var(--radius-control); background: var(--surface-hover); color: var(--muted); }
.file-icon .icon { width: 18px; height: 18px; }
.file-copy { min-width: 0; flex: 1; }
.file-copy strong { display: block; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.file-copy small { display: block; color: var(--muted); font-size: 14px; line-height: 1.5; margin-top: 4px; overflow-wrap: anywhere; }
.replace-label { color: var(--primary); white-space: nowrap; align-self: center; }
.analysis-card { display: flex; align-items: flex-start; gap: 10px; margin: 0 16px 16px; padding: 10px 16px; border-radius: var(--radius-control); background: var(--surface-subtle); }
.analysis-copy { min-width: 0; flex: 1; }
.analysis-copy strong, .analysis-copy span { display: block; }
.analysis-copy span { color: var(--muted); margin-top: 2px; }
.result-mark { display: flex; align-items: center; height: 21px; color: var(--success); flex-shrink: 0; }
.result-mark.done { color: var(--success); }
.route-tag { display: none; }
.loading i { width: 16px; height: 16px; margin-top: 3px; border: 2px solid var(--border); border-top-color: var(--primary); border-radius: 50%; animation: spin 800ms linear infinite; flex-shrink: 0; }
.loading strong, .loading span { display: block; }
.loading span { margin-top: 2px; color: var(--muted); }
.action-panel { display: flex; flex-direction: column; }
.action-options { padding: 16px 20px; }
.action-execute { flex-shrink: 0; padding: 16px 20px; border-top: 1px solid var(--border); }
.route-summary > span, .route-summary strong, .route-summary small { display: block; }
.route-summary > span { color: var(--subtle); }
.route-summary strong { margin-top: 4px; font-size: 16px; font-weight: 600; }
.route-summary small { font-size: 14px; color: var(--muted); margin-top: 4px; line-height: 1.5; }
.route-summary.pending { padding: 12px 0; }
.output-actions button, .readiness-card button { display: inline-flex; justify-content: center; align-items: center; gap: 6px; min-height: 36px; padding: 0 12px; border: 0; border-radius: var(--radius-control); background: var(--surface); color: var(--ink); box-shadow: var(--control-shadow); }
.output-actions button:hover, .readiness-card button:hover { background: var(--surface-hover); }
.advanced-entry { display: flex; align-items: center; justify-content: space-between; width: 100%; min-height: 36px; margin-top: 16px; padding: 0 10px; border: 0; border-radius: var(--radius-control); background: var(--surface-subtle); color: var(--ink); text-align: left; box-shadow: 0 0 0 1px var(--border); }
.advanced-entry:hover:not(:disabled) { background: var(--surface-hover); }
.action-hint { margin: 8px 0 0; color: var(--muted); font-size: 14px; }
.burn-option { display: flex; align-items: center; gap: 8px; padding: 16px 0; margin-top: 12px; border-top: 1px solid var(--border); }
.subtitle-style-editor { min-width: 0; margin: 0; padding: 0; border: 0; }
.style-field { display: grid; gap: 6px; min-width: 0; color: var(--muted); }
.compact-field { grid-template-columns: auto minmax(0,1fr); align-items: center; gap: 16px; }
.style-heading { display: flex; justify-content: space-between; margin: 20px 0 8px; color: var(--muted); }
.style-heading small { font-size: 14px; color: var(--primary); }
.preset-grid { display: grid; grid-template-columns: repeat(auto-fit,minmax(min(100%,4.5rem),1fr)); gap: 4px; padding: 4px; border-radius: 12px; background: var(--surface-hover); }
.preset-option { display: flex; align-items: center; justify-content: center; gap: 8px; min-width: 0; padding: 8px 6px; border: 0; border-radius: var(--radius-control); background: transparent; color: var(--muted); box-shadow: none; }
.preset-option:hover { background: var(--surface-hover); }
.preset-option.active { background: var(--surface); color: var(--ink); box-shadow: var(--control-shadow); }
.preset-option.active:hover { background: var(--surface-subtle); }
.preset-option i { display: grid; place-items: center; flex-shrink: 0; width: 24px; height: 24px; border-radius: 4px; background: #3b4352 !important; font-size: 14px; font-style: normal; text-shadow: 0 1px 1px #000; }
.preset-option strong, .preset-option small { display: block; }
.preset-option small { font-size: 14px; color: var(--muted); margin-top: 2px; }
.subtitle-preview { display: flex; justify-content: center; align-items: flex-end; height: 144px; margin-top: 12px; padding: 18px; border-radius: var(--radius-control); background: #242428; box-shadow: inset 0 0 0 1px rgb(255 255 255 / 6%); overflow: hidden; }
.subtitle-preview.position-top { align-items: flex-start; }
.subtitle-preview.position-middle { align-items: center; }
.preview-caption { display: flex; flex-direction: column; align-items: center; max-width: 100%; text-align: center; }
.preview-caption span { overflow-wrap: anywhere; line-height: 1.5; }
.preview-disclaimer { margin: 8px 0 0; color: var(--subtle); font-size: 14px; line-height: 1.5; }
.translation-link { margin-top: 16px; }
.translation-link label { display: flex; align-items: flex-start; gap: 8px; }
.translation-link input { margin-top: 4px; }
.translation-link strong, .translation-link small { display: block; }
.translation-link small { color: var(--muted); font-size: 14px; margin-top: 2px; }
.advanced-toggle { display: flex; align-items: center; justify-content: space-between; width: 100%; min-height: 40px; margin-top: 16px; padding: 0; border: 0; border-top: 1px solid var(--border); background: transparent; color: var(--ink); }
.advanced-wrap { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 180ms var(--ease-out); }
.advanced-wrap.open { grid-template-rows: 1fr; }
.advanced-inner { min-height: 0; overflow: hidden; }
.advanced-grid, .color-grid { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 16px; }
.advanced-section { padding: 16px 0; border-bottom: 1px solid var(--border); }
.advanced-section:last-child { border-bottom: 0; }
.advanced-section-title { display: block; margin-bottom: 12px; font-weight: 500; }
.range-field > span { display: flex; justify-content: space-between; gap: 8px; }
.range-field input { width: 100%; margin: 6px 0; }
input[type=range] { appearance: none; height: 4px; border-radius: 99px; background: var(--border-strong); cursor: pointer; }
input[type=range]::-webkit-slider-thumb { appearance: none; width: 16px; height: 16px; border-radius: 50%; background: var(--surface); box-shadow: 0 0 0 1px var(--border-strong), 0 1px 3px rgb(24 24 27 / 12%); }
input[type=range]::-moz-range-thumb { width: 16px; height: 16px; border: 1px solid var(--border-strong); border-radius: 50%; background: var(--surface); }
input[type=range]:active::-webkit-slider-thumb { box-shadow: 0 0 0 2px var(--primary); }
.color-control { display: flex; align-items: center; justify-content: space-between; gap: 8px; color: var(--muted); }
.color-control input { width: 36px; height: 28px; padding: 2px; border: 1px solid var(--border); border-radius: 6px; background: var(--surface); }
input[type=color]::-webkit-color-swatch-wrapper { padding: 2px; }
input[type=color]::-webkit-color-swatch { border: 0; border-radius: 3px; }
input[type=color]::-moz-color-swatch { border: 0; border-radius: 3px; }
.outline-row, .bilingual-controls { display: grid; gap: 12px; margin-top: 12px; }
.style-check { display: flex; align-items: center; gap: 8px; }
.readiness-card { display: flex; align-items: flex-start; flex-wrap: wrap; gap: 10px; padding: 12px 16px; margin-top: 20px; border-radius: 8px; background: var(--warning-soft); }
.readiness-mark { color: var(--warning); line-height: 21px; }
.readiness-copy { flex: 1; min-width: 160px; }
.readiness-copy > span { color: var(--muted); }
.readiness-copy strong { display: block; margin-top: 2px; font-weight: 500; }
.readiness-copy p { margin: 4px 0 0; color: var(--muted); }
.generate-button { display: flex; align-items: center; justify-content: center; width: 100%; min-height: 36px; margin-top: 0; padding: 8px 12px; border: 0; border-radius: var(--radius-control); background: var(--primary-fill); color: white; font-size: 14px; font-weight: 500; box-shadow: var(--primary-shadow); }
.generate-button:hover:not(:disabled) { background: var(--primary-fill-hover); }
.generate-button:active:not(:disabled) { background: var(--primary-hover); box-shadow: 0 0 0 1px var(--primary-hover), inset 0 1px 2px rgb(24 24 27 / 16%); }
button:disabled { opacity: 0.45; }
.task-progress { margin-top: 20px; }
.task-progress > div:first-child { display: flex; justify-content: space-between; gap: 12px; font-variant-numeric: tabular-nums; }
.task-progress .progress-track i { display: block; height: 100%; background: var(--primary); }
.task-progress button { border: 0; padding: 0; background: transparent; color: var(--muted); }
.message { margin: 16px 0 0; padding: 12px 16px; border-radius: 8px; overflow-wrap: anywhere; }
.message.error { color: var(--danger); background: var(--danger-soft); }
.message.success { color: var(--success); background: var(--success-soft); }
.output-panel { border-top: 1px solid var(--border); margin-top: 20px; padding-top: 16px; }
.output-heading { display: flex; align-items: center; gap: 8px; }
.output-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 12px; }
.output-actions .dark { background: var(--primary-fill); color: white; box-shadow: var(--primary-shadow); }
.output-actions .dark:hover { background: var(--primary-fill-hover); }
.preview-section { padding: 16px 20px; border-top: 1px solid var(--border); }
.preview-section .style-heading { margin: 0 0 12px; }
.subtitle-actions { display: flex; flex-direction: row-reverse; align-items: center; gap: 16px; flex-shrink: 0; width: 100%; max-width: 1040px; margin: 0 auto; padding: 12px 16px; border: 0; border-radius: var(--radius-panel); background: var(--surface); box-shadow: var(--control-shadow); }
.subtitle-actions .generate-button { width: auto; min-width: 200px; }
.subtitle-actions .action-hint { margin: 0; }
@keyframes spin { to { transform: rotate(360deg); } }
@container workspace (max-width: 760px) { .preset-option { display: grid; justify-items: center; gap: 4px; padding: 8px 4px; } }
@media (prefers-reduced-motion: reduce) { .loading i { animation: none; } .advanced-wrap { transition: none; } }

</style>
