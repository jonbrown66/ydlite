<script setup lang="ts">
import { computed, nextTick, onActivated, onBeforeUnmount, onMounted, ref } from 'vue'
import cookieIcon from 'iconoir/icons/cookie.svg?url'
import downloadIcon from 'iconoir/icons/download.svg?url'
import folderIcon from 'iconoir/icons/folder.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import navArrowRightIcon from 'iconoir/icons/nav-arrow-right.svg?url'
import pauseIcon from 'iconoir/icons/pause.svg?url'
import playIcon from 'iconoir/icons/play.svg?url'
import settingsIcon from 'iconoir/icons/settings.svg?url'
import xmarkIcon from 'iconoir/icons/xmark.svg?url'
import {
  checkDependencies,
  checkYtdlpUpdate,
  fetchThumbnail,
  getToolsDirectory,
  installMissingTools,
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
import PageHeader from '@/components/ui/PageHeader.vue'
import { usePreferencesStore } from '@/stores/preferences'
import { useDownloadStore } from '@/stores/download'
import DownloadOptionsFields from '@/components/DownloadOptionsFields.vue'
import ProgressBar from '@/components/ui/ProgressBar.vue'
import { formatLabel, readFileOptions } from '@/lib/downloadOptions'
import type { CookieSource, DependencyStatus, ToolInstallEvent, VideoInfo, VideoFormat, DownloadMode } from '../types'

const preferences = usePreferencesStore()
const download = useDownloadStore()
const icons = {
  cookie: cookieIcon,
  download: downloadIcon,
  folder: folderIcon,
  mediaVideo: mediaVideoIcon,
  navArrowRight: navArrowRightIcon,
  pause: pauseIcon,
  play: playIcon,
  settings: settingsIcon,
  xmark: xmarkIcon,
} as const

type IconName = keyof typeof icons

const url = ref('')
const saveDir = ref('')
const defaultDir = ref('')
const cookiesFilePath = ref('')
const deps = ref<DependencyStatus | null>(null)
const video = ref<VideoInfo | null>(null)
const thumbnailFailed = ref(false)
const thumbnailSrc = ref('')
const loadingDeps = ref(false)
const parsing = ref(false)
const advancedOpen = ref(false)
const urlInput = ref<HTMLInputElement | null>(null)
const dragActive = ref(false)
const toolsDir = ref('')
const showInstallConfirm = ref(false)
const installing = ref(false)
const updatingYtdlp = ref(false)
const installEvents = ref<Record<string, ToolInstallEvent>>({})
const completedPath = computed(() => download.state === 'completed' ? download.filePath : '')
let unlistenToolProgress: (() => void) | undefined

const selectedFormatId = ref<string>('best')
const fileOptions = ref(readFileOptions(preferences.settings.fileOptions))
const selectedAudioId = computed({ get: () => fileOptions.value.audioFormatId || '', set: (value: string) => { fileOptions.value.audioFormatId = value || null } })
const audioOptions = computed(() => [{ value: '', label: 'Auto' }, ...(video.value?.formats || []).filter(f => f.vcodec === 'none' && f.acodec && f.acodec !== 'none').map(f => ({ value: f.formatId, label: formatLabel(f) }))])

const selectedEntries = ref<Record<string, boolean>>({})
const selectedCount = computed(() => video.value?.entries?.filter(entry => selectedEntries.value[entry.url]).length ?? 0)

const missingTools = computed(() => {
  const missing: string[] = []
  if (deps.value && !deps.value.ytdlp_ok) missing.push('yt-dlp')
  if (deps.value && !deps.value.ffmpeg_ok) missing.push('ffmpeg')
  return missing
})
const hasMissingTools = computed(() => missingTools.value.length > 0)
const busy = computed(() => download.running || parsing.value || installing.value)

const canAnalyze = computed(() => Boolean(url.value.trim() && !busy.value))
const canDownload = computed(() => Boolean(video.value && saveDir.value && !busy.value && !hasMissingTools.value))

const percent = computed(() => download.percent)
const statusText = computed(() => {
  switch (download.state) {
    case 'running': return download.progress?.status === 'processing' ? 'Processing' : 'Downloading'
    case 'paused': return 'Paused'
    case 'completed': return download.total > 1 ? `Done · ${download.total} files` : 'Done'
    case 'failed': return 'Failed'
    default: return ''
  }
})

const durationText = computed(() => {
  const total = video.value?.duration
  if (!total) return ''
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = Math.floor(total % 60)
  return hours > 0
    ? `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
    : `${minutes}:${seconds.toString().padStart(2, '0')}`
})

const formatOptions = computed(() => {
  const detailed = (video.value?.formats || [])
    .filter(format => format.vcodec && format.vcodec !== 'none')
    .map(format => ({
      value: format.formatId,
      label: [format.height ? `${format.height}p` : format.resolution || 'Video', format.ext.toUpperCase(), format.filesize || format.filesizeApprox ? `${Math.round((format.filesize || format.filesizeApprox)! / 1048576)} MiB` : ''].filter(Boolean).join(' · '),
    }))
  return [
    { value: 'best', label: 'Best' },
    { value: 'p720', label: '720p' },
    { value: 'bestaudio', label: 'Audio · MP3' },
    ...detailed,
  ]
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

const speedText = computed(() => {
  const value = download.speed
  return value && value > 0 ? `${formatBytes(value)}/s` : ''
})

const sizeText = computed(() => {
  const metrics = download.progress?.metrics
  if (metrics?.downloadedBytes != null) {
    return metrics.totalBytes ? `${formatBytes(metrics.downloadedBytes)} / ${formatBytes(metrics.totalBytes)}` : formatBytes(metrics.downloadedBytes)
  }
  if (download.progress?.total) return download.progress.total
  if (video.value?.isPlaylist) return ''
  const audio = video.value?.formats?.filter(f => f.vcodec === 'none' && f.acodec && f.acodec !== 'none') || []
  const candidates = video.value?.formats?.filter(f => f.vcodec && f.vcodec !== 'none' && (selectedFormatId.value !== 'p720' || (f.height || 0) <= 720)) || []
  if (selectedFormatId.value === 'bestaudio') {
    const size = formatSize(audio.at(-1))
    return size ? `≈ ${formatBytes(size)}` : ''
  }
  const chosen = fileOptions.value.quality === 'quality' ? candidates.at(-1) : candidates.filter(f => f.ext === 'mp4').at(-1) || candidates.at(-1)
  const videoSize = formatSize(chosen)
  if (!videoSize) return ''
  if (chosen?.acodec && chosen.acodec !== 'none') return `≈ ${formatBytes(videoSize)}`
  const audioSize = formatSize(audio.filter(f => f.ext === 'm4a').at(-1) || audio.at(-1))
  return audioSize ? `≈ ${formatBytes(videoSize + audioSize)}` : ''
})

function iconStyle(name: IconName) {
  return { '--icon-url': `url("${icons[name]}")` }
}

onMounted(() => {
  defaultDir.value = preferences.settings.defaultDir
  saveDir.value = defaultDir.value
  cookiesFilePath.value = preferences.settings.cookiesFilePath
  void onToolInstallProgress(event => { installEvents.value = { ...installEvents.value, [event.tool]: event } })
    .then(stop => { unlistenToolProgress = stop })
    .catch(() => {})
  void refreshDependencies()
})

onActivated(() => { if (deps.value && !busy.value) void refreshDependencies() })

onBeforeUnmount(() => {
  unlistenToolProgress?.()
})

async function refreshDependencies() {
  loadingDeps.value = true
  try { deps.value = await checkDependencies() }
  catch { /* Missing tool status is shown as unavailable. */ }
  finally { loadingDeps.value = false }
}

async function refreshYtdlpUpdate() {
  try {
    const update = await checkYtdlpUpdate()
    if (deps.value) deps.value = { ...deps.value, ...update }
  } catch { /* Update checks are optional. */ }
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

async function handleAnalyze() {
  if (!canAnalyze.value) return
  video.value = null
  thumbnailSrc.value = ''
  thumbnailFailed.value = false
  download.reset()
  download.error = null
  parsing.value = true
  selectedEntries.value = {}
  try {
    const result = await parseVideo({ url: url.value, options: { cookieSource: selectedCookieSource() } })
    video.value = result
    void loadThumbnail(result.thumbnail)
    if (result.resolvedUrl && result.resolvedUrl !== url.value.trim()) url.value = result.resolvedUrl
    if (result.isPlaylist && result.entries) {
      result.entries.forEach(entry => { selectedEntries.value[entry.url] = true })
    }
    selectedFormatId.value = preferences.settings.defaultQuality === 'mp3' ? 'bestaudio' : preferences.settings.defaultQuality
    fileOptions.value.audioFormatId = null
    await nextTick()
    urlInput.value?.blur()
  } catch (error) {
    download.error = { message: 'Could not analyze this link.', detail: errorText(error) }
  } finally {
    parsing.value = false
  }
}

// Image hosts reject foreign referers, so the bytes are fetched by the backend.
async function loadThumbnail(url: string | null | undefined) {
  thumbnailSrc.value = ''
  thumbnailFailed.value = false
  if (!url) return
  try { thumbnailSrc.value = await fetchThumbnail(url) }
  catch { thumbnailFailed.value = true }
}

async function handleSelectDir() {
  const selected = await selectDirectory()
  if (selected) {
    saveDir.value = selected
    if (saveDir.value !== defaultDir.value) preferences.update({ defaultDir: saveDir.value })
    defaultDir.value = selected
  }
}

function downloadOptions(overwriteConfirmed = false) {
  const selected = selectedFormatId.value
  const custom = !['best', 'p720', 'bestaudio'].includes(selected)
  const mode: DownloadMode = selected === 'bestaudio' ? 'mp3' : selected === 'p720' ? 'p720' : custom ? 'custom' : 'best'
  return { dir: saveDir.value, mode, formatId: custom ? selected : null, cookieSource: selectedCookieSource(), fileOptions: { ...fileOptions.value }, overwriteConfirmed }
}

const showOverwriteConfirm = ref(false)

async function handleDownload() {
  if (!canDownload.value) return
  if (fileOptions.value.conflict === 'overwrite') { showOverwriteConfirm.value = true; return }
  await runDownload()
}

async function runDownload(overwriteConfirmed = false) {
  showOverwriteConfirm.value = false
  if (!canDownload.value) return
  download.error = null
  const targets = video.value?.isPlaylist
    ? (video.value.entries || []).filter(entry => selectedEntries.value[entry.url])
    : [{ url: video.value?.resolvedUrl || url.value, title: video.value?.title }]
  if (!targets.length) return
  await download.start(targets, downloadOptions(overwriteConfirmed))
}

function selectAllEntries(value: boolean) {
  if (busy.value || !video.value?.entries) return
  video.value.entries.forEach(entry => { selectedEntries.value[entry.url] = value })
}

async function handlePause() { await download.pause() }
async function handleCancel() { await download.cancel() }
async function handleResume() { await download.resume() }
async function handleRetry() { download.retryFailed() }

async function handleInstallTools() {
  installing.value = true
  installEvents.value = {}
  try {
    await installMissingTools()
    await refreshDependencies()
    showInstallConfirm.value = false
  } catch { /* The dialog stays open for a retry. */ }
  finally { installing.value = false }
}

async function handleUpdateYtdlp() {
  updatingYtdlp.value = true
  try {
    await updateYtdlp()
    await refreshDependencies()
    await refreshYtdlpUpdate()
  } catch { /* Keep the current status. */ }
  finally { updatingYtdlp.value = false }
}

async function openInstallConfirm() {
  if (!toolsDir.value) {
    try { toolsDir.value = await getToolsDirectory() }
    catch { return }
  }
  showInstallConfirm.value = true
}

async function handleOpenFile() {
  if (!completedPath.value) return
  try { await openPath(completedPath.value) } catch { /* Ignore a missing file. */ }
}

async function handleOpenFolder() {
  if (!completedPath.value) return
  try { await openParentFolder(completedPath.value) } catch { /* Ignore a missing folder. */ }
}

function errorText(value: unknown) {
  const payload = value && typeof value === 'object' ? value as { message?: string; detail?: string } : null
  return payload?.detail || payload?.message || String(value)
}

function handleDragOver(event: DragEvent) {
  event.preventDefault()
  dragActive.value = true
}
function handleDragLeave() { dragActive.value = false }
function handleDrop(event: DragEvent) {
  event.preventDefault()
  dragActive.value = false
  if (busy.value) return
  const text = event.dataTransfer?.getData('text/uri-list') || event.dataTransfer?.getData('text/plain') || ''
  const found = text.match(/https?:\/\/\S+/)
  if (found) url.value = found[0].trim()
}
</script>

<template>
  <section class="download-page" :class="{ dragging: dragActive }" @dragover="handleDragOver" @dragleave="handleDragLeave" @drop="handleDrop">
    <div class="content-scroll">
      <PageHeader title="Download" />

      <div class="input-row-container">
        <div class="input-row">
          <input
            ref="urlInput"
            v-model="url"
            class="url-input"
            type="url"
            aria-label="Video link"
            placeholder="Paste a link"
            :disabled="busy"
            @keydown.enter="handleAnalyze"
          />
          <button
            class="button icon-only"
            type="button"
            :class="{ active: cookiesFilePath }"
            :disabled="busy"
            :title="cookiesFilePath ? 'Cookies added' : 'Add cookies.txt'"
            aria-label="Cookies"
            @click="handleSelectCookiesFile"
          >
            <span class="icon" aria-hidden="true" :style="iconStyle('cookie')" />
          </button>
          <button class="button primary" type="button" :disabled="!canAnalyze" @click="handleAnalyze">
            {{ parsing ? '…' : 'Go' }}
          </button>
        </div>
        <div v-if="cookiesFilePath" class="cookies-inline-status">
          <span :title="cookiesFilePath">cookies.txt</span>
          <button class="link-button compact" type="button" :disabled="busy" @click="clearCookiesFile">Remove</button>
        </div>
      </div>

      <div v-if="deps && (hasMissingTools || deps.ytdlp_update_available)" class="tool-line">
        <span>{{ hasMissingTools ? `Missing ${missingTools.join(', ')}` : 'yt-dlp update available' }}</span>
        <button v-if="hasMissingTools" class="link-button compact" type="button" :disabled="installing" @click="openInstallConfirm">{{ installing ? 'Installing…' : 'Install' }}</button>
        <button v-else class="link-button compact" type="button" :disabled="updatingYtdlp" @click="handleUpdateYtdlp">{{ updatingYtdlp ? 'Updating…' : 'Update' }}</button>
      </div>

      <section v-if="!video && !parsing" class="download-empty" aria-label="Download">
        <span class="empty-symbol"><span class="icon" aria-hidden="true" :style="iconStyle('mediaVideo')" /></span>
        <p>Paste a link to start.</p>
      </section>

      <Transition name="reveal">
        <section v-if="video" class="details">
          <div class="preview">
            <div class="thumb">
              <img v-if="thumbnailSrc && !thumbnailFailed" :src="thumbnailSrc" alt="" @error="thumbnailFailed = true" />
              <span v-else class="thumbnail-fallback"><span class="icon" aria-hidden="true" :style="iconStyle('mediaVideo')" /></span>
            </div>
            <div class="video-meta">
              <h2>{{ video.title }}</h2>
              <p v-if="video.uploader || durationText">{{ [video.uploader, durationText].filter(Boolean).join(' · ') }}</p>
            </div>
          </div>

          <div v-if="video.isPlaylist" class="playlist-panel">
            <div class="playlist-header">
              <span>Selected {{ selectedCount }}/{{ video.entries?.length || 0 }}</span>
              <div class="playlist-actions-quick">
                <button class="link-button compact" type="button" :disabled="busy" @click="selectAllEntries(true)">All</button>
                <button class="link-button compact" type="button" :disabled="busy" @click="selectAllEntries(false)">None</button>
              </div>
            </div>
            <div class="playlist-list-scroll">
              <label v-for="entry in video.entries" :key="entry.url" class="playlist-item" :class="{ active: selectedEntries[entry.url] }">
                <input v-model="selectedEntries[entry.url]" type="checkbox" :disabled="busy" />
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
            <div class="format-select-row">
              <span>Format</span>
              <AppSelect v-model="selectedFormatId" :options="formatOptions" :disabled="busy" aria-label="Format" />
            </div>

            <div class="format-select-row">
              <span>Save to</span>
              <div class="path-row">
                <div class="path-display" :title="saveDir || 'Choose a folder'">{{ saveDir || 'Choose a folder' }}</div>
                <button class="button icon-only" type="button" title="Choose folder" aria-label="Choose folder" :disabled="busy" @click="handleSelectDir">
                  <span class="icon" aria-hidden="true" :style="iconStyle('folder')" />
                </button>
              </div>
            </div>

            <details class="advanced" :open="advancedOpen" @toggle="advancedOpen = ($event.target as HTMLDetailsElement).open">
              <summary>Options</summary>
              <div class="advanced-content">
                <div v-if="audioOptions.length > 1 && !video.isPlaylist" class="format-select-row">
                  <span>Audio</span>
                  <AppSelect v-model="selectedAudioId" :options="audioOptions" :disabled="busy" aria-label="Audio track" />
                </div>
                <DownloadOptionsFields v-model="fileOptions" :disabled="busy" />
              </div>
            </details>

            <div v-if="download.state !== 'idle'" class="progress-panel" :class="{ 'is-finished': download.state === 'completed' }" aria-live="polite">
              <div class="progress-head">
                <span class="progress-status">
                  <i v-if="download.running && download.indeterminate" class="progress-spinner" aria-hidden="true" />
                  {{ statusText }}<template v-if="download.total > 1"> · {{ Math.min(download.current + 1, download.total) }}/{{ download.total }}</template>
                </span>
                <strong v-if="percent !== null">{{ Math.round(percent) }}%</strong>
              </div>
              <ProgressBar
                :percent="percent"
                :state="download.state === 'completed' ? 'completed' : download.state === 'paused' ? 'paused' : 'running'"
                label="Download progress"
              />
              <p v-if="sizeText || speedText" class="progress-size"><span>{{ sizeText }}</span><span v-if="speedText">{{ speedText }}</span></p>
              <p v-if="download.error" class="progress-error">{{ download.error.message }}</p>
            </div>

            <div class="download-actions">
              <template v-if="download.running">
                <button class="button" type="button" @click="handlePause">
                  <span class="icon" aria-hidden="true" :style="iconStyle('pause')" />
                  <span>Pause</span>
                </button>
                <button class="button" type="button" @click="handleCancel">
                  <span class="icon" aria-hidden="true" :style="iconStyle('xmark')" />
                  <span>Cancel</span>
                </button>
              </template>
              <template v-else-if="download.paused">
                <button class="button primary" type="button" @click="handleResume">
                  <span class="icon" aria-hidden="true" :style="iconStyle('play')" />
                  <span>Start</span>
                </button>
                <button class="button" type="button" @click="handleCancel">Cancel</button>
              </template>
              <template v-else>
                <button v-if="download.state === 'failed' && download.failed" class="button" type="button" @click="handleRetry">Retry</button>
                <button v-if="video.isPlaylist" class="button primary" type="button" :disabled="!canDownload || selectedCount === 0" @click="handleDownload">
                  <span class="icon" aria-hidden="true" :style="iconStyle('download')" />
                  <span>{{ selectedCount ? `Download ${selectedCount}` : 'Download' }}</span>
                </button>
                <button v-else class="button primary" type="button" :disabled="!canDownload" @click="handleDownload">
                  <span class="icon" aria-hidden="true" :style="iconStyle('download')" />
                  <span>Download</span>
                </button>
                <button v-if="completedPath" class="button" type="button" @click="handleOpenFolder">
                  <span class="icon" aria-hidden="true" :style="iconStyle('folder')" />
                  <span>Folder</span>
                </button>
                <span v-if="!download.error && !completedPath && sizeText" class="size-hint">{{ sizeText }}</span>
              </template>
            </div>
          </div>
        </section>
      </Transition>

      <div v-if="download.error && !video" class="error-box" role="alert">
        <strong>{{ download.error.message }}</strong>
        <details>
          <summary><span class="icon disclosure-icon" aria-hidden="true" :style="iconStyle('navArrowRight')" /><span>Details</span></summary>
          <div class="details-content"><pre>{{ download.error.detail }}</pre></div>
        </details>
      </div>
    </div>

    <AppDialog :open="showOverwriteConfirm" title="Replace existing files?" @close="showOverwriteConfirm = false">
      <p>Files with the same name are overwritten. This cannot be undone.</p>
      <div class="modal-actions">
        <button class="button" type="button" @click="showOverwriteConfirm = false">Cancel</button>
        <button class="button primary-danger" type="button" @click="runDownload(true)">Replace</button>
      </div>
    </AppDialog>

    <AppDialog :open="showInstallConfirm" :title="installing ? 'Installing…' : `Install ${missingTools.join(', ')}?`" :busy="installing" @close="showInstallConfirm = false">
      <template v-if="!installing">
        <p>Tools are installed to:</p>
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
            <span>{{ installEvents[tool]?.status === 'installed' ? 'Installed' : installEvents[tool]?.status === 'error' ? 'Failed' : installEvents[tool] ? 'Installing…' : 'Waiting…' }}</span>
          </div>
        </div>
      </template>
    </AppDialog>

    <div v-if="dragActive" class="drop-overlay">Drop a link</div>
  </section>
</template>

<style scoped>
.download-page { height: 100%; min-width: 0; }
.input-row-container, .tool-line { width: 100%; max-width: 1040px; margin-inline: auto; }
.tool-line { margin-block-start: 10px; }
.input-row { grid-template-columns: minmax(0, 1fr) var(--control-height) 72px; }
.download-empty { padding-block: 64px; }
.download-empty p { margin: 0; color: var(--muted); }
.tool-line { display: flex; align-items: center; gap: 8px; margin-top: 10px; color: var(--muted); }
.advanced { margin-top: 16px; }
.advanced > summary { cursor: pointer; color: var(--muted); }
.advanced > summary:active { color: var(--ink); }
.advanced-content { display: grid; gap: 14px; padding-top: 14px; }
.advanced-content > .format-select-row { margin-bottom: 0; }
.progress-size { display: flex; gap: 16px; margin: 8px 0 0; color: var(--muted); font-variant-numeric: tabular-nums; }
.progress-error { margin: 8px 0 0; color: var(--danger); }
.size-hint { margin-inline-start: auto; color: var(--muted); }
.button.icon-only.active { color: var(--primary); }
@media (hover: hover) and (pointer: fine) { .advanced > summary:hover { color: var(--ink); } }
</style>
