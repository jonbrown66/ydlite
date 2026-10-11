<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave, useRoute } from 'vue-router'
import checkIcon from 'iconoir/icons/check.svg?url'
import eyeClosedIcon from 'iconoir/icons/eye-closed.svg?url'
import eyeIcon from 'iconoir/icons/eye.svg?url'
import trashIcon from 'iconoir/icons/trash.svg?url'
import AppDialog from '@/components/ui/AppDialog.vue'
import { modelDescription, runtimeDescription, runtimeName } from '@/lib/assetLabels'
import PageHeader from '@/components/ui/PageHeader.vue'
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
const customModelsStatus = ref('Choose a provider to load available models.')
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
    label: 'OpenAI',
    apiBase: 'https://api.openai.com/v1',
    website: 'https://platform.openai.com/api-keys',
    models: ['gpt-5.6-sol', 'gpt-5.6-terra', 'gpt-5.6-luna'],
  },
  {
    id: 'gemini',
    label: 'Gemini',
    apiBase: 'https://generativelanguage.googleapis.com/v1beta/openai',
    website: 'https://aistudio.google.com/apikey',
    models: ['gemini-3.7-flash', 'gemini-3.6-flash', 'gemini-3.5-flash-lite'],
  },
  {
    id: 'kimi',
    label: 'Kimi Code',
    apiBase: 'https://api.kimi.com/coding/v1',
    website: 'https://www.kimi.com/code/',
    models: ['k3', 'k3-256k', 'kimi-for-coding', 'kimi-for-coding-highspeed'],
  },
  {
    id: 'glm',
    label: 'GLM',
    apiBase: 'https://api.z.ai/api/paas/v4',
    website: 'https://open.bigmodel.cn/usercenter/apikeys',
    models: ['glm-5.3', 'glm-5.1', 'glm-5-turbo', 'glm-5'],
  },
  {
    id: 'deepseek',
    label: 'DeepSeek',
    apiBase: 'https://api.deepseek.com/v1',
    website: 'https://platform.deepseek.com/api_keys',
    models: ['deepseek-v4-flash', 'deepseek-v4-pro', 'deepseek-v4-flash-vision-exp'],
  },
  { id: 'custom', label: 'Custom provider' },
]

