<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useDownloadQueueStore } from '@/stores/downloadQueue'
import { usePreferencesStore } from '@/stores/preferences'
import { openParentFolder, selectDirectory } from '@/api/tauri'
import { AppSelect } from '@/components/ui/select'
import AppDialog from '@/components/ui/AppDialog.vue'
import DownloadOptionsFields from '@/components/DownloadOptionsFields.vue'
import { readFileOptions } from '@/lib/downloadOptions'
import type { QueueJob } from '@/lib/downloadQueue'
import type { DownloadMode } from '@/types'

const props = defineProps<{ importActive: boolean; blocked: boolean; toolsReady: boolean }>()
const queue = useDownloadQueueStore()
const preferences = usePreferencesStore()
const links = ref('')
const dir = ref(preferences.settings.defaultDir)
const mode = ref(preferences.settings.defaultQuality)
const message = ref('')
const actionError = ref('')
const filter = ref('all')
const editing = ref('')
const editDir = ref('')
const editMode = ref<DownloadMode>('best')
const fileOptions = ref(readFileOptions(preferences.settings.fileOptions))
const editFileOptions = ref(readFileOptions())
const formatOptions = [ { value: 'best', label: 'Best available · prefer MP4' }, { value: 'p720', label: 'Up to 720p · prefer MP4' }, { value: 'mp3', label: 'Audio only · MP3' } ]
const filterOptions = computed(() => [
  { value: 'all', label: `All tasks (${queue.jobs.length})` },
  { value: 'unfinished', label: `Unfinished (${queue.jobs.filter(job => job.status !== 'completed').length})` },
  { value: 'failed', label: `Failed (${queue.failed.length})` },
  { value: 'completed', label: `Completed (${queue.jobs.filter(job => job.status === 'completed').length})` },
])
const visible = computed(() => queue.jobs.filter(job => filter.value === 'all' || (filter.value === 'unfinished' ? job.status !== 'completed' : job.status === filter.value)))
const labels: Record<QueueJob['status'], string> = { pending: 'Awaiting analysis', analyzing: 'Analyzing…', queued: 'Ready', downloading: 'Downloading…', cancelling: 'Cancelling…', interrupted: 'Interrupted', completed: 'Completed', failed: 'Failed', cancelled: 'Cancelled' }
const activePercent = computed(() => typeof queue.progress?.percent === 'number' && Number.isFinite(queue.progress.percent) ? Math.min(100, Math.max(0, queue.progress.percent)) : null)
const activeLabel = computed(() => queue.progress?.status === 'processing' ? 'Processing…' : queue.progress?.status === 'starting' ? 'Preparing…' : 'Downloading…')
watch(() => preferences.settings, (next, previous) => {
  if (dir.value === previous.defaultDir) dir.value = next.defaultDir
  if (mode.value === previous.defaultQuality) mode.value = next.defaultQuality
  if (JSON.stringify(fileOptions.value) === JSON.stringify(previous.fileOptions)) fileOptions.value = readFileOptions(next.fileOptions)
})
function cookieSource() { return preferences.settings.cookiesFilePath ? { type: 'file' as const, path: preferences.settings.cookiesFilePath } : { type: 'none' as const } }
async function act(action: () => void | Promise<void>) {
  actionError.value = ''
  try { await action() } catch (error) { actionError.value = error instanceof Error ? error.message : 'Unable to complete this action. Try again.' }
}
async function importLinks() {
  if (props.blocked || queue.running || !links.value.trim()) return
  await act(async () => {
    if (!dir.value.trim()) throw new Error('Choose a save folder before importing links.')
    const result = queue.importLinks(links.value, { dir: dir.value, mode: mode.value, cookieSource: cookieSource(), fileOptions: { ...fileOptions.value } })
    message.value = `${result.ids.length} added · ${result.duplicates} duplicates skipped · ${result.invalid} invalid lines skipped`
    if (result.ids.length) {
      // Keep invalid input available for correction; valid input is safely in storage.
      links.value = result.invalidLines.join('\n')
      await queue.analyze(result.ids)
    }
  })
}
async function chooseFolder(edit = false) {
  await act(async () => { const selected = await selectDirectory(); if (selected) { if (edit) editDir.value = selected; else dir.value = selected } })
}
function beginEdit(job: QueueJob) { editing.value = job.id; editDir.value = job.dir; editMode.value = job.mode; editFileOptions.value = { ...job.fileOptions } }
function saveEdit(job: QueueJob) {
  void act(() => {
    if (!editDir.value.trim()) throw new Error('Choose a save folder.')
    queue.changeOptions(job.id, { dir: editDir.value, mode: editMode.value, formatId: editMode.value === 'custom' ? job.formatId : null, cookieSource: cookieSource(), fileOptions: { ...editFileOptions.value, audioFormatId: editMode.value === job.mode ? editFileOptions.value.audioFormatId : null } })
    editing.value = ''
  })
}
function appendLinks(text: string) {
  const input = text.split(/\r?\n/).map(line => line.trim()).filter(line => line && !line.startsWith('#')).join('\n')
  if (input) links.value = [links.value.trim(), input].filter(Boolean).join('\n')
}
defineExpose({ appendLinks })
</script>

