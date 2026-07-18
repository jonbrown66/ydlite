import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { clearSubtitleProjects, deleteSubtitleProject, listSubtitleProjects } from '@/api/tauri'
import type { DownloadHistoryItem, SubtitleProject } from '@/types'

export const useTasksStore = defineStore('tasks', () => {
  const downloads = ref<DownloadHistoryItem[]>([])
  const subtitleProjects = ref<SubtitleProject[]>([])
  const loading = ref(false)
  const error = ref('')

  const total = computed(() => downloads.value.length + subtitleProjects.value.length)

  async function refresh() {
    loading.value = true
    error.value = ''
    try {
      const raw = localStorage.getItem('ydlite.history')
      downloads.value = raw ? JSON.parse(raw) as DownloadHistoryItem[] : []
      subtitleProjects.value = await listSubtitleProjects()
    } catch (value) {
      error.value = value instanceof Error ? value.message : String(value)
    } finally {
      loading.value = false
    }
  }

  function saveDownloads(next: DownloadHistoryItem[]) {
    downloads.value = next
    localStorage.setItem('ydlite.history', JSON.stringify(next))
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
    removeDownload,
    clearDownloads,
    removeSubtitleProject,
    clearSubtitles,
  }
})
