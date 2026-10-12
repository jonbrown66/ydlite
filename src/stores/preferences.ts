import { defineStore } from 'pinia'
import { ref } from 'vue'
import { readFileOptions } from '@/lib/downloadOptions'
import type { FileOptions } from '@/types'

export type DownloadPreferences = {
  defaultDir: string
  cookiesFilePath: string
  defaultQuality: 'best' | 'p720' | 'mp3'
  clipboardSuggestions: boolean
  fileOptions: FileOptions
}

const storageKey = 'ydlite.downloadPreferences'

export const usePreferencesStore = defineStore('preferences', () => {
  function read(): DownloadPreferences {
    let saved: Partial<DownloadPreferences> = {}
    try { saved = JSON.parse(localStorage.getItem(storageKey) || '{}') || {} } catch { /* Keep legacy preferences. */ }
    return {
      defaultDir: typeof saved.defaultDir === 'string' ? saved.defaultDir : localStorage.getItem('ydlite.defaultDir') || '',
      cookiesFilePath: typeof saved.cookiesFilePath === 'string' ? saved.cookiesFilePath : localStorage.getItem('ydlite.cookiesFilePath') || '',
      defaultQuality: ['best', 'p720', 'mp3'].includes(saved.defaultQuality || '') ? saved.defaultQuality! : 'best',
      clipboardSuggestions: saved.clipboardSuggestions !== false,
      fileOptions: readFileOptions(saved.fileOptions),
    }
  }
  const settings = ref(read())
  function save(next: DownloadPreferences) {
    const clean = { ...next, defaultDir: next.defaultDir.trim(), cookiesFilePath: next.cookiesFilePath.trim(), fileOptions: { ...readFileOptions(next.fileOptions), audioFormatId: null } }
    localStorage.setItem(storageKey, JSON.stringify(clean))
    settings.value = clean
  }
  function update(patch: Partial<DownloadPreferences>) { save({ ...settings.value, ...patch }) }
  return { settings, save, update }
})
