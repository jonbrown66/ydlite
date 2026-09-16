import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { clearSubtitleProjects, deleteSubtitleProject, listSubtitleProjects } from '@/api/tauri'
import type { DownloadHistoryItem, SubtitleProject } from '@/types'

export const useTasksStore = defineStore('tasks', () => {
  const downloads = ref<DownloadHistoryItem[]>(readDownloads())
  function readDownloads(): DownloadHistoryItem[] {
    try {
      const parsed: unknown = JSON.parse(localStorage.getItem('ydlite.history') || '[]')
      return Array.isArray(parsed) ? parsed.filter((item): item is DownloadHistoryItem =>
        Boolean(item && typeof item.id === 'string' && typeof item.filePath === 'string' && typeof item.title === 'string'),
      ).slice(0, 10) : []
    } catch { return [] }
  }
  const subtitleProjects = ref<SubtitleProject[]>([])
  const loading = ref(false)
  const error = ref('')

  const total = computed(() => downloads.value.length + subtitleProjects.value.length)

  async function refresh() {
    if (loading.value) return
    loading.value = true
    error.value = ''
    try {
      downloads.value = readDownloads()
      subtitleProjects.value = await listSubtitleProjects()
    } catch (value) {
      error.value = value instanceof Error ? value.message : String(value)
    } finally {
      loading.value = false
    }
  }

  function saveDownloads(next: DownloadHistoryItem[]) {
    localStorage.setItem('ydlite.history', JSON.stringify(next))
    downloads.value = next
  }

  function addDownload(item: DownloadHistoryItem) {
    saveDownloads([item, ...downloads.value.filter(existing => existing.filePath !== item.filePath)].slice(0, 10))
  }
  function removeDownload(id: string) {
    saveDownloads(downloads.value.filter(item => item.id !== id))
  }

  function clearDownloads() {
    saveDownloads([])
  }

  async function removeSubtitleProject(id: string) {
    error.value = ''
    try {
      await deleteSubtitleProject(id)
      subtitleProjects.value = subtitleProjects.value.filter(project => project.id !== id)
      return true
    } catch (value) {
      error.value = errorText(value)
      return false
    }
  }

  async function clearSubtitles() {
    error.value = ''
    try {
      await clearSubtitleProjects()
      subtitleProjects.value = []
      return true
    } catch (value) {
      error.value = errorText(value)
      return false
    }
  }

  function errorText(value: unknown) {
    if (typeof value === 'object' && value && 'message' in value) {
      return String((value as { message: unknown }).message)
    }
    return value instanceof Error ? value.message : String(value)
  }

  return {
    downloads,
    subtitleProjects,
    loading,
    error,
    total,
    refresh,
    addDownload,
    removeDownload,
    clearDownloads,
    removeSubtitleProject,
    clearSubtitles,
  }
})
