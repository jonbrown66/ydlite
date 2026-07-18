<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import checkIcon from 'iconoir/icons/check.svg?url'
import trashIcon from 'iconoir/icons/trash.svg?url'
import { Button } from '@/components/ui/button'
import { AppSelect } from '@/components/ui/select'
import {
  clearAppCache,
  deleteWhisperModel,
  downloadWhisperModel,
  downloadWhisperRuntime,
  getGeminiSettings,
  getCacheStatus,
  listWhisperModels,
  listWhisperRuntimes,
  onWhisperDownloadProgress,
  saveGeminiSettings,
  testGeminiConnection,
  testOpenAiCompatibleConnection,
} from '@/api/tauri'
import type { CacheStatus, GeminiSettings, WhisperDownloadEvent, WhisperModelInfo, WhisperRuntimeInfo } from '@/types'

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
const apiKey = ref('')
const openaiApiKey = ref('')
const saving = ref(false)
const testing = ref(false)
const testingCustom = ref(false)
const notice = ref('')
const error = ref('')
const whisperModels = ref<WhisperModelInfo[]>([])
const whisperRuntimes = ref<WhisperRuntimeInfo[]>([])
const downloadProgress = ref<Record<string, WhisperDownloadEvent>>({})
const cacheStatus = ref<CacheStatus | null>(null)
const clearingCache = ref(false)
let unlistenDownload: undefined | (() => void)

const geminiModelOptions = [
  { value: 'gemini-3.1-flash-lite', label: '经济模式' },
  { value: 'gemini-3.5-flash', label: '高质量模式' },
]
const concurrencyOptions = [
  { value: 1, label: '1（稳定）' },
  { value: 2, label: '2（推荐）' },
]

function iconStyle(url: string) {
  return { '--icon-url': `url("${url}")` }
}