<template>
  <section class="saved-queue" aria-label="Saved download queue">
    <div id="downloads-panel-multiple" class="batch-input-panel" role="tabpanel" aria-labelledby="downloads-tab-multiple" :hidden="!importActive" tabindex="0">
      <form id="batch-import" class="batch-import" @submit.prevent="importLinks">
        <label for="batch-links">Video, audio or playlist links</label>
        <textarea id="batch-links" v-model="links" rows="4" placeholder="https://example.com/video-1&#10;https://example.com/video-2" :disabled="blocked || queue.running" aria-describedby="batch-help" />
        <p id="batch-help">Duplicates are skipped. Playlists expand into individual tasks. Failed links stay in the queue.</p>
        <details class="batch-file-options"><summary>Files and format preference</summary><DownloadOptionsFields v-model="fileOptions" :disabled="blocked || queue.running" /></details>
        <div class="batch-options">
          <AppSelect v-model="mode" :options="formatOptions" aria-label="Batch format" :disabled="blocked || queue.running" />
          <div class="batch-folder"><span :title="dir">{{ dir || 'Choose a save folder' }}</span><button class="button" type="button" :disabled="blocked || queue.running" @click="chooseFolder()">Choose folder</button></div>
          <button class="button primary" type="submit" :disabled="blocked || queue.running || !toolsReady || !links.trim()">Import and analyze</button>
        </div>
        <p v-if="message" class="queue-feedback" role="status">{{ message }}</p>
      </form>
    </div>

    <template v-if="queue.jobs.length">
      <div class="saved-queue-heading">
        <div><h2>Download queue <span>{{ queue.jobs.length }}</span></h2><p>Tasks are saved. Continue when you’re ready.</p></div>
        <div class="saved-queue-actions">
          <button v-if="!queue.running" class="button primary" type="button" :disabled="blocked || !toolsReady || !queue.ready.length || !!queue.storageError" @click="act(() => queue.run())">Continue {{ queue.ready.length }} tasks</button>
          <button v-else class="button" type="button" :disabled="queue.pauseRequested" @click="queue.pause()">{{ queue.pauseRequested ? 'Pausing after this task…' : 'Pause after current' }}</button>
          <button v-if="queue.failed.length" class="button" type="button" :disabled="blocked || queue.running || !toolsReady" @click="act(() => queue.retry(queue.failed.map(job => job.id)))">Retry {{ queue.failed.length }} failed</button>
        </div>
      </div>
      <p v-if="queue.jobs.some(job => job.status === 'interrupted')" class="queue-recovery-note" role="status">Some tasks were interrupted. Continue will retry them; partial files are reused when the source supports it.</p>
      <div class="queue-toolbar">
        <AppSelect v-model="filter" :options="filterOptions" aria-label="Filter download queue" />
        <button class="link-button compact" type="button" :disabled="queue.running || !queue.jobs.some(job => job.status === 'completed')" @click="act(() => queue.clearCompleted())">Remove completed</button>
      </div>
      <ul class="saved-queue-list">
        <li v-for="job in visible" :key="job.id" class="saved-queue-item" :data-job-id="job.id" :data-status="job.status">
          <div class="queue-item-main">
            <div class="queue-item-title"><strong :title="job.title">{{ job.title }}</strong><span class="queue-state" :class="job.status">{{ job.skipped && job.status === 'completed' ? 'Already downloaded' : job.id === queue.activeId && job.status === 'downloading' ? activeLabel : labels[job.status] }}</span></div>
            <a class="queue-source" :href="job.url" :title="job.url" target="_blank" rel="noreferrer">{{ job.url }}</a>
            <p class="queue-options-summary" :title="job.dir">{{ job.mode === 'custom' ? `Format ${job.formatId}` : job.mode === 'p720' ? 'Up to 720p' : job.mode === 'mp3' ? 'MP3' : 'Best available' }} · {{ job.dir }}</p>
            <p v-if="job.error" class="queue-item-error">{{ job.error.message }}</p>
            <RouterLink v-if="job.error && ['tools', 'login', 'access'].includes(job.error.kind)" class="link-button compact" to="/settings">{{ job.error.kind === 'login' ? 'Update login cookies' : 'Check download tools' }}</RouterLink>
            <button v-if="job.error && ['disk', 'permission', 'folder', 'format'].includes(job.error.kind)" class="link-button compact" type="button" :disabled="blocked || queue.running" @click="beginEdit(job)">{{ job.error.kind === 'format' ? 'Change format' : 'Change save folder' }}</button>
            <p v-if="job.status === 'cancelling' && job.phase === 'analyze'" class="queue-options-summary">Waiting for analysis to finish; its result will be discarded.</p>
            <div v-if="job.id === queue.activeId && job.status === 'downloading'" class="queue-item-progress" role="progressbar" aria-label="Task download progress" :aria-valuenow="activePercent ?? undefined" :aria-valuemin="0" :aria-valuemax="100" :aria-valuetext="activeLabel">
              <span :style="{ transform: `scaleX(${(activePercent || 0) / 100})` }" />
            </div>
          </div>
          <div class="queue-item-actions">
            <button v-if="['pending', 'queued', 'interrupted', 'analyzing', 'downloading'].includes(job.status)" class="link-button compact" type="button" :disabled="blocked" :aria-label="`Cancel ${job.title}`" @click="act(() => queue.cancel(job.id))">Cancel</button>
            <button v-if="['failed', 'cancelled', 'interrupted'].includes(job.status)" class="link-button compact" type="button" :disabled="blocked || queue.running || !toolsReady" :aria-label="`Retry ${job.title}`" @click="act(() => queue.retry([job.id]))">Retry</button>
            <details v-if="job.id !== queue.activeId" class="queue-item-menu">
              <summary :aria-label="`Actions for ${job.title}`">More</summary>
              <div>
                <button v-if="job.status !== 'completed'" type="button" :disabled="blocked || queue.running || !toolsReady" @click="act(() => queue.analyze([job.id], cookieSource()))">Reanalyze with current cookies</button>
                <button v-if="job.status !== 'completed'" type="button" :disabled="blocked || queue.running" @click="beginEdit(job)">Edit folder and format</button>
                <button v-if="job.filePath" type="button" @click="act(() => openParentFolder(job.filePath!))">Open folder</button>
                <button type="button" :disabled="blocked || queue.running" @click="act(() => queue.remove(job.id))">Remove task</button>
              </div>
            </details>
          </div>
          <div v-if="editing === job.id" class="queue-edit">
            <DownloadOptionsFields v-model="editFileOptions" :disabled="queue.running || blocked" />
            <AppSelect v-model="editMode" :disabled="queue.running || blocked" :options="job.mode === 'custom' ? [...formatOptions, { value: 'custom', label: `Format ${job.formatId}` }] : formatOptions" aria-label="Task format" />
            <div class="batch-folder"><span :title="editDir">{{ editDir }}</span><button class="button" type="button" :disabled="queue.running || blocked" @click="chooseFolder(true)">Choose folder</button></div>
            <button class="button primary" type="button" :disabled="queue.running || blocked" @click="saveEdit(job)">Save task</button><button class="button" type="button" @click="editing = ''">Cancel edit</button>
          </div>
        </li>
      </ul>
      <p v-if="!visible.length" class="queue-feedback">No tasks match this filter.</p>
    </template>
    <p v-if="queue.notice" class="queue-feedback" role="status">{{ queue.notice }}</p>
    <AppDialog :open="queue.overwriteCount > 0" title="Replace existing downloads?" @close="queue.confirmOverwrite(false)"><p>{{ queue.overwriteCount }} selected tasks use the replace policy. Existing files for the same media and format in their save folders will be overwritten. This cannot be undone.</p><div class="modal-actions"><button class="button" type="button" @click="queue.confirmOverwrite(false)">Keep existing files</button><button class="button danger" type="button" @click="queue.confirmOverwrite(true)">Replace and download</button></div></AppDialog>
    <details v-if="queue.logs.length" class="queue-log"><summary>Latest task logs</summary><pre>{{ queue.logs.join('\n') }}</pre></details>
    <p v-if="actionError" class="queue-item-error" role="alert">{{ actionError }}</p>
    <div v-if="queue.storageError" class="queue-storage-error" role="alert"><p>{{ queue.storageError }}</p><button v-if="!queue.storageError.startsWith('Unable to read')" class="button" type="button" :disabled="queue.running" @click="queue.saveAgain()">Try saving again</button></div>
  </section>
