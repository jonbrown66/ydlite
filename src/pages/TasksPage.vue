<script setup lang="ts">
import { computed, onActivated, ref } from 'vue'
import { useRouter } from 'vue-router'
import refreshIcon from 'iconoir/icons/refresh.svg?url'
import { Button } from '@/components/ui/button'
import PageHeader from '@/components/ui/PageHeader.vue'
import ActionMenu from '@/components/ui/ActionMenu.vue'
import { openParentFolder, openPath } from '@/api/tauri'
import { useTasksStore } from '@/stores/tasks'
const tasks = useTasksStore()
const router = useRouter()
const pendingDelete = ref('')
const pendingClear = ref(false)
const fileError = ref('')
const search = ref('')
const query = computed(() => search.value.trim().toLocaleLowerCase())
const visibleDownloads = computed(() => tasks.downloads.filter(item => `${item.title} ${item.extractor || ''} ${item.filePath}`.toLocaleLowerCase().includes(query.value)))
function iconStyle(url: string) { return { '--icon-url': `url("${url}")` } }
function formatDate(timestamp: string) {
  const date = new Date(timestamp)
  return Number.isNaN(date.getTime()) ? '' : date.toLocaleString('en-US', { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
}
async function openFile(path: string, folder = false) {
  fileError.value = ''
  try { if (folder) await openParentFolder(path); else await openPath(path) }
  catch { fileError.value = 'File not found.' }
}
function removeDownload(id: string) { try { tasks.removeDownload(id); pendingDelete.value = '' } catch { fileError.value = 'Unable to save.' } }
function clearDownloads() { try { tasks.clearDownloads(); pendingClear.value = false } catch { fileError.value = 'Unable to clear.' } }
function downloadAction(action: string, item: { id: string; filePath: string }) { if (action === 'folder') void openFile(item.filePath, true); else pendingDelete.value = `download:${item.id}` }
onActivated(() => tasks.refresh())
</script>

<template>
  <section class="workspace-page tasks-page">
    <PageHeader title="History"><Button size="icon" variant="outline" title="Refresh" aria-label="Refresh" @click="tasks.refresh"><i class="icon" :style="iconStyle(refreshIcon)" /></Button></PageHeader>
    <p v-if="fileError" class="inline-alert error" role="alert">{{ fileError }}</p>
    <div v-if="tasks.total > 0" class="task-toolbar"><input v-model="search" type="search" aria-label="Search history" placeholder="Search" /></div>
    <div v-if="tasks.total === 0" class="empty-state"><p>No downloads yet.</p><Button @click="router.push({ name: 'download' })">Download</Button></div>
    <div v-else-if="!visibleDownloads.length" class="empty-state"><p>No matches.</p><Button variant="outline" @click="search = ''">Clear</Button></div>
    <div v-else class="task-sections"><section class="task-section">
      <div class="section-heading"><h2>Downloads</h2><div class="section-tools"><template v-if="pendingClear"><Button size="sm" variant="ghost" @click="pendingClear = false">Cancel</Button><Button size="sm" class="primary-danger" @click="clearDownloads">Clear all</Button></template><Button v-else size="sm" variant="ghost" @click="pendingClear = true">Clear</Button></div></div>
      <div class="task-list"><article v-for="item in visibleDownloads" :key="item.id" class="task-row">
        <div class="task-copy"><strong :title="item.title">{{ item.title }}</strong><span :title="item.filePath">{{ item.filePath }}</span></div><time>{{ formatDate(item.completedAt) }}</time>
        <div class="task-actions"><template v-if="pendingDelete === `download:${item.id}`"><Button class="confirm-button" size="sm" variant="ghost" @click="pendingDelete = ''">Cancel</Button><Button class="confirm-button" size="sm" variant="danger" @click="removeDownload(item.id)">Delete</Button></template><template v-else><Button size="sm" variant="outline" @click="openFile(item.filePath)">Open</Button><ActionMenu label="More actions" :items="[{id:'folder',label:'Open folder'},{id:'delete',label:'Delete',danger:true}]" @select="downloadAction($event, item)" /></template></div>
      </article></div>
    </section></div>
  </section>
</template>

<style scoped>
.section-tools { display: flex; align-items: center; gap: 4px; }
.section-tools :deep(button) { block-size: var(--control-height-compact); padding-inline: var(--control-padding-inline); }
.task-actions .confirm-button { width: auto; min-width: 46px; padding-inline: 9px; }
.task-copy span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.task-row { grid-template-columns: minmax(0, 1fr) auto auto; }
</style>
