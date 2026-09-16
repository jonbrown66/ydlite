<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave, useRoute } from 'vue-router'
import checkIcon from 'iconoir/icons/check.svg?url'
import eyeClosedIcon from 'iconoir/icons/eye-closed.svg?url'
import eyeIcon from 'iconoir/icons/eye.svg?url'
import trashIcon from 'iconoir/icons/trash.svg?url'
import AppDialog from '@/components/ui/AppDialog.vue'
import { registerCloseGuard } from '@/lib/closeGuards'
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
  testOpenAiCompatibleConnection,
} from '@/api/tauri'
import type { CacheStatus, GeminiSettings, WhisperDownloadEvent, WhisperModelInfo, WhisperRuntimeInfo } from '@/types'

type ProcessingMode = GeminiSettings['processingMode']
type ProviderPreset = {
  id: string
  label: string
  apiBase?: string
  website?: string
  models?: string[]
}

const route = useRoute()
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
const openaiApiKey = ref('')
const showOpenaiApiKey = ref(false)
const saving = ref(false)
const testingCustom = ref(false)
const notice = ref('')
const error = ref('')
const customModels = ref<string[]>([])
const selectedProviderId = ref('openai')
const customModelsStatus = ref('选择服务商后会直接显示该服务商的预置模型。')
const whisperModels = ref<WhisperModelInfo[]>([])
const whisperRuntimes = ref<WhisperRuntimeInfo[]>([])
const downloadProgress = ref<Record<string, WhisperDownloadEvent>>({})
const cacheStatus = ref<CacheStatus | null>(null)
const clearingCache = ref(false)
const localAssetsSection = ref<HTMLElement | null>(null)
const providerSetupSection = ref<HTMLElement | null>(null)
const savedDraft = ref('')
let savedSettings: GeminiSettings | null = null
let unregisterCloseGuard: (() => void) | undefined
let pendingLeave: Promise<boolean> | undefined
const showUnsavedConfirm = ref(false)
let unlistenDownload: undefined | (() => void)
let resolveUnsavedConfirm: ((allow: boolean) => void) | undefined

const providerPresets: ProviderPreset[] = [
  {
    id: 'openai',
    label: 'OpenAI / ChatGPT · https://api.openai.com/v1',
    apiBase: 'https://api.openai.com/v1',
    website: 'https://platform.openai.com/api-keys',
    models: ['gpt-5.6-sol', 'gpt-5.6-terra', 'gpt-5.6-luna'],
  },
  {
    id: 'gemini',
    label: 'Gemini · https://generativelanguage.googleapis.com/v1beta/openai',
    apiBase: 'https://generativelanguage.googleapis.com/v1beta/openai',
    website: 'https://aistudio.google.com/apikey',
    models: ['gemini-3.7-flash', 'gemini-3.6-flash', 'gemini-3.5-flash-lite'],
  },
  {
    id: 'kimi',
    label: 'Kimi Code · https://api.kimi.com/coding/v1',
    apiBase: 'https://api.kimi.com/coding/v1',
    website: 'https://www.kimi.com/code/',
    models: ['k3', 'k3-256k', 'kimi-for-coding', 'kimi-for-coding-highspeed'],
  },
  {
    id: 'glm',
    label: 'GLM · https://api.z.ai/api/paas/v4',
    apiBase: 'https://api.z.ai/api/paas/v4',
    website: 'https://open.bigmodel.cn/usercenter/apikeys',
    models: ['glm-5.3', 'glm-5.1', 'glm-5-turbo', 'glm-5'],
  },
  {
    id: 'deepseek',
    label: 'DeepSeek · https://api.deepseek.com/v1',
    apiBase: 'https://api.deepseek.com/v1',
    website: 'https://platform.deepseek.com/api_keys',
    models: ['deepseek-v4-flash', 'deepseek-v4-pro', 'deepseek-v4-flash-vision-exp'],
  },
  { id: 'custom', label: '自定义 OpenAI 兼容接口' },
]

const processingModes: Array<{
  value: ProcessingMode
  title: string
  badge: string
}> = [
  {
    value: 'local_free',
    title: '本地免费',
    badge: '无需 Key',
  },
  {
    value: 'local_custom',
    title: '自定义接口',
    badge: '高级',
  },
]

