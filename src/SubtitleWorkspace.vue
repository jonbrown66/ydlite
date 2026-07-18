<script setup lang="ts">
import { computed, nextTick, onActivated, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import checkIcon from 'iconoir/icons/check.svg?url'
import folderIcon from 'iconoir/icons/folder.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import pageIcon from 'iconoir/icons/empty-page.svg?url'
import settingsIcon from 'iconoir/icons/settings.svg?url'
import {
  analyzeSubtitleSource,
  burnSubtitles,
  cancelSubtitleTask,
  createSubtitleProject,
  estimateTranscriptionCost,
  exportSubtitles,
  getGeminiSettings,
  importSubtitleTrack,
  listSubtitleProjects,
  listWhisperModels,
  listWhisperRuntimes,
  onSubtitleProgress,
  openParentFolder,
  openPath,
  selectMediaFile,
  startGeminiTranscription,
  startGeminiTranslation,
  startBingTranslation,
  startOpenAiCompatibleTranslation,
  startWhisperTranscription,
} from './api/tauri'
import type {
  CostEstimate,
  GeminiSettings,
  MediaSubtitleAnalysis,
  SubtitleProgressEvent,
  SubtitleProject,
} from './types'

const props = defineProps<{ initialSourcePath?: string }>()
const emit = defineEmits<{ consumedInitial: [] }>()
const router = useRouter()

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
})
const burnVideo = ref(true)
const analyzing = ref(false)
const processing = ref(false)
const progress = ref<SubtitleProgressEvent | null>(null)
const estimate = ref<CostEstimate | null>(null)
const error = ref('')
const notice = ref('')
const subtitleOutput = ref('')
const videoOutput = ref('')
const localModelReady = ref(false)
const localRuntimeReady = ref(false)
const resultPanel = ref<HTMLElement | null>(null)
let unlisten: undefined | (() => void)

