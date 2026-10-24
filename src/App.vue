<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import brandMark from '@/assets/brand-mark.svg'
import clockIcon from 'iconoir/icons/clock.svg?url'
import downloadIcon from 'iconoir/icons/download.svg?url'
import settingsIcon from 'iconoir/icons/settings.svg?url'
import navArrowLeftIcon from 'iconoir/icons/nav-arrow-left.svg?url'
import sunIcon from 'iconoir/icons/sun-light.svg?url'
import moonIcon from 'iconoir/icons/half-moon.svg?url'
import AppDialog from '@/components/ui/AppDialog.vue'
import { canCloseWindow } from '@/lib/closeGuards'
import { Button } from '@/components/ui/button'
import { useDownloadStore } from '@/stores/download'
import { useAppearanceStore } from '@/stores/appearance'

const appWindow = getCurrentWindow()
const download = useDownloadStore()
const appearance = useAppearanceStore()
const themeAction = computed(() => appearance.resolved === 'dark' ? 'Switch to light mode' : 'Switch to dark mode')
const maximized = ref(false)
const sidebarCollapsed = ref(localStorage.getItem('ydlite.sidebarCollapsed') === '1')
const showCloseConfirm = ref(false)
const closeError = ref('')
let closePending: Promise<boolean> | undefined
let resolveClose: ((allow: boolean) => void) | undefined
let unlistenClose: (() => void) | undefined
let disposed = false

function usePointerMotion() { document.documentElement.dataset.interaction = 'pointer' }
function useKeyboardMotion() { document.documentElement.dataset.interaction = 'keyboard' }

function settleClose(allow: boolean) {
  showCloseConfirm.value = false
  resolveClose?.(allow)
  resolveClose = undefined
}

async function checkClose() {
  if (closePending) return closePending
  closePending = (async () => {
    try {
      if (!await canCloseWindow()) return false
      if (!download.running) return true
      showCloseConfirm.value = true
      return await new Promise<boolean>(resolve => { resolveClose = resolve })
    } catch {
      closeError.value = 'Unable to close the window. Try again.'
      return false
    }
  })()
  try { return await closePending } finally { closePending = undefined }
}

async function requestWindowClose() {
  try { await appWindow.close() }
  catch { closeError.value = 'Unable to close the window. Try again.' }
}

let unlistenResize: (() => void) | undefined

const navItems = [
  { name: 'download', label: 'Download', icon: downloadIcon },
  { name: 'tasks', label: 'History', icon: clockIcon },
] as const

function iconStyle(url: string) {
  return { '--icon-url': `url("${url}")` }
}

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value
  localStorage.setItem('ydlite.sidebarCollapsed', sidebarCollapsed.value ? '1' : '0')
}

async function syncMaximized() {
  maximized.value = await appWindow.isMaximized()
}

async function toggleMaximize() {
  if (await appWindow.isMaximized()) await appWindow.unmaximize()
  else await appWindow.maximize()
  await syncMaximized()
}

onMounted(async () => {
  document.addEventListener('pointerdown', usePointerMotion, true)
  document.addEventListener('keydown', useKeyboardMotion, true)
  unlistenClose = await appWindow.onCloseRequested(async event => {
    if (closePending) { event.preventDefault(); return }
    if (!await checkClose()) event.preventDefault()
  })
  if (disposed) { unlistenClose(); return }
  await syncMaximized()
  unlistenResize = await appWindow.onResized(() => void syncMaximized())
})

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', usePointerMotion, true)
  document.removeEventListener('keydown', useKeyboardMotion, true)
  delete document.documentElement.dataset.interaction
  disposed = true
  unlistenResize?.()
  unlistenClose?.()
  settleClose(false)
})
</script>

<template>
  <main class="app-frame" :class="{ 'sidebar-collapsed': sidebarCollapsed, 'window-maximized': maximized }">
    <header class="app-titlebar" data-tauri-drag-region @dblclick="toggleMaximize">
      <div class="app-brand" data-tauri-drag-region>
        <span class="app-brand-mark" data-tauri-drag-region>
          <img :src="brandMark" width="26" height="26" alt="" draggable="false" />
        </span>
        <strong data-tauri-drag-region>YDLite</strong>
      </div>
      <div class="titlebar-drag-area" data-tauri-drag-region />
      <div class="window-actions" role="group" aria-label="Window controls" @dblclick.stop>
        <button class="window-control" type="button" aria-label="Minimize window" title="Minimize" @click="appWindow.minimize()">
          <svg class="window-control-icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M3 8.5h10" /></svg>
        </button>
        <button class="window-control" type="button" :aria-label="maximized ? 'Restore window' : 'Maximize window'" :title="maximized ? 'Restore' : 'Maximize'" @click="toggleMaximize">
          <svg class="window-control-icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
            <path v-if="maximized" d="M5.5 5.5v-3h8v8h-3m-8-5h8v8h-8z" />
            <path v-else d="M3.5 3.5h9v9h-9z" />
          </svg>
        </button>
        <button class="window-control close-window" type="button" aria-label="Close window" title="Close" @click="requestWindowClose">
          <svg class="window-control-icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="m3.5 3.5 9 9m0-9-9 9" /></svg>
        </button>
      </div>
    </header>

    <div class="app-body">
      <aside class="app-sidebar">
        <div class="sidebar-main">
          <nav aria-label="Main navigation">
            <RouterLink v-for="item in navItems" :key="item.name" :to="{ name: item.name }" class="sidebar-link" :title="item.label" :aria-label="item.label">
              <i class="icon" :style="iconStyle(item.icon)" />
              <span>{{ item.label }}</span>
            </RouterLink>
          </nav>
        </div>
        <div class="sidebar-footer">
          <button class="sidebar-link appearance-toggle" type="button" :aria-label="themeAction" :title="themeAction" @click="appearance.toggle()">
            <i class="icon" aria-hidden="true" :style="iconStyle(appearance.resolved === 'dark' ? moonIcon : sunIcon)" />
            <span>{{ appearance.resolved === 'dark' ? 'Dark' : 'Light' }}</span>
          </button>
          <RouterLink :to="{ name: 'settings' }" class="sidebar-link settings-link" title="Settings" aria-label="Settings">
            <i class="icon" :style="iconStyle(settingsIcon)" />
            <span>Settings</span>
          </RouterLink>
          <button class="sidebar-collapse-toggle" type="button" :aria-label="sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'" :title="sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'" @click="toggleSidebar">
            <i class="icon" :style="iconStyle(navArrowLeftIcon)" />
            <span>{{ sidebarCollapsed ? 'Expand' : 'Collapse' }}</span>
          </button>
        </div>
      </aside>

      <section class="app-content">
        <RouterView v-slot="{ Component }">
          <Transition name="route-fade" mode="out-in">
            <KeepAlive>
              <component :is="Component" />
            </KeepAlive>
          </Transition>
        </RouterView>
      </section>
    </div>
    <p v-if="closeError" class="window-error" role="alert">{{ closeError }}</p>
    <p v-if="appearance.error" class="window-error" role="alert">{{ appearance.error }}</p>
    <AppDialog :open="showCloseConfirm" title="Download in progress" @close="settleClose(false)">
      <p>Closing will stop the current download.</p>
      <div class="modal-actions">
        <Button variant="outline" @click="settleClose(false)">Keep downloading</Button>
        <Button class="primary-danger" @click="settleClose(true)">Close anyway</Button>
      </div>
    </AppDialog>
  </main>
</template>
