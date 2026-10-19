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
import { useActivityStore } from '@/stores/activity'
import type { DependencyStatus, ToolInstallEvent } from '@/types'

const preferences = usePreferencesStore()
const activity = useActivityStore()
const downloadRunning = computed(() => activity.visibleTasks.some(task => task.kind === 'download' && ['active', 'cancelling'].includes(task.state)))
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
const qualityOptions = [{ value: 'best', label: 'Best available · prefer MP4' }, { value: 'p720', label: 'Up to 720p · prefer MP4' }, { value: 'mp3', label: 'Audio only · MP3' }]
function errorText(value: unknown) { return value && typeof value === 'object' && 'message' in value ? String(value.message) : String(value) }
async function chooseDirectory() { try { const path = await selectDirectory(); if (path) draft.value.defaultDir = path } catch (value) { error.value = errorText(value) } }
async function chooseCookies() { try { const path = await selectCookiesFile(); if (path) draft.value.cookiesFilePath = path } catch (value) { error.value = errorText(value) } }
function save() {
  error.value = ''; notice.value = ''
  try { preferences.save(draft.value); draft.value = cloneSettings(); notice.value = 'Download preferences saved.'; return true }
  catch { error.value = 'Unable to save preferences. Your changes are still here.'; return false }
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
  if (downloadRunning.value && ['install', 'update'].includes(action)) { error.value = 'Finish or cancel the download before replacing its tools.'; return }
  busy.value = true; error.value = ''; notice.value = ''; toolProgress.value = ''
  try {
    if (action === 'install') await installMissingTools()
    else if (action === 'update') await updateYtdlp()
    else if (action === 'folder') { await openPath(await getToolsDirectory()); return }
    await refreshTools()
    if (deps.value?.ytdlp_ok) deps.value = { ...deps.value, ...await checkYtdlpUpdate() }
    notice.value = action === 'check' ? 'Tool check complete.' : 'Tools ready.'
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
        <h2>Download defaults</h2>
        <p class="section-description">Choose a starting format and folder. You can change them for each download.</p>
        <div class="settings-fields">
          <label><span>Default save folder</span><span class="settings-input-group"><input v-model="draft.defaultDir" aria-label="Default save folder" placeholder="Choose a folder" /><Button variant="outline" @click="chooseDirectory">Browse</Button></span></label>
          <label><span>Default format</span><AppSelect v-model="draft.defaultQuality" :options="qualityOptions" aria-label="Default format" /></label>
          <label class="settings-toggle"><input v-model="draft.clipboardSuggestions" type="checkbox" /><span>Suggest a video URL from the clipboard when the app gains focus</span></label>
        </div>
      </section>
      <section>
        <h2>Files and formats</h2>
        <p class="section-description">Defaults apply to new tasks. Saved tasks keep their own options.</p>
        <DownloadOptionsFields v-model="draft.fileOptions" />
      </section>
      <section>
        <h2>Login cookies</h2>
        <p class="section-description">An optional cookies.txt file for sites requiring sign-in. The file stays on your computer and is read by yt-dlp.</p>
        <div class="settings-input-group"><input v-model="draft.cookiesFilePath" aria-label="Cookies file" placeholder="No cookies file selected" /><Button variant="outline" @click="chooseCookies">Choose file</Button><Button v-if="draft.cookiesFilePath" variant="ghost" @click="draft.cookiesFilePath = ''">Clear</Button></div>
      </section>
      <section>
        <h2>Download tools</h2>
        <p class="section-description">yt-dlp downloads media. FFmpeg merges video and audio and converts MP3 files.</p>
        <div class="tool-row"><div><strong>yt-dlp</strong><span>{{ deps ? deps.ytdlp_ok ? deps.ytdlp_version || 'Installed' : 'Not installed' : 'Checking…' }}</span><code v-if="deps?.ytdlp_ok">{{ deps.ytdlp_path }}</code></div><Button v-if="deps?.ytdlp_update_available" variant="outline" :disabled="busy || downloadRunning" @click="toolAction('update')">Update yt-dlp</Button></div>
        <div class="tool-row"><div><strong>FFmpeg</strong><span>{{ deps ? deps.ffmpeg_ok ? 'Installed' : 'Not installed' : 'Checking…' }}</span><code v-if="deps?.ffmpeg_ok">{{ deps.ffmpeg_path }}</code></div></div>
        <div class="tool-actions"><Button v-if="deps && (!deps.ytdlp_ok || !deps.ffmpeg_ok)" :disabled="busy || downloadRunning" @click="toolAction('install')">Install missing tools</Button><Button variant="outline" :disabled="busy" @click="toolAction('check')">{{ busy ? 'Working…' : 'Check for updates' }}</Button><Button variant="ghost" :disabled="busy" @click="toolAction('folder')">Open tools folder</Button></div>
        <p v-if="busy && toolProgress" role="status">{{ toolProgress }}</p>
      </section>
    </div>
    <footer class="settings-actions"><span v-if="hasUnsavedChanges" class="save-state" role="status">Unsaved changes</span><Button :disabled="!hasUnsavedChanges" @click="save">Save changes</Button></footer>
    <p v-if="error" class="inline-alert error" role="alert">{{ error }}</p>
    <p v-if="notice" class="inline-alert success" role="status">{{ notice }}</p>
    <AppDialog :open="showUnsavedConfirm" title="Save changes?" @close="settleUnsavedConfirm(false)"><p>You have unsaved changes.</p><div class="modal-actions"><Button variant="ghost" @click="settleUnsavedConfirm(false)">Keep editing</Button><Button variant="outline" @click="discardAndContinue">Discard changes</Button><Button @click="saveAndContinue">Save and leave</Button></div></AppDialog>
  </section>
</template>

<style scoped>
.settings-page { display: flex; flex-direction: column; overflow: hidden; }
.settings-page > .page-header { flex-shrink: 0; }
.settings-sheet { flex: 1; min-block-size: 0; inline-size: 100%; max-inline-size: 1040px; margin: 1px auto; overflow-y: auto; overscroll-behavior: contain; scrollbar-gutter: stable; border-radius: var(--radius-panel); background: var(--surface); box-shadow: 0 0 0 1px var(--border); scrollbar-width: thin; }
.settings-sheet > section { padding: 20px 24px; border-bottom: 1px solid var(--border); }
.settings-sheet > section:last-child { border: 0; }
h2 { margin: 0 0 8px; font-size: 16px; font-weight: 600; }
.section-description { margin: 0 0 18px; color: var(--muted); max-width: 72ch; }
.settings-fields { display: grid; gap: 18px; }
.settings-fields > label { display: grid; gap: 6px; }
.settings-input-group { display: flex; gap: 8px; align-items: center; min-width: 0; flex-wrap: wrap; }
.settings-input-group input { flex: 1 1 220px; min-width: 0; height: 36px; padding: 0 12px; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--surface); color: var(--ink); }
.settings-fields > .settings-toggle { display: flex; gap: 10px; align-items: flex-start; cursor: pointer; }
.settings-toggle input { margin-top: 3px; }
.tool-row { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding-block: 12px; border-top: 1px solid var(--border); }
.tool-row > div { display: grid; gap: 4px; min-width: 0; }
.tool-row span { color: var(--muted); }
.tool-row code { font: inherit; font-size: 12px; color: var(--subtle); overflow-wrap: anywhere; }
.tool-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 16px; }
.settings-actions { flex-shrink: 0; display: flex; align-items: center; gap: 16px; width: 100%; max-width: 1040px; margin: 16px auto 0; padding: 12px 16px; border-radius: var(--radius-panel); background: var(--surface); box-shadow: var(--control-shadow); }
.settings-actions > button { margin-left: auto; }
.save-state { color: var(--muted); }
.inline-alert { flex-shrink: 0; margin-bottom: 0; }
@container workspace (max-width: 620px) { .settings-sheet > section { padding: 18px 16px; } }
</style>
