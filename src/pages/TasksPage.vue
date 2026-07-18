<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import folderIcon from 'iconoir/icons/folder.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import pageIcon from 'iconoir/icons/empty-page.svg?url'
import refreshIcon from 'iconoir/icons/refresh.svg?url'
import trashIcon from 'iconoir/icons/trash.svg?url'
import { Button } from '@/components/ui/button'
import { openParentFolder, openPath } from '@/api/tauri'
import { useTasksStore } from '@/stores/tasks'
import type { SubtitleProject } from '@/types'

const tasks = useTasksStore()
const router = useRouter()
const pendingDelete = ref('')
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
  return Number.isNaN(date.getTime()) ? '时间未知' : date.toLocaleString('zh-CN')
}

function artifactPath(project: SubtitleProject, kind: 'subtitle' | 'video') {
  return project.artifacts.filter(item => item.kind === kind).at(-1)?.path || ''
}

function formatDuration(ms: number) {
  if (!ms) return ''
  const seconds = Math.max(1, Math.round(ms / 1000))
  if (seconds < 60) return `${seconds} 秒`
  const minutes = Math.floor(seconds / 60)
  const remain = seconds % 60
  return remain ? `${minutes}分${remain}秒` : `${minutes} 分钟`
}

function projectElapsed(project: SubtitleProject) {
  const elapsed = (project.performance?.stages || [])
    .filter(stage => stage.status === 'completed')
    .reduce((total, stage) => total + (stage.durationMs || 0), 0)
  return formatDuration(elapsed)
}

function projectStatus(project: SubtitleProject) {
  if (project.status === 'paused') return '可继续'
  if (project.status === 'failed') return '失败'
  if (project.status === 'burning' || project.status === 'processing') return '处理中'
  return ''
}

async function removeProject(id: string) {
  if (await tasks.removeSubtitleProject(id)) pendingDelete.value = ''
}

function removeDownload(id: string) {
  tasks.removeDownload(id)
  pendingDelete.value = ''
}

async function clearSubtitles() {
  if (await tasks.clearSubtitles()) pendingClear.value = ''
}

function clearDownloads() {
  tasks.clearDownloads()
  pendingClear.value = ''
}

onMounted(() => void tasks.refresh())
</script>

