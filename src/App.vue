<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import clockIcon from 'iconoir/icons/clock.svg?url'
import downloadIcon from 'iconoir/icons/download.svg?url'
import minusIcon from 'iconoir/icons/minus.svg?url'
import settingsIcon from 'iconoir/icons/settings.svg?url'
import translateIcon from 'iconoir/icons/translate.svg?url'
import navArrowLeftIcon from 'iconoir/icons/nav-arrow-left.svg?url'
import xmarkIcon from 'iconoir/icons/xmark.svg?url'
import AppDialog from '@/components/ui/AppDialog.vue'
import { canCloseWindow } from '@/lib/closeGuards'
import { Button } from '@/components/ui/button'
import SidebarProgress from '@/components/SidebarProgress.vue'
import { useActivityStore } from '@/stores/activity'

const appWindow = getCurrentWindow()
const activity = useActivityStore()
const maximized = ref(false)
const sidebarCollapsed = ref(localStorage.getItem('ydlite.sidebarCollapsed') === '1')
const showCloseConfirm = ref(false)
const closeError = ref('')
let closePending: Promise<boolean> | undefined
let resolveClose: ((allow: boolean) => void) | undefined
let unlistenClose: (() => void) | undefined
let disposed = false

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
      if (!activity.visibleTasks.some(task => ['active', 'cancelling'].includes(task.state))) return true
      showCloseConfirm.value = true
      return await new Promise<boolean>(resolve => { resolveClose = resolve })
    } catch {
      closeError.value = '无法检查未保存的内容，请稍后再试。'
      return false
    }
  })()
  try { return await closePending } finally { closePending = undefined }
}

async function requestWindowClose() {
  try { await appWindow.close() }
  catch { closeError.value = '无法关闭窗口，请稍后再试。' }
}

let unlistenResize: (() => void) | undefined

const navItems = [
  { name: 'download', label: '视频下载', icon: downloadIcon },
  { name: 'subtitles', label: '自动字幕', icon: translateIcon },
  { name: 'tasks', label: '任务记录', icon: clockIcon },
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
  void activity.startListening()
  unlistenClose = await appWindow.onCloseRequested(async event => {
    if (closePending) { event.preventDefault(); return }
    if (!await checkClose()) event.preventDefault()
  })
  if (disposed) { unlistenClose(); return }
  await syncMaximized()
  unlistenResize = await appWindow.onResized(() => void syncMaximized())
})

onBeforeUnmount(() => {
  disposed = true
  unlistenResize?.()
  unlistenClose?.()
  settleClose(false)
  activity.stopListening()
})
</script>

<template>
  <main class="app-frame" :class="{ 'sidebar-collapsed': sidebarCollapsed }">
    <header class="app-titlebar" data-tauri-drag-region @dblclick="toggleMaximize">
      <div class="app-brand" data-tauri-drag-region>
        <span class="app-brand-mark" data-tauri-drag-region>
          <span class="brand-glyph">Y</span>
        </span>
        <strong data-tauri-drag-region>YDLite</strong>
      </div>
      <div class="titlebar-drag-area" data-tauri-drag-region />
      <div class="window-actions" @dblclick.stop>
        <Button size="icon" variant="ghost" aria-label="最小化窗口" title="最小化" @click="appWindow.minimize()">
          <i class="icon" :style="iconStyle(minusIcon)" />
        </Button>
        <Button size="icon" variant="ghost" :aria-label="maximized ? '还原窗口' : '最大化窗口'" :title="maximized ? '还原' : '最大化'" @click="toggleMaximize">
          <span class="maximize-icon" :class="{ restore: maximized }" />
        </Button>
        <Button class="close-window" size="icon" variant="ghost" aria-label="关闭窗口" title="关闭" @click="requestWindowClose">
          <i class="icon" :style="iconStyle(xmarkIcon)" />
        </Button>
      </div>
    </header>

    <div class="app-body">
      <aside class="app-sidebar">
        <div class="sidebar-main">
          <nav aria-label="主要功能">
            <RouterLink v-for="item in navItems" :key="item.name" :to="{ name: item.name }" class="sidebar-link" :title="item.label" :aria-label="item.label">
              <i class="icon" :style="iconStyle(item.icon)" />
              <span>{{ item.label }}</span>
            </RouterLink>
          </nav>
        </div>
        <div class="sidebar-footer">
          <SidebarProgress />
          <RouterLink :to="{ name: 'settings' }" class="sidebar-link settings-link" title="设置" aria-label="设置">
            <i class="icon" :style="iconStyle(settingsIcon)" />
            <span>设置</span>
          </RouterLink>
          <button class="sidebar-collapse-toggle" type="button" :aria-label="sidebarCollapsed ? '展开侧栏' : '收起侧栏'" :title="sidebarCollapsed ? '展开侧栏' : '收起侧栏'" @click="toggleSidebar">
            <i class="icon" :style="iconStyle(navArrowLeftIcon)" />
            <span>{{ sidebarCollapsed ? '展开' : '收起侧栏' }}</span>
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
    <AppDialog :open="showCloseConfirm" title="仍有任务正在运行" @close="settleClose(false)">
      <p>关闭窗口会退出应用，未完成的任务可能中断。你可以返回任务页，等待完成或先取消任务。</p>
      <div class="modal-actions">
        <Button variant="outline" @click="settleClose(false)">继续等待</Button>
        <Button variant="danger" @click="settleClose(true)">仍然关闭</Button>
      </div>
    </AppDialog>
  </main>
</template>
