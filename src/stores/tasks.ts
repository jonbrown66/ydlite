import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { DownloadHistoryItem } from '@/types'

export const downloadHistoryLimit = 100
export function readDownloadHistory(): DownloadHistoryItem[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem('ydlite.history') || '[]')
    return Array.isArray(parsed) ? parsed.filter((item): item is DownloadHistoryItem =>
      Boolean(item && typeof item.id === 'string' && typeof item.filePath === 'string'
        && typeof item.title === 'string' && typeof item.url === 'string' && typeof item.completedAt === 'string'),
    ).slice(0, downloadHistoryLimit) : []
  } catch { return [] }
}
export const useTasksStore = defineStore('tasks', () => {
  const downloads = ref<DownloadHistoryItem[]>(readDownloadHistory())
  const loading = ref(false)
  const error = ref('')
  const total = computed(() => downloads.value.length)
  function refresh() { downloads.value = readDownloadHistory() }
  function saveDownloads(next: DownloadHistoryItem[]) {
    localStorage.setItem('ydlite.history', JSON.stringify(next))
    downloads.value = next
  }
  function addDownload(item: DownloadHistoryItem) {
    saveDownloads([item, ...downloads.value.filter(existing => existing.filePath !== item.filePath)].slice(0, downloadHistoryLimit))
  }
  function removeDownload(id: string) { saveDownloads(downloads.value.filter(item => item.id !== id)) }
  function clearDownloads() { saveDownloads([]) }
  return { downloads, loading, error, total, refresh, addDownload, removeDownload, clearDownloads }
})
