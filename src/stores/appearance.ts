import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

export type Appearance = 'system' | 'light' | 'dark'
export const appearanceStorageKey = 'ydlite.appearance'

export function normalizeAppearance(value: unknown): Appearance {
  return value === 'light' || value === 'dark' ? value : 'system'
}

export const useAppearanceStore = defineStore('appearance', () => {
  const preference = ref<Appearance>('system')
  const systemDark = ref(false)
  const error = ref('')
  const resolved = computed(() => preference.value === 'system' ? systemDark.value ? 'dark' : 'light' : preference.value)
  let dispose: (() => void) | undefined

  function apply() {
    const root = document.documentElement
    root.dataset.theme = resolved.value
    root.style.colorScheme = resolved.value
    document.querySelector('meta[name="theme-color"]')?.setAttribute('content', resolved.value === 'dark' ? '#17191d' : '#f7f7f8')
  }

  function initialize() {
    if (dispose) return
    try { preference.value = normalizeAppearance(localStorage.getItem(appearanceStorageKey)) }
    catch { preference.value = 'system' }
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    systemDark.value = media.matches
    const onSystemChange = () => { systemDark.value = media.matches; apply() }
    const onStorage = (event: StorageEvent) => {
      if (event.storageArea === localStorage && (event.key === appearanceStorageKey || event.key === null)) {
        preference.value = normalizeAppearance(event.newValue)
        error.value = ''
        apply()
      }
    }
    media.addEventListener('change', onSystemChange)
    window.addEventListener('storage', onStorage)
    dispose = () => { media.removeEventListener('change', onSystemChange); window.removeEventListener('storage', onStorage); dispose = undefined }
    apply()
  }

  function choose(value: Appearance) {
    const next = normalizeAppearance(value)
    try { localStorage.setItem(appearanceStorageKey, next) }
    catch { error.value = 'Unable to save appearance. Your current theme is unchanged.'; return }
    preference.value = next
    error.value = ''
    apply()
  }

  function toggle() { choose(resolved.value === 'dark' ? 'light' : 'dark') }
  function stop() { dispose?.() }
  return { preference, resolved, error, initialize, choose, toggle, stop }
})