<template>
  <section class="workspace-page tasks-page">
    <header class="page-heading">
      <div>
        <h1>任务记录</h1>
        <p>管理最近任务。删除记录不会删除已生成的文件。</p>
      </div>
      <Button size="icon" variant="outline" :disabled="tasks.loading" :title="tasks.loading ? '正在刷新' : '刷新'" aria-label="刷新任务" @click="tasks.refresh">
        <i class="icon" :style="iconStyle(refreshIcon)" />
      </Button>
    </header>

    <p v-if="tasks.error" class="inline-alert error">{{ tasks.error }}</p>

    <div v-if="!tasks.loading && tasks.total === 0" class="empty-state">
      <strong>还没有任务记录</strong>
      <p>先下载视频，或者导入本地视频进行转录翻译。</p>
      <div>
        <Button @click="router.push({ name: 'download' })">开始下载</Button>
        <Button variant="outline" @click="router.push({ name: 'subtitles' })">生成字幕</Button>
      </div>
    </div>

    <div v-else class="task-sections">
      <section v-if="tasks.subtitleProjects.length" class="task-section">
        <div class="section-heading">
          <h2>字幕项目</h2>
          <div class="section-tools">
            <span>{{ tasks.subtitleProjects.length }} 个</span>
            <template v-if="pendingClear === 'subtitles'">
              <Button size="sm" variant="ghost" @click="pendingClear = ''">取消</Button>
              <Button size="sm" variant="danger" @click="clearSubtitles">确认清空</Button>
            </template>
            <Button v-else size="sm" variant="ghost" @click="pendingClear = 'subtitles'">清空</Button>
          </div>
        </div>
        <div class="task-list">
          <article v-for="project in tasks.subtitleProjects" :key="project.id" class="task-row">
            <div class="task-mark subtitle">字</div>
            <div class="task-copy">
              <strong>{{ project.title }}</strong>
              <span>
                {{ project.sourceLanguage || '自动识别' }} → 中文 · {{ project.segments.length }} 段
                <template v-if="projectElapsed(project)"> · {{ projectElapsed(project) }}</template>
                <template v-if="projectStatus(project)"> · {{ projectStatus(project) }}</template>
              </span>
            </div>
            <time>{{ formatDate(project.updatedAt) }}</time>
            <div class="task-actions">
              <template v-if="pendingDelete === `subtitle:${project.id}`">
                <Button class="confirm-button" size="sm" variant="ghost" @click="pendingDelete = ''">取消</Button>
                <Button class="confirm-button" size="sm" variant="danger" @click="removeProject(project.id)">删除记录</Button>
              </template>
              <template v-else>
                <Button v-if="artifactPath(project, 'video')" size="icon" variant="ghost" title="播放字幕视频" aria-label="播放字幕视频" @click="openPath(artifactPath(project, 'video'))">
                  <i class="icon" :style="iconStyle(mediaVideoIcon)" />
                </Button>
                <Button v-if="artifactPath(project, 'subtitle')" size="icon" variant="ghost" title="打开字幕" aria-label="打开字幕" @click="openPath(artifactPath(project, 'subtitle'))">
                  <i class="icon" :style="iconStyle(pageIcon)" />
                </Button>
                <Button size="icon" variant="ghost" title="打开文件夹" aria-label="打开文件夹" @click="openParentFolder(project.sourcePath)">
                  <i class="icon" :style="iconStyle(folderIcon)" />
                </Button>
                <Button size="icon" variant="ghost" title="删除项目记录" aria-label="删除项目记录" @click="pendingDelete = `subtitle:${project.id}`">
                  <i class="icon" :style="iconStyle(trashIcon)" />
                </Button>
              </template>
            </div>
          </article>
        </div>
      </section>

      <section v-if="tasks.downloads.length" class="task-section">
        <div class="section-heading">
          <h2>下载记录</h2>
          <div class="section-tools">
            <span>{{ tasks.downloads.length }} 个</span>
            <template v-if="pendingClear === 'downloads'">
              <Button size="sm" variant="ghost" @click="pendingClear = ''">取消</Button>
              <Button size="sm" variant="danger" @click="clearDownloads">确认清空</Button>
            </template>
            <Button v-else size="sm" variant="ghost" @click="pendingClear = 'downloads'">清空</Button>
          </div>
        </div>
        <div class="task-list">
          <article v-for="item in tasks.downloads" :key="item.id" class="task-row">
            <div class="task-mark download">下</div>
            <div class="task-copy">
              <strong>{{ item.title }}</strong>
              <span>{{ item.extractor || '视频' }}</span>
            </div>
            <time>{{ formatDate(item.completedAt) }}</time>
            <div class="task-actions">
              <template v-if="pendingDelete === `download:${item.id}`">
                <Button class="confirm-button" size="sm" variant="ghost" @click="pendingDelete = ''">取消</Button>
                <Button class="confirm-button" size="sm" variant="danger" @click="removeDownload(item.id)">删除记录</Button>
              </template>
              <template v-else>
                <Button size="icon" variant="ghost" title="打开视频" aria-label="打开视频" @click="openPath(item.filePath)">
                  <i class="icon" :style="iconStyle(mediaVideoIcon)" />
                </Button>
                <Button size="icon" variant="ghost" title="打开文件夹" aria-label="打开文件夹" @click="openParentFolder(item.filePath)">
                  <i class="icon" :style="iconStyle(folderIcon)" />
                </Button>
                <Button size="icon" variant="ghost" title="删除下载记录" aria-label="删除下载记录" @click="pendingDelete = `download:${item.id}`">
                  <i class="icon" :style="iconStyle(trashIcon)" />
                </Button>
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

.task-actions :deep(button[aria-label*="删除"]) {
  color: var(--subtle);
}

.task-actions :deep(button[aria-label*="删除"]:hover) {
  color: var(--danger);
}
</style>