const hasCustomKey = computed(() => Boolean(settings.value.hasOpenaiApiKey || openaiApiKey.value.trim()))
const canTestCustom = computed(() => Boolean(hasCustomKey.value && settings.value.openaiModel.trim()))
const providerOptions = computed(() => providerPresets.map(({ id, label }) => ({ value: id, label })))
const selectedProvider = computed(() =>
  providerPresets.find(provider => provider.id === selectedProviderId.value) ?? providerPresets[0],
)
const customModelOptions = computed(() => {
  const models = [...customModels.value]
  if (settings.value.openaiModel && !models.includes(settings.value.openaiModel)) {
    models.unshift(settings.value.openaiModel)
  }
  if (models.length) return models.map(model => ({ value: model, label: model }))
  return [{ value: '', label: customModelsStatus.value || '等待自动加载模型' }]
})

function draftSignature() {
  return JSON.stringify({
    processingMode: settings.value.processingMode,
    whisperModel: settings.value.whisperModel,
    whisperRuntime: settings.value.whisperRuntime,
    openaiApiBase: settings.value.openaiApiBase,
    openaiModel: settings.value.openaiModel,
  })
}

const hasUnsavedChanges = computed(() =>
  Boolean(savedDraft.value) && (
    savedDraft.value !== draftSignature()
    || Boolean(openaiApiKey.value.trim())
  ),
)

function markSaved() {
  savedDraft.value = draftSignature()
  savedSettings = structuredClone({ ...settings.value })
}

function requestUnsavedConfirm() {
  if (!hasUnsavedChanges.value) return Promise.resolve(true)
  if (pendingLeave) return pendingLeave
  showUnsavedConfirm.value = true
  pendingLeave = new Promise<boolean>((resolve) => {
    resolveUnsavedConfirm = resolve
  })
  return pendingLeave
}

function settleUnsavedConfirm(allow: boolean) {
  showUnsavedConfirm.value = false
  const resolve = resolveUnsavedConfirm
  resolveUnsavedConfirm = undefined
  pendingLeave = undefined
  resolve?.(allow)
}

function discardAndContinue() {
  if (savedSettings) settings.value = structuredClone(savedSettings)
  openaiApiKey.value = ''
  showOpenaiApiKey.value = false
  syncSelectedProvider()
  customModels.value = selectedProvider.value.models ?? []
  settleUnsavedConfirm(true)
}

async function saveAndContinue() {
  if (await persistSettings(true)) settleUnsavedConfirm(true)
}
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

async function persistSettings(showNotice = true) {
  saving.value = true
  error.value = ''
  if (showNotice) notice.value = ''
  try {
    settings.value = await saveGeminiSettings({
      // 保留旧字段的当前值以兼容已有项目；设置页不再管理专属 Gemini/GLM 凭据。
      apiKey: undefined,
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
      glmApiKey: undefined,
    })
    openaiApiKey.value = ''
    markSaved()
    if (showNotice) notice.value = '设置已保存。'
    return true
  } catch (value) {
    error.value = errorText(value)
    return false
  } finally {
    saving.value = false
  }
}

async function save() {
  await persistSettings(true)
}