const needsGemini = computed(() =>
  settings.value.processingMode === 'gemini' && analysis.value?.strategy !== 'extract_chinese',
)
const needsLocalAssets = computed(() =>
  settings.value.processingMode !== 'gemini' && analysis.value?.strategy === 'transcribe_audio',
)
const needsCustomAi = computed(() =>
  settings.value.processingMode === 'local_custom' && analysis.value?.strategy !== 'extract_chinese',
)
const canGenerate = computed(() =>
  Boolean(
    analysis.value
    && !analyzing.value
    && !processing.value
    && (!needsGemini.value || settings.value.hasApiKey)
    && (!needsCustomAi.value || (settings.value.hasOpenaiApiKey && Boolean(settings.value.openaiModel)))
    && (!needsLocalAssets.value || (localModelReady.value && localRuntimeReady.value)),
  ),
)
const actionLabel = computed(() => {
  if (!analysis.value) return '选择视频'
  if (processing.value) return '正在生成中文字幕…'
  if (analysis.value.strategy === 'extract_chinese') return '提取中文字幕'
  if (analysis.value.strategy === 'translate_subtitle') return '翻译成中文字幕'
  return '生成中文字幕'
})
const stageLabel = computed(() => {
  if (!progress.value) return ''
  if (
    progress.value.message
    && (
      progress.value.recoverable
      || ['uploading', 'rate_limited', 'burning'].includes(progress.value.stage)
    )
  ) {
    return progress.value.message
  }
  const labels: Record<string, string> = {
    extracting: '正在提取音频',
    uploading: '正在安全上传音频',
    transcribing: '正在识别语音并翻译',
    translate: '正在翻译现有字幕',
    polish: '正在校对字幕',
    burning: '正在生成字幕视频',
    completed: '处理完成',
  }
  return labels[progress.value.stage] ?? progress.value.message
})
const processingPlan = computed(() => {
  if (!analysis.value) return '等待视频'
  if (analysis.value.strategy === 'extract_chinese') return '直接提取'
  if (settings.value.processingMode === 'gemini') return 'Gemini'
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
    if (settings.value.processingMode === 'gemini') {
      const quality = settings.value.defaultModel === 'gemini-3.5-flash'
      low = durationMinutes * (quality ? 0.35 : 0.25) + 0.5
      high = durationMinutes * (quality ? 0.65 : 0.5) + 1
    } else if (settings.value.whisperRuntime === 'cuda') {
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
  if (needsGemini.value && !settings.value.hasApiKey) return '请先配置 Gemini Key'
  if (needsCustomAi.value && (!settings.value.hasOpenaiApiKey || !settings.value.openaiModel)) return '请先配置自定义 AI'
  if (needsLocalAssets.value && !localModelReady.value) return '请先下载本地模型'
  if (needsLocalAssets.value && !localRuntimeReady.value) return '请先下载运行组件'
  return ''
})

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

async function refreshLocalAssets() {
  const [models, runtimes] = await Promise.all([listWhisperModels(), listWhisperRuntimes()])
  localModelReady.value = Boolean(models.find(item => item.id === settings.value.whisperModel)?.installed)
  localRuntimeReady.value = Boolean(runtimes.find(item => item.id === settings.value.whisperRuntime)?.installed)
}

async function refreshProcessingSettings() {
  settings.value = await getGeminiSettings()
  await refreshLocalAssets()
  if (analysis.value?.strategy === 'transcribe_audio' && settings.value.processingMode === 'gemini') {
    estimate.value = await estimateTranscriptionCost(
      settings.value.defaultModel,
      analysis.value.durationMs,
      true,
    )
  } else {
    estimate.value = null
  }
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

function outputPath(extension: string) {
  return sourcePath.value.replace(/\.[^\\/.]+$/, '') + extension
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

async function chooseVideo(path?: string) {
  const selected = path ?? await selectMediaFile()
  if (!selected) return
  sourcePath.value = selected
  analysis.value = null
  project.value = null
  subtitleOutput.value = ''
  videoOutput.value = ''
  error.value = ''
  notice.value = ''
  analyzing.value = true
  try {
    analysis.value = await analyzeSubtitleSource(selected)
    const previous = (await listSubtitleProjects()).find(item => item.sourcePath === selected)
    if (previous) {
      project.value = previous
      restoreArtifacts(previous)
    }
    if (analysis.value.strategy === 'transcribe_audio' && settings.value.processingMode === 'gemini') {
      estimate.value = await estimateTranscriptionCost(
        settings.value.defaultModel,
        analysis.value.durationMs,
        true,
      )
    } else {
      estimate.value = null
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
  project.value = await createSubtitleProject({
    sourcePath: sourcePath.value,
    durationMs: analysis.value?.durationMs || undefined,
    model: settings.value.processingMode !== 'gemini'
      ? settings.value.whisperModel
      : settings.value.defaultModel,
    targetLanguage: 'zh-CN',
  })
  return project.value
}

async function generateChineseSubtitles() {
  if (!analysis.value) {
    await chooseVideo()
    return
  }
  processing.value = true
  error.value = ''
  notice.value = ''
  try {
    let current = await ensureProject()
    const track = analysis.value.recommendedTrack
    if (track) {
      current = await importSubtitleTrack({
        projectId: current.id,
        streamIndex: track.streamIndex,
        language: track.language,
      })
      if (analysis.value.strategy === 'translate_subtitle') {
        current = settings.value.processingMode === 'local_free'
          ? await startBingTranslation({ projectId: current.id, targetLanguage: 'zh-CN' })
          : settings.value.processingMode === 'local_custom'
            ? await startOpenAiCompatibleTranslation({ projectId: current.id, targetLanguage: 'zh-CN' })
            : await startGeminiTranslation({
              projectId: current.id,
              targetLanguage: 'zh-CN',
              model: settings.value.defaultModel,
            })
      }
    } else {
      if (settings.value.processingMode !== 'gemini') {
        current = await startWhisperTranscription({
          projectId: current.id,
          model: settings.value.whisperModel,
          runtime: settings.value.whisperRuntime,
        })
        current = settings.value.processingMode === 'local_free'
          ? await startBingTranslation({ projectId: current.id, targetLanguage: 'zh-CN' })
          : await startOpenAiCompatibleTranslation({ projectId: current.id, targetLanguage: 'zh-CN' })
      } else {
        current = await startGeminiTranscription({
          projectId: current.id,
          translate: true,
          targetLanguage: 'zh-CN',
          model: settings.value.defaultModel,
        })
      }
    }
    project.value = current
    const content = analysis.value.strategy === 'extract_chinese' ? 'source' : 'translated'
    const srtPath = outputPath('.zh-CN.srt')
    subtitleOutput.value = await exportSubtitles({
      projectId: current.id,
      outputPath: srtPath,
      format: 'srt',
      content,
    })
    if (burnVideo.value) {
      const mp4Path = outputPath('.subtitled.mp4')
      videoOutput.value = await burnSubtitles({ projectId: current.id, outputPath: mp4Path, content })
    }
    project.value = (await listSubtitleProjects()).find(item => item.id === current.id) ?? current
    restoreArtifacts(project.value)
    notice.value = burnVideo.value
      ? '中文字幕和带字幕视频都已生成。'
      : '中文字幕文件已生成。'
    await nextTick()
    resultPanel.value?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
  } catch (value) {
    error.value = errorText(value)
  } finally {
    processing.value = false
  }
}

async function cancel() {
  if (project.value) await cancelSubtitleTask(project.value.id)
}

watch(() => props.initialSourcePath, path => {
  if (path) void chooseVideo(path)
})

onMounted(async () => {
  try {
    await refreshProcessingSettings()
    unlisten = await onSubtitleProgress(event => {
      if (!project.value || event.projectId === project.value.id) progress.value = event
    })
    if (props.initialSourcePath) await chooseVideo(props.initialSourcePath)
  } catch (value) {
    error.value = errorText(value)
  }
})
onActivated(() => {
  void refreshProcessingSettings().catch(value => {
    error.value = errorText(value)
  })
})
onBeforeUnmount(() => unlisten?.())
</script>

<template>
  <section class="simple-subtitle">
    <header class="simple-header">
      <div>
        <h1>转录翻译</h1>
        <p>有字幕直接翻译，没有字幕再识别。</p>
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
            <strong>输出设置</strong>
          </div>
        </div>

        <section class="action-panel">
          <div v-if="analysis" class="route-summary">
            <span>方案</span>
            <strong>{{ processingPlan }}</strong>
            <small>
              {{ processingTimeEstimate }}
              <template v-if="settings.processingMode === 'gemini' && estimate"> · ${{ estimate.estimatedUsdLow.toFixed(3) }}–${{ estimate.estimatedUsdHigh.toFixed(3) }}</template>
            </small>
          </div>
          <div v-else class="route-summary pending">
            <span>当前方案</span>
            <strong>等待选择视频</strong>
            <small>分析完成后会自动选择最省时的处理方式</small>
          </div>

          <label v-if="analysis" class="burn-option">
            <input v-model="burnVideo" type="checkbox" />
            <span><strong>生成字幕视频</strong></span>
          </label>

          <button class="generate-button" type="button" :disabled="analysis ? !canGenerate : analyzing" @click="generateChineseSubtitles">
            <span>{{ actionLabel }}</span>
          </button>
          <p v-if="generateHint" class="action-hint">{{ generateHint }}</p>

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
              <button v-if="videoOutput" class="dark" type="button" @click="openPath(videoOutput)"><i class="icon" :style="iconStyle(mediaVideoIcon)" />播放视频</button>
              <button v-if="subtitleOutput" type="button" @click="openPath(subtitleOutput)"><i class="icon" :style="iconStyle(pageIcon)" />字幕</button>
              <button type="button" @click="openParentFolder(subtitleOutput || videoOutput)"><i class="icon" :style="iconStyle(folderIcon)" />文件夹</button>
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

.burn-option strong {
  color: var(--workspace-ink);
  font-size: 12px;
}

.burn-option small {
  margin-top: 4px;
  color: var(--workspace-subtle);
  font-size: 10px;
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

.settings-button:focus-visible,
.file-card:focus-visible,
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
}

@container workspace (min-width: 900px) {
  .subtitle-layout {
    grid-template-columns: minmax(0, 1.45fr) minmax(280px, 0.75fr);
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
}

@media (prefers-reduced-motion: reduce) {
  .settings-button:hover,
  .output-actions button:hover { transform: none; }
}
</style>