const processingModes: Array<{
  value: ProcessingMode
  title: string
  badge: string
}> = [
  {
    value: 'local_free',
    title: 'Local · Free',
    badge: 'No API key',
  },
  {
    value: 'local_custom',
    title: 'Custom API',
    badge: 'Advanced',
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
  return [{ value: '', label: customModelsStatus.value || 'Choose a model' }]
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
  return new Intl.DateTimeFormat('en-US', {
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
    notice.value = `Cache cleared. Freed ${formatBytes(released)}.`
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
      // Preserve legacy fields for existing projects.
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
    if (showNotice) notice.value = 'Settings saved.'
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
    await testOpenAiCompatibleConnection()
    notice.value = 'Connection verified.'
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
    ? `Choose a model.`
    : 'Save your API key to test the connection.'
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
    notice.value = 'Whisper model installed.'
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
    notice.value = 'Whisper runtime installed.'
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
      customModelsStatus.value = `Choose a model.`
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
    ? `Choose a model.`
    : 'Save your API key to test the connection.'
})
</script>

<template>
  <section class="workspace-page settings-page">
    <PageHeader title="Settings" />

    <div class="settings-sheet">
      <section>
        <fieldset class="processing-mode-list">
          <legend>Processing mode</legend>
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
          </label>
        </fieldset>
      </section>

      <section v-if="settings.processingMode === 'local_custom'" ref="providerSetupSection">
        <h2 class="settings-section-title">AI provider</h2>
        <div class="settings-fields">
          <label class="wide">
            <span>Provider</span>
            <AppSelect
              :model-value="selectedProviderId"
              :options="providerOptions"
              aria-label="Provider"
              @update:model-value="selectProvider"
            />
            <small v-if="selectedProvider.website" class="field-hint">
              <a :href="selectedProvider.website" target="_blank" rel="noreferrer">{{ selectedProvider.label }} API dashboard ↗</a>
            </small>
          </label>
          <label class="wide">
            <span>Base URL</span>
            <input v-model="settings.openaiApiBase" :readonly="selectedProviderId !== 'custom'" type="url" placeholder="https://api.example.com/v1" />
          </label>
          <label class="wide">
            <span>Model</span>
            <AppSelect
              v-model="settings.openaiModel"
              :options="customModelOptions"
              :disabled="!customModels.length && !settings.openaiModel"
              aria-label="Model"
            />

          </label>
          <label class="wide">
            <span>API Key</span>
            <span class="key-input">
              <input v-model="openaiApiKey" :type="showOpenaiApiKey ? 'text' : 'password'" autocomplete="off" :placeholder="settings.hasOpenaiApiKey ? 'Saved securely. Leave blank to keep.' : 'Enter API key'" />
              <button
                type="button"
                class="key-visibility"
                :aria-label="showOpenaiApiKey ? 'Hide API key' : 'Show API key'"
                :title="showOpenaiApiKey ? 'Hide API key' : 'Show API key'"
                @click="showOpenaiApiKey = !showOpenaiApiKey"
              >
                <i class="icon" :style="iconStyle(showOpenaiApiKey ? eyeClosedIcon : eyeIcon)" />
              </button>
            </span>
          </label>
          <Button variant="outline" :disabled="saving || testingCustom || !canTestCustom" @click="testCustomConnection">
            {{ testingCustom ? 'Testing…' : 'Save and test' }}
          </Button>
        </div>
      </section>

      <section ref="localAssetsSection">
        <div class="whisper-assets">
          <div class="asset-section-heading">
            <div>
              <strong>Speech recognition</strong>
            </div>
          </div>
          <div class="asset-grid">
            <article v-for="model in whisperModels" :key="model.id" class="asset-card" :class="{ selected: settings.whisperModel === model.id }">
              <label>
                <input v-model="settings.whisperModel" type="radio" :value="model.id" />
                <span><strong>{{ model.name }}</strong><small>{{ model.downloadSize }}</small></span>
              </label>
              <p>{{ modelDescription(model.id) }}</p>
              <div v-if="progressFor('model', model.id)?.status === 'downloading'" class="asset-progress">
                <i :style="{ width: `${progressFor('model', model.id)?.percent || 0}%` }" />
                <span>{{ Math.round(progressFor('model', model.id)?.percent || 0) }}%</span>
              </div>
              <div class="asset-actions">
                <span :class="{ installed: model.installed }">{{ model.installed ? 'Installed' : 'Not installed' }}</span>
                <Button v-if="!model.installed" size="sm" variant="outline" :disabled="['downloading', 'extracting'].includes(progressFor('model', model.id)?.status || '')" @click="installModel(model.id)">
                  {{ progressFor('model', model.id)?.status === 'downloading' ? 'Downloading…' : `Download ${model.downloadSize}` }}
                </Button>
                <Button v-else size="sm" variant="ghost" @click="removeModel(model.id)">Remove</Button>
              </div>
            </article>
          </div>

          <div class="asset-section-heading runtime-heading">
            <div>
              <strong>Runtime</strong>
            </div>
          </div>
          <div class="runtime-grid">
            <article v-for="runtime in whisperRuntimes" :key="runtime.id" class="runtime-row">
              <label>
                <input v-model="settings.whisperRuntime" type="radio" :value="runtime.id" />
                <span><strong>{{ runtimeName(runtime.id) }}</strong><small>{{ runtimeDescription(runtime.id) }} · {{ runtime.downloadSize }}</small></span>
              </label>
              <div v-if="progressFor('runtime', runtime.id)?.status === 'downloading'" class="asset-progress compact">
                <i :style="{ width: `${progressFor('runtime', runtime.id)?.percent || 0}%` }" />
              </div>
              <span v-if="runtime.installed" class="installed">Installed</span>
              <Button v-else size="sm" variant="outline" :disabled="['downloading', 'extracting'].includes(progressFor('runtime', runtime.id)?.status || '')" @click="installRuntime(runtime.id)">
                {{ progressFor('runtime', runtime.id)?.status === 'extracting' ? 'Installing…' : progressFor('runtime', runtime.id)?.status === 'downloading' ? `${Math.round(progressFor('runtime', runtime.id)?.percent || 0)}%` : `Download ${runtime.downloadSize}` }}
              </Button>
            </article>
          </div>
        </div>
      </section>

      <section>
        <h2 class="settings-section-title">Storage</h2>
        <div class="cache-manager">
          <div class="cache-summary">
            <div>
              <span>Cache size</span>
              <strong>{{ cacheStatus ? formatBytes(cacheStatus.cacheBytes) : 'Calculating…' }}</strong>
            </div>
            <Button variant="outline" :disabled="clearingCache || !cacheStatus" @click="cleanCache">
              <i class="icon" :style="iconStyle(trashIcon)" />
              {{ clearingCache ? 'Clearing…' : 'Clear cache' }}
            </Button>
          </div>
          <p v-if="cacheStatus" class="preserved-size">
            Models {{ formatBytes(cacheStatus.modelBytes) }} · Projects {{ formatBytes(cacheStatus.projectBytes) }}
          </p>
          <div v-if="cacheStatus?.records.length" class="cleanup-history">
            <span>Cleanup history</span>
            <ol>
              <li v-for="record in cacheStatus.records.slice(0, 3)" :key="record.id">
                <time>{{ formatCleanupTime(record.cleanedAt) }}</time>
                <strong>{{ formatBytes(record.releasedBytes) }}</strong>
              </li>
            </ol>
          </div>
        </div>
      </section>

    </div>
      <footer class="settings-actions">
        <span v-if="saving || hasUnsavedChanges" class="save-state" role="status">{{ saving ? 'Saving…' : 'Unsaved changes' }}</span>
        <Button :disabled="saving || !hasUnsavedChanges" @click="save">
          <i class="icon" :style="iconStyle(checkIcon)" />
          {{ saving ? 'Saving…' : 'Save changes' }}
        </Button>
      </footer>

    <p v-if="error" class="inline-alert error" role="alert">{{ error }}</p>
    <p v-if="notice" class="inline-alert success" role="status">{{ notice }}</p>

    <AppDialog :open="showUnsavedConfirm" title="Save changes?" :busy="saving" @close="settleUnsavedConfirm(false)">
      <p>You have unsaved changes.</p>
      <div class="modal-actions">
        <Button variant="ghost" :disabled="saving" @click="settleUnsavedConfirm(false)">Keep editing</Button>
        <Button variant="outline" :disabled="saving" @click="discardAndContinue">Discard changes</Button>
        <Button :disabled="saving" @click="saveAndContinue">{{ saving ? 'Saving…' : 'Save and leave' }}</Button>
      </div>
    </AppDialog>
  </section>
</template>

<style scoped>

.settings-page { display: flex; flex-direction: column; overflow: hidden; }
.settings-page > .page-header { flex-shrink: 0; }
.settings-sheet { flex: 1; min-block-size: 0; inline-size: 100%; max-inline-size: 1040px; margin-inline: auto; overflow-y: auto; overscroll-behavior: contain; scrollbar-gutter: stable; padding: 0; margin-block: 1px; border-radius: var(--radius-panel); background: var(--surface); box-shadow: 0 0 0 1px var(--border); scrollbar-width: thin; }
.settings-sheet > section { margin: 0; padding: 20px 24px; border-bottom: 1px solid var(--border); }
.settings-sheet > section:last-child { border-bottom: 0; }
.settings-section-title { margin: 0 0 16px; font-size: 16px; font-weight: 600; }
.section-description { margin: 0 0 16px; color: var(--muted); line-height: 1.5; }
.processing-mode-list { display: grid; grid-template-columns: repeat(auto-fit,minmax(min(100%,14rem),1fr)); gap: 12px; margin: 0; padding: 0; border: 0; min-width: 0; }
.processing-mode-list legend { font-size: 16px; font-weight: 600; margin-bottom: 12px; padding: 0; }
.processing-mode-list .section-description { grid-column: 1 / -1; margin: 0; }
.processing-mode { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; padding: 12px 16px; border: 1px solid var(--border); border-radius: var(--radius-control); cursor: pointer; }
.processing-mode:hover { background: var(--surface-hover); }
.processing-mode.selected { border-color: var(--primary-line); background: var(--primary-soft); }
.processing-mode:has(input:focus-visible) { outline: 2px solid var(--primary); outline-offset: 2px; }
.processing-mode input:focus-visible { outline: none; }
.processing-mode input { margin: 0; }
.mode-copy { flex: 1; }
.mode-copy strong { font-weight: 500; }
.mode-badge { color: var(--muted); white-space: nowrap; }
.settings-fields { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 16px; }
.settings-fields label { display: grid; gap: 6px; min-width: 0; }
.settings-fields label.wide { grid-column: 1 / -1; }
.settings-fields label > span:first-child { color: var(--muted); }
.settings-fields input { width: 100%; min-width: 0; height: 36px; padding: 0 12px; border: 1px solid var(--border); border-radius: 8px; background: var(--surface); color: var(--ink); }
.settings-fields input:hover:not(:disabled):not(:read-only) { border-color: var(--border-strong); }
.settings-fields input:read-only { background: var(--surface-subtle); color: var(--muted); }
.field-hint { color: var(--muted); font-size: 14px; font-weight: 400; overflow-wrap: anywhere; }
.field-hint a { color: var(--primary); text-decoration: none; }
.field-hint a:hover { text-decoration: underline; }
.key-input { position: relative; display: block; }
.key-input input { padding-right: 40px; }
.key-visibility { position: absolute; right: 4px; top: 2px; display: grid; place-items: center; width: 32px; height: 32px; border: 0; border-radius: 6px; background: transparent; color: var(--muted); }
.key-visibility:hover { background: var(--surface-hover); }
.settings-fields > button { width: fit-content; }
.asset-section-heading { display: flex; justify-content: space-between; gap: 12px; margin-bottom: 12px; }
.asset-section-heading strong { font-size: 16px; font-weight: 600; }
.asset-grid { display: grid; grid-template-columns: repeat(auto-fit,minmax(min(100%,18rem),1fr)); gap: 12px; }
.asset-card { display: flex; flex-direction: column; min-width: 0; padding: 16px; border: 1px solid var(--border); border-radius: var(--radius-control); }
.asset-card.selected { border-color: var(--primary-line); background: var(--surface-subtle); }
.asset-card:has(input:focus-visible) { outline: 2px solid var(--primary); outline-offset: 2px; }
.asset-card input:focus-visible { outline: none; }
.asset-card > label, .runtime-row > label { display: flex; align-items: flex-start; gap: 10px; cursor: pointer; }
.asset-card input, .runtime-row input { margin-top: 4px; }
.asset-card label strong, .asset-card label small, .runtime-row label strong, .runtime-row label small { display: block; }
.asset-card label strong, .runtime-row label strong { font-weight: 500; }
.asset-card small, .runtime-row small { color: var(--muted); font-size: 14px; margin-top: 2px; }
.asset-card > p { margin: 12px 0; color: var(--muted); }
.asset-actions { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; margin-top: auto; padding-top: 12px; }
.asset-actions > span { color: var(--muted); }
.asset-actions .installed, .installed { color: var(--success); }
.runtime-heading { margin-top: 24px; margin-bottom: 12px; }
.runtime-grid { display: grid; }
.runtime-row { display: flex; flex-wrap: wrap; align-items: center; gap: 16px; padding: 16px 0; border-bottom: 1px solid var(--border); }
.runtime-row:first-child { padding-top: 0; }
.runtime-row:last-child { padding-bottom: 0; border-bottom: 0; }
.runtime-row label { flex: 1 1 220px; min-width: 0; }
.runtime-row button, .runtime-row > span { flex-shrink: 0; }
.asset-progress { position: relative; height: 24px; margin-top: 8px; }
.asset-progress i { display: block; height: 4px; border-radius: 4px; background: var(--primary); }
.asset-progress span { display: block; margin-top: 2px; color: var(--muted); font-variant-numeric: tabular-nums; }
.asset-progress.compact { width: 80px; height: 4px; margin: 0; background: var(--border); }
.cache-summary { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
.cache-summary span, .cache-summary strong { display: block; }
.cache-summary span { color: var(--muted); }
.cache-summary strong { margin-top: 4px; font-size: 14px; font-weight: 500; font-variant-numeric: tabular-nums; }
.preserved-size, .empty-history { color: var(--muted); margin: 12px 0 0; }
.cleanup-history { margin-top: 20px; padding-top: 16px; border-top: 1px solid var(--border); }
.cleanup-history > span { font-weight: 500; }
.cleanup-history ol { margin: 8px 0 0; padding: 0; list-style: none; }
.cleanup-history li { display: flex; justify-content: space-between; gap: 16px; padding: 6px 0; color: var(--muted); }
.settings-actions { flex-shrink: 0; display: flex; align-items: center; justify-content: space-between; gap: 16px; width: 100%; max-width: 1040px; margin: 16px auto 0; padding: 12px 16px; border: 0; border-radius: var(--radius-panel); background: var(--surface); box-shadow: var(--control-shadow); }
.save-state { color: var(--muted); }
.settings-actions > button { margin-left: auto; }
.settings-page > .inline-alert { flex-shrink: 0; margin-bottom: 0; }

</style>