async function testCustomConnection() {
  if (!await persistSettings(false)) return
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

function normalizeApiBase(apiBase: string) {
  return apiBase.trim().replace(/\/+$/, '')
}

function syncSelectedProvider() {
  const normalizedBase = normalizeApiBase(settings.value.openaiApiBase)
  const preset = providerPresets.find(provider =>
    provider.apiBase && normalizeApiBase(provider.apiBase) === normalizedBase,
  )
  selectedProviderId.value = preset?.id ?? 'custom'
}

function selectProvider(value: string | number) {
  const provider = providerPresets.find(item => item.id === String(value))
  if (!provider) return
  selectedProviderId.value = provider.id
  if (provider.apiBase) settings.value.openaiApiBase = provider.apiBase
  settings.value.openaiModel = ''
  customModels.value = provider.models ?? []
  customModelsStatus.value = provider.models?.length
    ? `已准备 ${provider.models.length} 个 ${provider.label.split(' · ')[0]} 模型。`
    : '自定义接口请在保存 Key 后测试连接。'
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

function requestedSetup() {
  const setup = route.query.setup
  return typeof setup === 'string' && ['custom', 'local'].includes(setup) ? setup : ''
}

async function applyRequestedSetup(scroll = true) {
  const setup = requestedSetup()
  const modeBySetup: Record<string, ProcessingMode> = {
    custom: 'local_custom',
    local: 'local_free',
  }
  if (setup && modeBySetup[setup]) settings.value.processingMode = modeBySetup[setup]
  if (!scroll || !setup) return
  await nextTick()
  const target = setup === 'local' ? localAssetsSection.value : providerSetupSection.value
  target?.scrollIntoView({ behavior: 'smooth', block: 'center' })
}

onMounted(async () => {
  unregisterCloseGuard = registerCloseGuard(requestUnsavedConfirm)
  try {
    settings.value = await getGeminiSettings()
    if (!processingModes.some(mode => mode.value === settings.value.processingMode)) {
      settings.value.processingMode = 'local_free'
    }
    syncSelectedProvider()
    customModels.value = selectedProvider.value.models ?? []
    if (customModels.value.length) {
      customModelsStatus.value = `已准备 ${customModels.value.length} 个 ${selectedProvider.value.label.split(' · ')[0]} 模型。`
    }
    await applyRequestedSetup(false)
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
    await applyRequestedSetup(Boolean(requestedSetup()))
    markSaved()
  } catch (value) {
    error.value = errorText(value)
  }
})
onBeforeUnmount(() => {
  unlistenDownload?.()
  unregisterCloseGuard?.()
  settleUnsavedConfirm(false)
})

onBeforeRouteLeave(async () => requestUnsavedConfirm())

watch(() => route.query.setup, () => {
  void applyRequestedSetup()
})

watch(() => settings.value.openaiApiBase, () => {
  syncSelectedProvider()
  customModels.value = selectedProvider.value.models ?? []
  customModelsStatus.value = customModels.value.length
    ? `已准备 ${customModels.value.length} 个 ${selectedProvider.value.label.split(' · ')[0]} 模型。`
    : '自定义接口请在保存 Key 后测试连接。'
})
</script>

<template>
  <section class="workspace-page settings-page">
    <header class="page-heading">
      <h1>设置</h1>
    </header>

    <div class="settings-sheet">
      <section>
        <fieldset class="processing-mode-list">
          <legend>处理方式</legend>
          <label
            v-for="mode in processingModes"
            :key="mode.value"
            class="processing-mode"
            :class="{ selected: settings.processingMode === mode.value }"
          >
            <input v-model="settings.processingMode" type="radio" :value="mode.value" />
            <span class="mode-copy">
              <span class="mode-title-line">
                <strong>{{ mode.title }}</strong>
              </span>
            </span>
            <span class="mode-badge">{{ mode.badge }}</span>
          </label>
        </fieldset>
      </section>

      <section v-if="settings.processingMode === 'local_custom'" ref="providerSetupSection">
        <div class="settings-fields">
          <label class="wide">
            <span>服务商 / API Base URL</span>
            <AppSelect
              :model-value="selectedProviderId"
              :options="providerOptions"
              aria-label="服务商和 API Base URL"
              @update:model-value="selectProvider"
            />
            <small v-if="selectedProvider.website" class="field-hint">
              <a :href="selectedProvider.website" target="_blank" rel="noreferrer">打开 {{ selectedProvider.label.split(' · ')[0] }} 官方 API 平台</a>
            </small>
          </label>
          <label v-if="selectedProviderId === 'custom'" class="wide">
            <span>自定义 API Base URL</span>
            <input v-model="settings.openaiApiBase" type="url" placeholder="https://api.example.com/v1" />
          </label>
          <label class="wide">
            <span>模型</span>
            <AppSelect
              v-model="settings.openaiModel"
              :options="customModelOptions"
              :disabled="!customModels.length && !settings.openaiModel"
              aria-label="可选模型"
            />
            <small class="field-hint">{{ customModelsStatus }}</small>
          </label>
          <label class="wide">
            <span>API Key</span>
            <span class="key-input">
              <input v-model="openaiApiKey" :type="showOpenaiApiKey ? 'text' : 'password'" autocomplete="off" :placeholder="settings.hasOpenaiApiKey ? '已安全保存；留空不会修改' : '输入 API Key'" />
              <button
                type="button"
                class="key-visibility"
                :aria-label="showOpenaiApiKey ? '隐藏 API Key' : '显示 API Key'"
                :title="showOpenaiApiKey ? '隐藏 API Key' : '显示 API Key'"
                @click="showOpenaiApiKey = !showOpenaiApiKey"
              >
                <i class="icon" :style="iconStyle(showOpenaiApiKey ? eyeClosedIcon : eyeIcon)" />
              </button>
            </span>
          </label>
          <Button variant="outline" :disabled="saving || testingCustom || !canTestCustom" @click="testCustomConnection">
            {{ testingCustom ? '正在测试' : '保存并测试连接' }}
          </Button>
        </div>
      </section>

      <section ref="localAssetsSection">
        <div class="whisper-assets">
          <div class="asset-section-heading">
            <div>
              <strong>识别模型</strong>
            </div>
          </div>
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

          <div class="asset-section-heading runtime-heading">
            <div>
              <strong>运行组件</strong>
            </div>
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

      <section>
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
        <span class="save-state" role="status">{{ saving ? '正在保存…' : hasUnsavedChanges ? '有未保存的修改' : '设置已保存' }}</span>
        <Button :disabled="saving || !hasUnsavedChanges" @click="save">
          <i class="icon" :style="iconStyle(checkIcon)" />
          {{ saving ? '正在保存' : '保存设置' }}
        </Button>
      </footer>
    </div>

    <p v-if="error" class="inline-alert error" role="alert">{{ error }}</p>
    <p v-if="notice" class="inline-alert success" role="status">{{ notice }}</p>

    <AppDialog :open="showUnsavedConfirm" title="保存修改？" :busy="saving" @close="settleUnsavedConfirm(false)">
      <p>离开前的修改尚未保存。</p>
      <div class="modal-actions">
        <Button variant="ghost" :disabled="saving" @click="settleUnsavedConfirm(false)">继续编辑</Button>
        <Button variant="outline" :disabled="saving" @click="discardAndContinue">放弃修改</Button>
        <Button :disabled="saving" @click="saveAndContinue">{{ saving ? '正在保存' : '保存并离开' }}</Button>
      </div>
    </AppDialog>
  </section>
</template>

<style scoped>
.settings-setup-overview {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(260px, 0.8fr);
  align-items: center;
  gap: 24px;
  width: 100%;
  max-width: 960px;
  margin: 0 auto 28px;
  padding: 18px 20px;
  border: 1px solid var(--workspace-border);
  border-radius: 12px;
  background: var(--workspace-surface);
}

.settings-setup-overview > div:first-child > span,
.settings-setup-overview strong,
.settings-setup-overview p {
  display: block;
}

.settings-setup-overview > div:first-child > span {
  color: var(--workspace-subtle);
  font-size: 10px;
  font-weight: 700;
}

.settings-setup-overview > div:first-child > strong {
  margin-top: 4px;
  color: var(--workspace-ink);
  font-size: 16px;
  letter-spacing: -0.01em;
}

.settings-setup-overview p {
  margin: 5px 0 0;
  color: var(--workspace-muted);
  font-size: 11px;
  line-height: 1.55;
}

.setup-status {
  display: grid;
  grid-template-columns: 9px minmax(0, 1fr);
  align-items: start;
  gap: 9px;
  padding: 12px 13px;
  border-radius: 9px;
  background: var(--workspace-surface-muted);
}

.setup-status > i {
  width: 8px;
  height: 8px;
  margin-top: 4px;
  border-radius: 50%;
  background: var(--warning);
}

.setup-status.ready {
  background: var(--success-soft);
}

.setup-status.ready > i {
  background: var(--workspace-success);
}

.setup-status strong,
.setup-status small {
  display: block;
}

.setup-status strong {
  color: var(--workspace-ink);
  font-size: 11px;
}

.setup-status small {
  margin-top: 3px;
  color: var(--workspace-muted);
  font-size: 10px;
  line-height: 1.45;
}

.whisper-assets {
  min-width: 0;
}

.processing-mode-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  min-width: 0;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  gap: 10px;
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
  grid-template-columns: 18px minmax(0, 1fr);
  align-items: start;
  gap: 12px;
  min-height: 64px;
  padding: 12px 14px;
  border: 1px solid var(--workspace-border);
  border-radius: 10px;
  background: var(--workspace-surface);
  cursor: pointer;
  transition: border-color 140ms ease-out, background-color 140ms ease-out, transform 140ms var(--workspace-ease);
}

.processing-mode:hover {
  border-color: var(--workspace-border-strong);
  background: var(--workspace-surface-muted);
  transform: translateY(-1px);
}

.processing-mode.selected {
  border-color: var(--workspace-border);
  background: var(--workspace-surface);
  box-shadow: none;
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
  gap: 5px;
}

.mode-title-line {
  display: flex;
  align-items: center;
  gap: 7px;
}

.mode-copy strong {
  color: var(--workspace-ink);
  font-size: 13px;
  font-weight: 700;
}

.processing-mode.selected .mode-copy strong {
  color: var(--workspace-accent);
}

.mode-copy small {
  color: var(--workspace-muted);
  font-size: 11px;
  line-height: 1.45;
}

.mode-copy em {
  overflow: hidden;
  color: var(--workspace-subtle);
  font-size: 10px;
  font-style: normal;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.processing-mode.selected .mode-copy small {
  color: color-mix(in oklch, var(--workspace-accent) 55%, var(--workspace-ink));
}

.mode-badge {
  grid-column: 2;
  justify-self: start;
  margin-top: 0;
  padding: 3px 6px;
  border-radius: 5px;
  background: var(--workspace-surface-muted);
  color: var(--workspace-muted);
  font-size: 10px;
  font-weight: 700;
  text-align: center;
}

.mode-recommended {
  padding: 2px 5px;
  border-radius: 4px;
  background: color-mix(in oklch, var(--workspace-accent-soft) 78%, var(--workspace-surface));
  color: var(--workspace-accent);
  font-size: 9px;
  font-weight: 700;
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
  grid-template-columns: minmax(0, 1fr);
  gap: 0;
  padding: 26px 0;
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
  gap: 16px 14px;
}

.settings-fields label {
  gap: 8px;
  color: var(--workspace-muted);
  font-size: 11px;
  font-weight: 600;
}

.settings-fields input,
.settings-fields select {
  height: 40px;
  padding: 0 12px;
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
  box-shadow: 0 0 0 3px color-mix(in oklch, var(--workspace-accent) 16%, transparent);
}

.field-hint {
  color: var(--workspace-subtle);
  font-size: 10px;
  font-weight: 500;
  line-height: 1.45;
}

.field-hint a {
  color: var(--workspace-accent);
  font-weight: 650;
  text-decoration: none;
}

.field-hint a:hover,
.field-hint a:focus-visible {
  text-decoration: underline;
  outline: none;
}

.key-input {
  position: relative;
  display: block;
}

.key-input input {
  width: 100%;
  padding-right: 44px;
}

.key-visibility {
  position: absolute;
  top: 50%;
  right: 5px;
  display: grid;
  width: 34px;
  height: 34px;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--workspace-muted);
  cursor: pointer;
  transform: translateY(-50%);
}

.key-visibility:hover,
.key-visibility:focus-visible {
  background: var(--workspace-surface-muted);
  color: var(--workspace-accent);
  outline: none;
}

.asset-section-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 18px;
  margin: 0 0 9px;
}