function errorText(value: unknown) {
  if (typeof value === 'object' && value && 'message' in value) {
    const payload = value as { message: unknown; detail?: unknown }
    return payload.detail ? `${String(payload.message)}\n${String(payload.detail)}` : String(payload.message)
  }
  return String(value)
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`
}

function formatCleanupTime(value: number) {
  return new Intl.DateTimeFormat('zh-CN', {
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(value * 1000))
}

async function cleanCache() {
  clearingCache.value = true
  error.value = ''
  notice.value = ''
  const preservedEntries = ['ydlite.defaultDir', 'ydlite.cookiesFilePath', 'ydlite.history']
    .map(key => [key, localStorage.getItem(key)] as const)
  try {
    cacheStatus.value = await clearAppCache()
    preservedEntries.forEach(([key, value]) => {
      if (value !== null) localStorage.setItem(key, value)
    })
    const released = cacheStatus.value.records[0]?.releasedBytes ?? 0
    notice.value = `缓存已清理，释放 ${formatBytes(released)}。`
  } catch (value) {
    error.value = errorText(value)
  } finally {
    clearingCache.value = false
  }
}

async function save() {
  saving.value = true
  error.value = ''
  notice.value = ''
  try {
    settings.value = await saveGeminiSettings({
      apiKey: apiKey.value.trim() || undefined,
      defaultModel: settings.value.defaultModel,
      defaultTargetLanguage: 'zh-CN',
      maxCostUsd: settings.value.maxCostUsd,
      maxConcurrency: settings.value.maxConcurrency,
      processingMode: settings.value.processingMode,
      whisperModel: settings.value.whisperModel,
      whisperRuntime: settings.value.whisperRuntime,
      openaiApiKey: openaiApiKey.value.trim() || undefined,
      openaiApiBase: settings.value.openaiApiBase,
      openaiModel: settings.value.openaiModel,
    })
    apiKey.value = ''
    openaiApiKey.value = ''
    notice.value = '设置已保存。'
  } catch (value) {
    error.value = errorText(value)
  } finally {
    saving.value = false
  }
}

async function testCustomConnection() {
  testingCustom.value = true
  error.value = ''
  notice.value = ''
  try {
    notice.value = await testOpenAiCompatibleConnection()
  } catch (value) {
    error.value = errorText(value)
  } finally {
    testingCustom.value = false
  }
}

async function refreshWhisperAssets() {
  [whisperModels.value, whisperRuntimes.value] = await Promise.all([
    listWhisperModels(),
    listWhisperRuntimes(),
  ])
}

function progressFor(type: 'model' | 'runtime', id: string) {
  return downloadProgress.value[`${type}:${id}`]
}

async function installModel(id: string) {
  error.value = ''
  notice.value = ''
  try {
    await downloadWhisperModel(id)
    await refreshWhisperAssets()
    notice.value = '本地 Whisper 模型下载完成。'
  } catch (value) {
    error.value = errorText(value)
  }
}

async function removeModel(id: string) {
  error.value = ''
  try {
    await deleteWhisperModel(id)
    await refreshWhisperAssets()
  } catch (value) {
    error.value = errorText(value)
  }
}

async function installRuntime(id: string) {
  error.value = ''
  notice.value = ''
  try {
    await downloadWhisperRuntime(id)
    await refreshWhisperAssets()
    notice.value = 'Whisper 运行组件安装完成。'
  } catch (value) {
    error.value = errorText(value)
  }
}

async function testConnection() {
  testing.value = true
  error.value = ''
  notice.value = ''
  try {
    notice.value = await testGeminiConnection()
  } catch (value) {
    error.value = errorText(value)
  } finally {
    testing.value = false
  }
}

onMounted(async () => {
  try {
    settings.value = await getGeminiSettings()
    await Promise.all([
      refreshWhisperAssets(),
      getCacheStatus().then(value => { cacheStatus.value = value }),
    ])
    unlistenDownload = await onWhisperDownloadProgress(event => {
      downloadProgress.value = {
        ...downloadProgress.value,
        [`${event.assetType}:${event.id}`]: event,
      }
    })
  } catch (value) {
    error.value = errorText(value)
  }
})
onBeforeUnmount(() => unlistenDownload?.())
</script>

<template>
  <section class="workspace-page settings-page">
    <header class="page-heading">
      <div>
        <h1>设置</h1>
        <p>选择字幕处理方式。模型仅在你主动下载后安装。</p>
      </div>
    </header>

    <div class="settings-sheet">
      <section>
        <div class="setting-intro">
          <h2>默认字幕模式</h2>
          <p>选择默认使用的识别与翻译渠道。</p>
        </div>
        <fieldset class="processing-mode-list">
          <legend>处理方式</legend>
          <label class="processing-mode" :class="{ selected: settings.processingMode === 'local_free' }">
            <input v-model="settings.processingMode" type="radio" value="local_free" />
            <span class="mode-copy">
              <strong>本地免费</strong>
              <small>Whisper 本地识别，必应翻译，不需要 API Key</small>
            </span>
            <span class="mode-badge recommended">推荐</span>
          </label>
          <label class="processing-mode" :class="{ selected: settings.processingMode === 'local_custom' }">
            <input v-model="settings.processingMode" type="radio" value="local_custom" />
            <span class="mode-copy">
              <strong>本地识别 + 自定义 AI</strong>
              <small>Whisper 本地识别，使用你自己的兼容接口翻译</small>
            </span>
            <span class="mode-badge">自定义</span>
          </label>
          <label class="processing-mode" :class="{ selected: settings.processingMode === 'gemini' }">
            <input v-model="settings.processingMode" type="radio" value="gemini" />
            <span class="mode-copy">
              <strong>Gemini 云端</strong>
              <small>使用 Gemini 完成识别与翻译，需要 Gemini Key</small>
            </span>
            <span class="mode-badge">云端</span>
          </label>
        </fieldset>
      </section>

      <section v-if="settings.processingMode === 'local_custom'">
        <div class="setting-intro">
          <h2>OpenAI 兼容接口（可选）</h2>
          <p>用于自定义 AI 翻译，支持 OpenAI、OpenRouter、Groq、硅基流动及兼容服务。</p>
        </div>
        <div class="settings-fields">
          <label class="wide">
            <span>API Base URL</span>
            <input v-model="settings.openaiApiBase" type="url" placeholder="https://api.example.com/v1" />
          </label>
          <label>
            <span>模型名称</span>
            <input v-model="settings.openaiModel" type="text" placeholder="填写服务支持的模型ID" />
          </label>
          <label>
            <span>API Key</span>
            <input v-model="openaiApiKey" type="password" autocomplete="off" :placeholder="settings.hasOpenaiApiKey ? '已安全保存；留空不会修改' : '输入 API Key'" />
          </label>
          <Button variant="outline" :disabled="testingCustom || !settings.hasOpenaiApiKey || !settings.openaiModel" @click="testCustomConnection">
            {{ testingCustom ? '正在测试' : '测试自定义接口' }}
          </Button>
        </div>
      </section>

      <section v-if="settings.processingMode !== 'gemini'">
        <div class="setting-intro">
          <h2>本地 Whisper</h2>
          <p>模型和运行组件均为可选下载。未安装时不会在后台自动下载。</p>
        </div>
        <div class="whisper-assets">
          <div class="asset-grid">
            <article v-for="model in whisperModels" :key="model.id" class="asset-card" :class="{ selected: settings.whisperModel === model.id }">
              <label>
                <input v-model="settings.whisperModel" type="radio" :value="model.id" />
                <span><strong>{{ model.name }}</strong><small>{{ model.downloadSize }}</small></span>
              </label>
              <p>{{ model.description }}</p>
              <div v-if="progressFor('model', model.id)?.status === 'downloading'" class="asset-progress">
                <i :style="{ width: `${progressFor('model', model.id)?.percent || 0}%` }" />
                <span>{{ Math.round(progressFor('model', model.id)?.percent || 0) }}%</span>
              </div>
              <div class="asset-actions">
                <span :class="{ installed: model.installed }">{{ model.installed ? '已安装' : '未下载' }}</span>
                <Button v-if="!model.installed" size="sm" variant="outline" :disabled="['downloading', 'extracting'].includes(progressFor('model', model.id)?.status || '')" @click="installModel(model.id)">
                  {{ progressFor('model', model.id)?.status === 'downloading' ? '下载中' : `下载 ${model.downloadSize}` }}
                </Button>
                <Button v-else size="sm" variant="ghost" @click="removeModel(model.id)">删除</Button>
              </div>
            </article>
          </div>

          <div class="runtime-grid">
            <article v-for="runtime in whisperRuntimes" :key="runtime.id" class="runtime-row">
              <label>
                <input v-model="settings.whisperRuntime" type="radio" :value="runtime.id" />
                <span><strong>{{ runtime.name }}</strong><small>{{ runtime.description }} · {{ runtime.downloadSize }}</small></span>
              </label>
              <div v-if="progressFor('runtime', runtime.id)?.status === 'downloading'" class="asset-progress compact">
                <i :style="{ width: `${progressFor('runtime', runtime.id)?.percent || 0}%` }" />
              </div>
              <span v-if="runtime.installed" class="installed">已安装</span>
              <Button v-else size="sm" variant="outline" :disabled="['downloading', 'extracting'].includes(progressFor('runtime', runtime.id)?.status || '')" @click="installRuntime(runtime.id)">
                {{ progressFor('runtime', runtime.id)?.status === 'extracting' ? '安装中' : progressFor('runtime', runtime.id)?.status === 'downloading' ? `${Math.round(progressFor('runtime', runtime.id)?.percent || 0)}%` : `下载 ${runtime.downloadSize}` }}
              </Button>
            </article>
          </div>
        </div>
      </section>

      <section v-if="settings.processingMode === 'gemini'">
        <div class="setting-intro">
          <h2>Gemini（可选）</h2>
          <p>只在选择 Gemini 云端模式时使用，Key 会安全保存在 Windows 凭据管理器。</p>
        </div>
        <div class="settings-fields">
          <label class="wide">
            <span>Gemini Auth Key</span>
            <input v-model="apiKey" type="password" autocomplete="off" :placeholder="settings.hasApiKey ? '已安全保存；留空不会修改' : '输入 Gemini Auth Key'" />
          </label>
          <label>
            <span>默认处理模式</span>
            <AppSelect
              v-model="settings.defaultModel"
              :options="geminiModelOptions"
              aria-label="默认处理模式"
            />
          </label>
          <label>
            <span>单任务费用上限（美元）</span>
            <input v-model.number="settings.maxCostUsd" type="number" min="0" step="0.1" />
          </label>
          <label>
            <span>并发数</span>
            <AppSelect
              v-model="settings.maxConcurrency"
              :options="concurrencyOptions"
              aria-label="并发数"
            />
          </label>
        </div>
      </section>

      <section>
        <div class="setting-intro">
          <h2>存储</h2>
          <p>清理临时音频和界面缓存。模型、项目和输出文件会保留。</p>
        </div>
        <div class="cache-manager">
          <div class="cache-summary">
            <div>
              <span>可清理缓存</span>
              <strong>{{ cacheStatus ? formatBytes(cacheStatus.cacheBytes) : '计算中…' }}</strong>
            </div>
            <Button variant="outline" :disabled="clearingCache || !cacheStatus" @click="cleanCache">
              <i class="icon" :style="iconStyle(trashIcon)" />
              {{ clearingCache ? '清理中' : '清理缓存' }}
            </Button>
          </div>
          <p v-if="cacheStatus" class="preserved-size">
            已保留：模型 {{ formatBytes(cacheStatus.modelBytes) }} · 项目 {{ formatBytes(cacheStatus.projectBytes) }}
          </p>
          <div v-if="cacheStatus?.records.length" class="cleanup-history">
            <span>清理记录</span>
            <ol>
              <li v-for="record in cacheStatus.records.slice(0, 3)" :key="record.id">
                <time>{{ formatCleanupTime(record.cleanedAt) }}</time>
                <strong>{{ formatBytes(record.releasedBytes) }}</strong>
              </li>
            </ol>
          </div>
          <p v-else class="empty-history">暂无清理记录</p>
        </div>
      </section>

      <footer class="settings-actions">
        <div v-if="settings.processingMode === 'gemini'" class="key-status">
          <i :class="{ active: settings.hasApiKey }" />
          {{ settings.hasApiKey ? '已保存 API Key' : '尚未保存 API Key' }}
        </div>
        <div v-else-if="settings.processingMode === 'local_custom'" class="key-status">
          <i :class="{ active: settings.hasOpenaiApiKey }" />
          {{ settings.hasOpenaiApiKey ? '已保存自定义 API Key' : '尚未保存自定义 API Key' }}
        </div>
        <div v-else class="key-status">
          <i class="active" />
          本地免费模式
        </div>
        <Button v-if="settings.processingMode === 'gemini'" variant="outline" :disabled="testing || !settings.hasApiKey" @click="testConnection">
          {{ testing ? '正在测试' : '测试连接' }}
        </Button>
        <Button :disabled="saving" @click="save">
          <i class="icon" :style="iconStyle(checkIcon)" />
          {{ saving ? '正在保存' : '保存设置' }}
        </Button>
      </footer>
    </div>

    <p v-if="error" class="inline-alert error">{{ error }}</p>
    <p v-if="notice" class="inline-alert success">{{ notice }}</p>
  </section>
</template>

<style scoped>
.whisper-assets {
  min-width: 0;
}

.processing-mode-list {
  min-width: 0;
  margin: 0;
  padding: 0;
  overflow: hidden;
  border: 1px solid var(--workspace-border);
  border-radius: 10px;
  background: var(--workspace-surface);
}

.processing-mode-list legend {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
}

.processing-mode {
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
  min-height: 66px;
  padding: 11px 14px;
  border-bottom: 1px solid var(--workspace-border);
  cursor: pointer;
  transition: background-color 140ms ease-out;
}

.processing-mode:last-child {
  border-bottom: 0;
}

.processing-mode:hover {
  background: var(--workspace-surface-muted);
}

.processing-mode.selected {
  background: var(--workspace-accent-soft);
}

.processing-mode input {
  width: 16px;
  height: 16px;
  margin: 0;
  accent-color: var(--workspace-accent);
}

.mode-copy {
  display: grid;
  min-width: 0;
  gap: 3px;
}

.mode-copy strong {
  color: var(--workspace-ink);
  font-size: 13px;
  font-weight: 700;
}

.mode-copy small {
  color: var(--workspace-muted);
  font-size: 11px;
  line-height: 1.45;
}

.processing-mode.selected .mode-copy small {
  color: color-mix(in oklch, var(--workspace-accent) 55%, var(--workspace-ink));
}

.mode-badge {
  min-width: 46px;
  padding: 4px 7px;
  border-radius: 5px;
  background: var(--workspace-surface-muted);
  color: var(--workspace-muted);
  font-size: 10px;
  font-weight: 700;
  text-align: center;
}

.mode-badge.recommended {
  background: color-mix(in oklch, var(--workspace-accent-soft) 72%, var(--workspace-surface));
  color: var(--workspace-accent);
}

.settings-sheet {
  overflow: visible;
  border: 0;
}

.settings-sheet > section {
  grid-template-columns: 190px minmax(0, 1fr);
  gap: 32px;
  padding: 24px 0;
  border-top: 1px solid var(--workspace-border);
}

.settings-sheet > section:first-child {
  padding-top: 0;
  border-top: 0;
}

.setting-intro {
  position: sticky;
  top: 0;
  align-self: start;
  padding-top: 4px;
}

.setting-intro h2 {
  margin: 0 0 8px;
  color: var(--workspace-ink);
  font-size: 15px;
  font-weight: 700;
}

.setting-intro p {
  color: var(--workspace-muted);
  font-size: 12px;
  line-height: 1.65;
}

.settings-fields {
  gap: 18px 14px;
}

.settings-fields label {
  gap: 8px;
  color: var(--workspace-muted);
  font-size: 11px;
  font-weight: 600;
}

.settings-fields input,
.settings-fields select {
  height: 44px;
  padding: 0 13px;
  border-color: var(--workspace-border);
  border-radius: 9px;
  background: var(--workspace-surface);
  color: var(--workspace-ink);
  font-size: 13px;
  transition: border-color 150ms ease-out, box-shadow 150ms ease-out, background-color 150ms ease-out;
}

.settings-fields input:hover,
.settings-fields select:hover {
  border-color: var(--workspace-border-strong);
}

.settings-fields input:focus,
.settings-fields select:focus {
  border-color: var(--workspace-accent);
  outline: 0;
  box-shadow: 0 0 0 3px color-mix(in oklch, var(--workspace-accent) 14%, transparent);
}

.asset-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 0;
  overflow: hidden;
  border: 1px solid var(--workspace-border);
  border-radius: 10px;
}

.asset-card {
  display: grid;
  grid-template-columns: minmax(190px, 0.8fr) minmax(220px, 1.2fr) auto;
  align-items: center;
  gap: 14px;
  min-width: 0;
  min-height: 72px;
  padding: 12px 14px;
  border: 0;
  border-bottom: 1px solid var(--workspace-border);
  border-radius: 0;
  background: var(--workspace-surface);
  transition: background-color 140ms ease-out;
}

.asset-card:last-child {
  border-bottom: 0;
}

.asset-card.selected {
  background: var(--workspace-accent-soft);
  box-shadow: inset 3px 0 var(--workspace-accent);
}

.asset-card label,
.runtime-row label {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  cursor: pointer;
}

.asset-card input,
.runtime-row input {
  width: 16px;
  height: 16px;
  margin-top: 2px;
  accent-color: var(--workspace-accent);
}

.asset-card label span,
.runtime-row label span {
  display: grid;
  min-width: 0;
  gap: 4px;
}

.asset-card strong,
.runtime-row strong {
  color: var(--workspace-ink);
  font-size: 13px;
}

.asset-card small,
.runtime-row small {
  color: var(--workspace-subtle);
  font-size: 11px;
  line-height: 1.45;
}

.asset-card p {
  min-height: 0;
  margin: 0;
  color: var(--workspace-muted);
  font-size: 11px;
  line-height: 1.6;
}

.asset-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  margin-left: 0;
}

.asset-actions > span,
.runtime-row > span {
  color: var(--workspace-subtle);
  font-size: 11px;
}

.installed {
  color: var(--workspace-success) !important;
  font-weight: 700;
}

.asset-progress {
  position: relative;
  height: 5px;
  grid-column: 1 / -1;
  margin: -5px 0 3px;
  border-radius: 99px;
  background: var(--workspace-border);
}

.asset-progress i {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--workspace-accent);
  transition: width 300ms var(--workspace-ease);
}

.asset-progress span {
  position: absolute;
  right: 0;
  top: -18px;
  color: var(--workspace-muted);
  font-size: 10px;
}

.runtime-grid {
  display: grid;
  gap: 0;
  margin-top: 18px;
  overflow: hidden;
  border: 1px solid var(--workspace-border);
  border-radius: 10px;
  background: color-mix(in oklch, var(--workspace-surface) 72%, transparent);
}

.runtime-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 120px auto;
  align-items: center;
  gap: 14px;
  min-height: 58px;
  padding: 11px 14px;
  border-bottom: 1px solid var(--workspace-border);
  transition: background-color 140ms ease-out;
}

.runtime-row:last-child {
  border-bottom: 0;
}

.runtime-row:hover {
  background: var(--workspace-surface);
}

.asset-progress.compact {
  margin: 0;
}

.settings-actions {
  position: static;
  gap: 10px;
  margin-top: 8px;
  padding: 14px 16px;
  border: 1px solid var(--workspace-border);
  border-radius: 10px;
  background: var(--workspace-surface);
  box-shadow: none;
}

.cache-manager {
  min-width: 0;
  padding: 2px 0;
}

.cache-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  min-height: 50px;
}

.cache-summary > div {
  display: grid;
  gap: 4px;
}

.cache-summary span,
.cleanup-history > span {
  color: var(--workspace-muted);
  font-size: 11px;
  font-weight: 600;
}

.cache-summary strong {
  color: var(--workspace-ink);
  font-size: 20px;
  font-weight: 700;
  letter-spacing: -0.02em;
}

.preserved-size,
.empty-history {
  margin: 9px 0 0;
  color: var(--workspace-subtle);
  font-size: 11px;
}

.cleanup-history {
  display: grid;
  grid-template-columns: 88px minmax(0, 1fr);
  gap: 14px;
  margin-top: 20px;
  padding-top: 14px;
  border-top: 1px solid var(--workspace-border);
}

.cleanup-history ol {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.cleanup-history li {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  color: var(--workspace-muted);
  font-size: 11px;
}

.cleanup-history li strong {
  color: var(--workspace-ink);
  font-weight: 650;
}

.key-status {
  font-size: 11px;
}

.key-status i {
  width: 8px;
  height: 8px;
}

@media (max-width: 1060px) {
  .settings-sheet > section {
    grid-template-columns: 180px minmax(0, 1fr);
    gap: 30px;
  }

  .asset-card {
    grid-template-columns: minmax(180px, 0.8fr) minmax(180px, 1fr);
  }

  .asset-actions {
    grid-column: 1 / -1;
    justify-content: flex-end;
  }
}

@media (max-width: 900px) {
  .settings-sheet > section {
    grid-template-columns: 1fr;
    gap: 18px;
  }

  .setting-intro {
    position: static;
  }

  .asset-grid {
    grid-template-columns: 1fr;
  }

  .asset-card {
    grid-template-columns: 1fr;
  }

  .asset-actions {
    grid-column: auto;
    justify-content: space-between;
  }

  .runtime-row {
    grid-template-columns: 1fr auto;
  }

  .runtime-row .asset-progress {
    grid-column: 1 / -1;
  }
}

@container workspace (max-width: 780px) {
  .settings-sheet > section {
    grid-template-columns: 1fr;
    gap: 16px;
  }

  .setting-intro {
    position: static;
  }

  .asset-card {
    grid-template-columns: 1fr;
  }

  .asset-actions {
    grid-column: auto;
    justify-content: space-between;
  }

  .runtime-row {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .runtime-row .asset-progress {
    grid-column: 1 / -1;
  }

  .settings-actions {
    flex-wrap: wrap;
  }

  .key-status {
    flex: 1 0 100%;
    margin-right: 0;
  }
}

@container workspace (max-width: 520px) {
  .processing-mode {
    grid-template-columns: 18px minmax(0, 1fr);
  }

  .mode-badge {
    grid-column: 2;
    justify-self: start;
  }

  .runtime-row {
    grid-template-columns: 1fr;
  }

  .runtime-row > .installed,
  .runtime-row > button {
    justify-self: start;
  }

  .settings-actions > button {
    flex: 1 1 auto;
  }
}
</style>
