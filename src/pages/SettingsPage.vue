<script setup lang="ts">
import { computed, onActivated, onBeforeUnmount, onMounted, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { Button } from '@/components/ui/button'
import { AppSelect } from '@/components/ui/select'
import AppDialog from '@/components/ui/AppDialog.vue'
import PageHeader from '@/components/ui/PageHeader.vue'
import DownloadOptionsFields from '@/components/DownloadOptionsFields.vue'
import { registerCloseGuard } from '@/lib/closeGuards'
import { checkDependencies, checkYtdlpUpdate, getToolsDirectory, installMissingTools, onToolInstallProgress, openPath, selectCookiesFile, selectDirectory, updateYtdlp } from '@/api/tauri'
import { usePreferencesStore, type DownloadPreferences } from '@/stores/preferences'
import { useDownloadStore } from '@/stores/download'
import { useAppearanceStore, type Appearance } from '@/stores/appearance'
import type { DependencyStatus, ToolInstallEvent } from '@/types'

const preferences = usePreferencesStore()
const download = useDownloadStore()
const appearance = useAppearanceStore()
const appearances: Array<{ value: Appearance; label: string }> = [{ value: 'system', label: 'System' }, { value: 'light', label: 'Light' }, { value: 'dark', label: 'Dark' }]
const downloadRunning = computed(() => download.running)
const cloneSettings = () => ({ ...preferences.settings, fileOptions: { ...preferences.settings.fileOptions } })
const draft = ref<DownloadPreferences>(cloneSettings())
const deps = ref<DependencyStatus | null>(null)
const busy = ref(false)
const toolProgress = ref('')
const error = ref('')
const notice = ref('')
const showUnsavedConfirm = ref(false)
let pendingLeave: Promise<boolean> | undefined
let resolveLeave: ((allow: boolean) => void) | undefined
let unregisterCloseGuard: (() => void) | undefined
let unlistenTools: (() => void) | undefined
let disposed = false
const hasUnsavedChanges = computed(() => JSON.stringify(draft.value) !== JSON.stringify(preferences.settings))
const qualityOptions = [{ value: 'best', label: 'Best' }, { value: 'p720', label: '720p' }, { value: 'mp3', label: 'Audio · MP3' }]
function errorText(value: unknown) { return value && typeof value === 'object' && 'message' in value ? String(value.message) : String(value) }
async function chooseDirectory() { try { const path = await selectDirectory(); if (path) draft.value.defaultDir = path } catch (value) { error.value = errorText(value) } }
async function chooseCookies() { try { const path = await selectCookiesFile(); if (path) draft.value.cookiesFilePath = path } catch (value) { error.value = errorText(value) } }
function save() {
  error.value = ''
  try { preferences.save(draft.value); draft.value = cloneSettings(); notice.value = 'Saved.'; return true }
  catch { error.value = 'Unable to save.'; return false }
}
function settleUnsavedConfirm(allow: boolean) { showUnsavedConfirm.value = false; resolveLeave?.(allow); resolveLeave = undefined }
function discardAndContinue() { draft.value = cloneSettings(); settleUnsavedConfirm(true) }
function saveAndContinue() { if (save()) settleUnsavedConfirm(true) }
async function requestUnsavedConfirm() {
  if (!hasUnsavedChanges.value) return true
  if (pendingLeave) return pendingLeave
  showUnsavedConfirm.value = true
  pendingLeave = new Promise<boolean>(resolve => { resolveLeave = resolve })
  try { return await pendingLeave } finally { pendingLeave = undefined }
}
async function refreshTools() { try { deps.value = await checkDependencies() } catch (value) { error.value = errorText(value) } }
async function toolAction(action: 'install' | 'update' | 'check' | 'folder') {
  if (busy.value) return
  if (downloadRunning.value && ['install', 'update'].includes(action)) { error.value = 'Stop the download first.'; return }
  busy.value = true; error.value = ''; notice.value = ''; toolProgress.value = ''
  try {
    if (action === 'install') await installMissingTools()
    else if (action === 'update') await updateYtdlp()
    else if (action === 'folder') { await openPath(await getToolsDirectory()); return }
    await refreshTools()
    if (deps.value?.ytdlp_ok) deps.value = { ...deps.value, ...await checkYtdlpUpdate() }
    notice.value = 'Done.'
  } catch (value) { error.value = errorText(value) }
  finally { busy.value = false }
}
function handleToolProgress(event: ToolInstallEvent) { toolProgress.value = `${event.tool} · ${event.status}${typeof event.percent === 'number' ? ` · ${Math.round(event.percent)}%` : ''}` }
onMounted(async () => {
  unregisterCloseGuard = registerCloseGuard(requestUnsavedConfirm)
  try { const unlisten = await onToolInstallProgress(handleToolProgress); if (disposed) unlisten(); else unlistenTools = unlisten }
  catch (value) { error.value = errorText(value) }
})
onActivated(() => { if (!hasUnsavedChanges.value) draft.value = cloneSettings(); void refreshTools() })
onBeforeUnmount(() => { disposed = true; unlistenTools?.(); unregisterCloseGuard?.(); settleUnsavedConfirm(false) })
onBeforeRouteLeave(requestUnsavedConfirm)
</script>

<template>
  <section class="workspace-page settings-page">
    <PageHeader title="Settings" />
    <div class="settings-sheet">
      <section>
        <h2>Appearance</h2>
        <div class="appearance-options segmented-control" role="group" aria-label="Appearance">
          <Button v-for="option in appearances" :key="option.value" class="segmented-option button ghost" :aria-pressed="appearance.preference === option.value" @click="appearance.choose(option.value)">{{ option.label }}</Button>
        </div>
      </section>
      <section>
        <h2>Defaults</h2>
        <div class="settings-fields">
          <label><span>Save folder</span><span class="settings-input-group"><input v-model="draft.defaultDir" aria-label="Save folder" placeholder="Choose a folder" /><Button variant="outline" @click="chooseDirectory">Browse</Button></span></label>
          <label><span>Format</span><AppSelect v-model="draft.defaultQuality" :options="qualityOptions" aria-label="Default format" /></label>
        </div>
      </section>
      <section>
        <h2>Files</h2>
        <DownloadOptionsFields v-model="draft.fileOptions" />
      </section>
      <section>
        <h2>Cookies</h2>
        <p class="hint">Optional cookies.txt for sites that need sign-in.</p>
        <div class="settings-input-group"><input v-model="draft.cookiesFilePath" aria-label="Cookies file" placeholder="No file" /><Button variant="outline" @click="chooseCookies">Choose</Button><Button v-if="draft.cookiesFilePath" variant="ghost" @click="draft.cookiesFilePath = ''">Clear</Button></div>
      </section>
      <section>
        <h2>Tools</h2>
        <div class="tool-row"><div><strong>yt-dlp</strong><span>{{ deps ? deps.ytdlp_ok ? deps.ytdlp_version || 'Installed' : 'Missing' : '…' }}</span></div><Button v-if="deps?.ytdlp_update_available" variant="outline" :disabled="busy || downloadRunning" @click="toolAction('update')">Update</Button></div>
        <div class="tool-row"><div><strong>FFmpeg</strong><span>{{ deps ? deps.ffmpeg_ok ? 'Installed' : 'Missing' : '…' }}</span></div></div>
        <div class="tool-actions"><Button v-if="deps && (!deps.ytdlp_ok || !deps.ffmpeg_ok)" :disabled="busy || downloadRunning" @click="toolAction('install')">Install missing tools</Button><Button variant="outline" :disabled="busy" @click="toolAction('check')">{{ busy ? 'Working…' : 'Check' }}</Button><Button variant="ghost" :disabled="busy" @click="toolAction('folder')">Open folder</Button></div>
        <p v-if="busy && toolProgress" role="status" class="hint">{{ toolProgress }}</p>
      </section>
    </div>
    <footer class="settings-actions"><span v-if="hasUnsavedChanges" class="save-state" role="status">Unsaved</span><Button :disabled="!hasUnsavedChanges" @click="save">Save</Button></footer>
    <p v-if="error" class="inline-alert error" role="alert">{{ error }}</p>
    <p v-if="notice" class="inline-alert success" role="status">{{ notice }}</p>
    <AppDialog :open="showUnsavedConfirm" title="Save changes?" @close="settleUnsavedConfirm(false)"><p>You have unsaved changes.</p><div class="modal-actions"><Button variant="ghost" @click="settleUnsavedConfirm(false)">Keep editing</Button><Button variant="outline" @click="discardAndContinue">Discard</Button><Button @click="saveAndContinue">Save</Button></div></AppDialog>
  </section>
</template>

<style scoped>
.settings-page { display: flex; flex-direction: column; overflow: hidden; }
.settings-page > .page-header { flex-shrink: 0; }
.settings-sheet { flex: 1; min-block-size: 0; inline-size: 100%; max-inline-size: 1040px; margin: 1px auto; overflow-y: auto; overscroll-behavior: contain; scrollbar-gutter: stable; border-radius: var(--radius-panel); background: var(--surface); box-shadow: 0 0 0 1px var(--border); scrollbar-width: thin; }
.settings-sheet > section { padding: 20px 24px; border-bottom: 1px solid var(--border); }
.settings-sheet > section:last-child { border: 0; }
h2 { margin: 0 0 12px; font-size: 15px; font-weight: 600; }
.hint { margin: 0 0 12px; color: var(--muted); }
.settings-fields { display: grid; gap: 16px; }
.settings-fields > label { display: grid; gap: 6px; }
.settings-input-group { display: flex; gap: 8px; align-items: center; min-width: 0; flex-wrap: wrap; }
.settings-input-group input { flex: 1 1 220px; min-inline-size: 0; block-size: var(--control-height); padding-block: 0; padding-inline: var(--control-padding-inline); border: 0; border-radius: var(--radius-control); background: var(--surface); color: var(--ink); box-shadow: var(--control-shadow); }
.settings-input-group input:focus-visible { box-shadow: none; }
.tool-row { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding-block: 10px; border-top: 1px solid var(--border); }
.tool-row > div { display: grid; gap: 2px; min-width: 0; }
.tool-row span { color: var(--muted); }
.tool-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 14px; }
.settings-actions { flex-shrink: 0; display: flex; align-items: center; gap: 16px; width: 100%; max-width: 1040px; margin: 16px auto 0; padding: 12px 16px; border-radius: var(--radius-panel); background: var(--surface); box-shadow: var(--control-shadow); }
.settings-actions > button { margin-left: auto; }
.save-state { color: var(--muted); }
.inline-alert { flex-shrink: 0; margin-bottom: 0; }
@container workspace (max-width: 620px) { .settings-sheet > section { padding: 16px; } }
</style>