.asset-section-heading.runtime-heading {
  margin-top: 22px;
}

.asset-section-heading strong,
.asset-section-heading small,
.asset-section-heading > span {
  display: block;
}

.asset-section-heading strong {
  color: var(--workspace-ink);
  font-size: 12px;
}

.asset-section-heading small,
.asset-section-heading > span {
  margin-top: 4px;
  color: var(--workspace-subtle);
  font-size: 10px;
  line-height: 1.45;
}

.asset-section-heading > span {
  max-width: 45%;
  margin: 0;
  overflow: hidden;
  color: var(--workspace-muted);
  text-align: right;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.asset-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 0;
  overflow: hidden;
  border: 1px solid var(--workspace-border);
  border-radius: var(--radius-panel);
}

.asset-card {
  position: relative;
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
  transition: background-color 160ms ease-out, box-shadow 160ms ease-out;
}

.asset-card:last-child {
  border-bottom: 0;
}

.asset-card.selected {
  background: var(--workspace-surface);
  box-shadow: none;
}

.asset-card.selected strong {
  color: var(--workspace-accent);
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
  margin-top: 0;
  overflow: hidden;
  border: 1px solid var(--workspace-border);
  border-radius: var(--radius-panel);
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
  position: sticky;
  bottom: 0;
  z-index: 5;
  gap: 10px;
  margin-top: 26px;
  padding: 14px 0;
  border: 0;
  background: var(--workspace-surface);
  box-shadow: none;
}

.unsaved-modal p {
  margin: 0;
  color: var(--workspace-muted);
  font-size: 13px;
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
  .settings-setup-overview {
    grid-template-columns: 1fr;
    gap: 14px;
  }

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
  .settings-setup-overview {
    grid-template-columns: 1fr;
    gap: 14px;
    margin-bottom: 22px;
    padding: 15px;
  }

  .processing-mode-list {
    grid-template-columns: 1fr;
  }

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
    min-height: 0;
  }

  .mode-badge {
    grid-column: 2;
    justify-self: start;
  }

  .runtime-row {
    grid-template-columns: 1fr;
  }

  .asset-section-heading {
    align-items: flex-start;
    flex-direction: column;
    gap: 5px;
  }

  .asset-section-heading > span {
    max-width: 100%;
    text-align: left;
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
