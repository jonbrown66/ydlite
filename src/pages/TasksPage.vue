<script setup lang="ts">
import { computed, onActivated, ref } from 'vue'
import { useRouter } from 'vue-router'
import downloadIcon from 'iconoir/icons/download.svg?url'
import translateIcon from 'iconoir/icons/translate.svg?url'
import refreshIcon from 'iconoir/icons/refresh.svg?url'
import { Button } from '@/components/ui/button'
import PageHeader from '@/components/ui/PageHeader.vue'
import ActionMenu from '@/components/ui/ActionMenu.vue'
import { openParentFolder, openPath } from '@/api/tauri'
import { useTasksStore } from '@/stores/tasks'
import type { SubtitleProject } from '@/types'

const tasks = useTasksStore()
const router = useRouter()
const pendingDelete = ref('')
const fileError = ref('')
const search = ref('')
const category = ref<'all' | 'downloads' | 'subtitles'>('all')
const categories = [
  { value: 'all', label: 'All' },
  { value: 'downloads', label: 'Downloads' },
  { value: 'subtitles', label: 'Subtitle projects' },
] as const
const query = computed(() => search.value.trim().toLocaleLowerCase())
const visibleDownloads = computed(() => category.value === 'subtitles' ? [] : tasks.downloads.filter(item =>
  `${item.title} ${item.extractor || ''}`.toLocaleLowerCase().includes(query.value),
))
const visibleProjects = computed(() => category.value === 'downloads' ? [] : tasks.subtitleProjects.filter(project =>
  `${project.title} ${project.sourceLanguage || ''}`.toLocaleLowerCase().includes(query.value),
))
function resetFilters() { search.value = ''; category.value = 'all' }
async function openFile(path: string, folder = false) {
  fileError.value = ''
  try { if (folder) await openParentFolder(path); else await openPath(path) }
  catch { fileError.value = 'File unavailable. Check its original location.' }
}
const pendingClear = ref<'' | 'subtitles' | 'downloads'>('')

function iconStyle(url: string) {
  return { '--icon-url': `url("${url}")` }
}

function formatDate(timestamp: number | string) {
  const numeric = typeof timestamp === 'number'
    ? timestamp
    : /^\d+$/.test(timestamp)
      ? Number(timestamp)
      : Number.NaN
  const value = Number.isFinite(numeric) && numeric < 1_000_000_000_000
    ? numeric * 1000
    : timestamp
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? 'Unknown date' : date.toLocaleString('en-US', { month: 'short', day: 'numeric', year: 'numeric', hour: 'numeric', minute: '2-digit' })
}

function artifactPath(project: SubtitleProject, kind: 'subtitle' | 'video') {
  return project.artifacts.filter(item => item.kind === kind).at(-1)?.path || ''
}

function formatDuration(ms: number) {
  if (!ms) return ''
  const seconds = Math.max(1, Math.round(ms / 1000))
  if (seconds < 60) return `${seconds} s`
  const minutes = Math.floor(seconds / 60)
  const remain = seconds % 60
  return remain ? `${minutes} min ${remain} s` : `${minutes} min`
}

function projectElapsed(project: SubtitleProject) {
  const elapsed = (project.performance?.stages || [])
    .filter(stage => stage.status === 'completed')
    .reduce((total, stage) => total + (stage.durationMs || 0), 0)
  return formatDuration(elapsed)
}

function projectStatus(project: SubtitleProject) {
  if (project.status === 'paused') return 'Paused'
  if (project.status === 'failed') return 'Failed'
  if (project.status === 'burning' || project.status === 'processing') return 'Processing'
  return ''
}

function openProject(project: SubtitleProject) {
  void router.push({ name: 'subtitles', query: { project: project.id } })
}

async function removeProject(id: string) {
  if (await tasks.removeSubtitleProject(id)) pendingDelete.value = ''
}

function removeDownload(id: string) {
  tasks.removeDownload(id)
  pendingDelete.value = ''
}
function downloadAction(action: string, item: { id: string; filePath: string }) {
  if (action === 'folder') void openFile(item.filePath, true)
  else pendingDelete.value = `download:${item.id}`
}
function projectActions(project: SubtitleProject) {
  return [
    ...(artifactPath(project, 'video') ? [{id:'video',label:'Play video'}] : []),
    ...(artifactPath(project, 'subtitle') ? [{id:'subtitle',label:'Open subtitles'}] : []),
    {id:'folder',label:'Open folder'},
    {id:'delete',label:'Delete',danger:true},
  ]
}
function projectAction(action: string, project: SubtitleProject) {
  if (action === 'delete') pendingDelete.value = `subtitle:${project.id}`
  else if (action === 'folder') void openFile(project.sourcePath, true)
  else void openFile(artifactPath(project, action === 'video' ? 'video' : 'subtitle'))
}

async function clearSubtitles() {
  if (await tasks.clearSubtitles()) pendingClear.value = ''
}

function clearDownloads() {
  tasks.clearDownloads()
  pendingClear.value = ''
}

onActivated(() => void tasks.refresh())
</script>