</template>

<style scoped>
.batch-file-options { margin-bottom: 14px; }
.batch-file-options > summary { color: var(--muted); font-size: 13px; cursor: pointer; }
.batch-file-options[open] > summary { margin-bottom: 12px; }
.queue-edit > .output-options { flex: 1 1 100%; }
.saved-queue { width: 100%; max-width: 1040px; margin: 0 auto 24px; min-width: 0; }
.batch-input-panel[hidden] { display: none; }
.batch-import { display: grid; gap: 10px; padding: 0 1px 12px; }
.batch-import label { font-size: 13px; font-weight: 600; }
.batch-import textarea { inline-size: 100%; min-block-size: 110px; resize: vertical; padding-block: 10px; padding-inline: var(--control-padding-inline); border: 0; border-radius: var(--radius-control); box-shadow: var(--control-shadow); font: inherit; font-size: 14px; line-height: 1.6; color: var(--ink); background: var(--surface); }
.batch-import textarea:focus-visible { box-shadow: none; }
.batch-import p, .saved-queue-heading p, .queue-feedback, .queue-recovery-note { margin: 0; font-size: 12px; line-height: 1.6; color: var(--muted); }
.batch-options { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
.batch-folder { display: flex; gap: 10px; align-items: center; flex: 1; min-width: 180px; }
.batch-folder > span { min-width: 0; max-width: 220px; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); font-size: 12px; }
.batch-folder .button { flex-shrink: 0; }
.saved-queue-heading { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 12px; padding: 22px 0 14px; }
.saved-queue-heading h2 { font-size: 16px; margin: 0 0 4px; font-weight: 600; }
.saved-queue-heading h2 span { font-size: 12px; color: var(--muted); font-weight: 400; margin-left: 6px; }
.saved-queue-actions { display: flex; gap: 10px; flex-wrap: wrap; }
.queue-toolbar { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 10px 0; }
.queue-toolbar :deep(.app-select) { max-width: 200px; }
.saved-queue-list { list-style: none; padding: 0; margin: 0; max-height: 430px; overflow: auto; scrollbar-gutter: stable; border-block: 1px solid var(--border); }
.saved-queue-item { display: flex; flex-wrap: wrap; align-items: start; gap: 12px; padding: 14px 2px; border-bottom: 1px solid var(--border); }
.saved-queue-item:last-child { border-bottom: 0; }
.queue-item-main { flex: 1; min-width: 0; }
.queue-item-title { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }
.queue-item-title strong { min-width: 0; font-size: 13px; font-weight: 600; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.queue-state { font-size: 11px; color: var(--muted); flex-shrink: 0; }
.queue-state.failed { color: var(--danger); }.queue-state.completed { color: var(--success); }.queue-state.downloading, .queue-state.analyzing { color: var(--primary); }
.queue-source { display: block; color: var(--muted); font-size: 12px; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; text-decoration: none; margin: 4px 0; }.queue-source:hover { text-decoration: underline; }
.queue-options-summary { color: var(--subtle); font-size: 11px; margin: 0; overflow-wrap: anywhere; }
.queue-item-error { color: var(--danger); font-size: 12px; line-height: 1.6; margin: 7px 0 0; }
.queue-item-actions { display: flex; align-items: center; gap: 8px; }
.queue-item-menu summary { cursor: pointer; min-block-size: var(--control-height-compact); padding-block: 6px; padding-inline: 8px; border-radius: var(--radius-control); font-size: 14px; line-height: 20px; color: var(--muted); }
.queue-item-menu summary:active { background: var(--surface-hover); color: var(--ink); }
.queue-item-menu > div { display: grid; gap: var(--control-item-gap); max-inline-size: 160px; padding: var(--control-inset); border-radius: var(--control-group-radius); background: var(--surface); box-shadow: 0 0 0 1px var(--border); }
.queue-item-menu button { min-block-size: var(--control-height); border: 0; padding-block: 4px; padding-inline: var(--control-padding-inline); background: transparent; color: var(--ink); font-size: 14px; line-height: 20px; text-align: start; border-radius: var(--radius-option); cursor: pointer; }
.queue-item-menu button:disabled { opacity: var(--disabled-opacity); cursor: not-allowed; }
.queue-item-menu button:focus-visible { outline-offset: calc(-1 * var(--focus-width)); }
.queue-item-menu button:active:not(:disabled) { background: var(--primary-soft); }
@media (hover: hover) and (pointer: fine) { .queue-item-menu button:hover:not(:disabled), .queue-item-menu summary:hover { background: var(--surface-hover); } }
.queue-item-progress { height: 3px; background: var(--border); margin-top: 9px; border-radius: 2px; overflow: hidden; }.queue-item-progress span { display: block; height: 100%; background: var(--primary); transform-origin: left; }
.queue-edit { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; width: 100%; padding-top: 10px; }
.queue-feedback, .queue-recovery-note { padding-top: 8px; }.queue-storage-error { padding: 12px; margin-top: 10px; background: var(--danger-soft); border-radius: var(--radius-control); color: var(--danger); font-size: 13px; }
.queue-log { margin-top: 12px; font-size: 12px; color: var(--muted); }.queue-log summary { cursor: pointer; }.queue-log pre { max-height: 180px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; padding: 12px; background: var(--surface-subtle); border-radius: var(--radius-control); }
@media (max-width: 820px) { .batch-options > :first-child { width: 100%; } .batch-folder { width: 100%; flex-basis: 100%; }.queue-item-title { flex-direction: column; gap: 2px; }.queue-item-title strong { max-width: 100%; }.queue-edit > :first-child { width: 100%; } }
</style>