<template>
  <section class="workspace-page tasks-page">
    <PageHeader title="History">
      <Button size="icon" variant="outline" :disabled="tasks.loading" :title="tasks.loading ? 'Refreshing…' : 'Refresh'" aria-label="Refresh history" @click="tasks.refresh">
        <i class="icon" :style="iconStyle(refreshIcon)" />
      </Button>
    </PageHeader>

    <p v-if="tasks.error || fileError" class="inline-alert error" role="alert">{{ fileError || tasks.error }}</p>
    <p v-if="tasks.loading" class="loading-state" role="status">Loading history…</p>

    <div v-if="tasks.total > 0" class="task-toolbar">
      <div class="task-filters" role="group" aria-label="Task type">
        <button v-for="item in categories" :key="item.value" type="button" :aria-pressed="category === item.value" @click="category = item.value">{{ item.label }}</button>
      </div>
      <input v-model="search" type="search" aria-label="Search history" placeholder="Search by name or source" />
    </div>

    <div v-if="!tasks.loading && tasks.total === 0" class="empty-state">
      <strong>No history yet</strong>
      <p>Your downloads and subtitle projects will appear here.</p>
      <div>
        <Button @click="router.push({ name: 'download' })">Download</Button>
        <Button variant="outline" @click="router.push({ name: 'subtitles' })">Generate subtitles</Button>
      </div>
    </div>

    <div v-else-if="!tasks.loading && !visibleDownloads.length && !visibleProjects.length" class="empty-state">
      <strong>No matching tasks</strong>
      <p>Try another search or reset the filters.</p>
      <Button variant="outline" @click="resetFilters">Reset filters</Button>
    </div>
    <div v-else class="task-sections">
      <section v-if="visibleDownloads.length" class="task-section">
        <div class="section-heading">
          <h2>Downloads</h2>
          <div class="section-tools">
            <span>{{ visibleDownloads.length }}</span>
            <template v-if="pendingClear === 'downloads'">
              <Button size="sm" variant="ghost" @click="pendingClear = ''">Cancel</Button>
              <Button size="sm" variant="danger" @click="clearDownloads">Clear all records</Button>
            </template>
            <Button v-else size="sm" variant="ghost" title="Clear download history" @click="pendingClear = 'downloads'">Clear all</Button>
          </div>
        </div>
        <div class="task-list">
          <article v-for="item in visibleDownloads" :key="item.id" class="task-row">
            <div class="task-mark download"><i class="icon" :style="iconStyle(downloadIcon)" /></div>
            <div class="task-copy">
              <strong :title="item.title">{{ item.title }}</strong>
              <span>{{ item.extractor || 'Video' }}</span>
            </div>
            <time>{{ formatDate(item.completedAt) }}</time>
            <div class="task-actions">
              <template v-if="pendingDelete === `download:${item.id}`">
                <Button class="confirm-button" size="sm" variant="ghost" @click="pendingDelete = ''">Cancel</Button>
                <Button class="confirm-button" size="sm" variant="danger" title="Delete record only; keep files" @click="removeDownload(item.id)">Delete</Button>
              </template>
              <template v-else>
                <Button size="sm" variant="outline" @click="openFile(item.filePath)">Open video</Button>
                <ActionMenu label="More download actions" :items="[{id:'folder',label:'Open folder'},{id:'delete',label:'Delete',danger:true}]" @select="downloadAction($event, item)" />
              </template>
            </div>
          </article>
        </div>
      </section>

      <section v-if="visibleProjects.length" class="task-section">
        <div class="section-heading">
          <h2>Subtitle projects</h2>
          <div class="section-tools">
            <span>{{ visibleProjects.length }}</span>
            <template v-if="pendingClear === 'subtitles'">
              <Button size="sm" variant="ghost" @click="pendingClear = ''">Cancel</Button>
              <Button size="sm" variant="danger" @click="clearSubtitles">Clear all records</Button>
            </template>
            <Button v-else size="sm" variant="ghost" title="Clear subtitle history" @click="pendingClear = 'subtitles'">Clear all</Button>
          </div>
        </div>
        <div class="task-list">
          <article v-for="project in visibleProjects" :key="project.id" class="task-row">
            <div class="task-mark subtitle"><i class="icon" :style="iconStyle(translateIcon)" /></div>
            <div class="task-copy">
              <strong :title="project.title">{{ project.title }}</strong>
              <span>
                {{ project.sourceLanguage || 'Auto-detect' }} → Chinese · {{ project.segments.length }} segments
                <template v-if="projectElapsed(project)"> · {{ projectElapsed(project) }}</template>
                <template v-if="projectStatus(project)"> · {{ projectStatus(project) }}</template>
              </span>
            </div>
            <time>{{ formatDate(project.updatedAt) }}</time>
            <div class="task-actions">
              <template v-if="pendingDelete === `subtitle:${project.id}`">
                <Button class="confirm-button" size="sm" variant="ghost" @click="pendingDelete = ''">Cancel</Button>
                <Button class="confirm-button" size="sm" variant="danger" title="Delete record only; keep files" @click="removeProject(project.id)">Delete</Button>
              </template>
              <template v-else>
                <Button class="task-action-text" size="sm" variant="outline" @click="openProject(project)">
                  {{ ['paused', 'failed'].includes(project.status) ? 'Resume' : 'Open project' }}
                </Button>
                <ActionMenu label="More subtitle actions" :items="projectActions(project)" @select="projectAction($event, project)" />
              </template>
            </div>
          </article>
        </div>
      </section>


    </div>
  </section>
</template>

<style scoped>
.section-tools {
  display: flex;
  align-items: center;
  gap: 4px;
}

.section-tools :deep(button) {
  height: 28px;
  padding-inline: 9px;
}

.task-actions .confirm-button {
  width: auto;
  min-width: 46px;
  padding-inline: 9px;
}

.task-actions .task-action-text {
  width: auto;
  min-width: max-content;
  flex: 0 0 auto;
  padding-inline: 9px;
}

.task-actions :deep(button[aria-label*="Remove"]) {
  color: var(--subtle);
}

.task-actions :deep(button[aria-label*="Remove"]:hover) {
  color: var(--danger);
}
</style>
