<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import clockIcon from 'iconoir/icons/clock.svg?url'
import downloadIcon from 'iconoir/icons/download.svg?url'
import mediaVideoIcon from 'iconoir/icons/media-video.svg?url'
import minusIcon from 'iconoir/icons/minus.svg?url'
import settingsIcon from 'iconoir/icons/settings.svg?url'
import translateIcon from 'iconoir/icons/translate.svg?url'
import xmarkIcon from 'iconoir/icons/xmark.svg?url'
import { Button } from '@/components/ui/button'

const appWindow = getCurrentWindow()
const maximized = ref(false)
let unlistenResize: (() => void) | undefined

const navItems = [
  { name: 'download', label: '视频下载', icon: downloadIcon },
  { name: 'subtitles', label: '转录翻译', icon: translateIcon },
  { name: 'tasks', label: '任务记录', icon: clockIcon },
] as const

function iconStyle(url: string) {
  return { '--icon-url': `url("${url}")` }
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
  await syncMaximized()
  unlistenResize = await appWindow.onResized(() => void syncMaximized())
})

onBeforeUnmount(() => unlistenResize?.())
</script>

<template>
  <main class="app-frame">
    <header class="app-titlebar" data-tauri-drag-region @dblclick="toggleMaximize">
      <div class="app-brand" data-tauri-drag-region>
        <span class="app-brand-mark" data-tauri-drag-region>
          <i class="icon" :style="iconStyle(mediaVideoIcon)" />
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
        <Button class="close-window" size="icon" variant="ghost" aria-label="关闭窗口" title="关闭" @click="appWindow.close()">
          <i class="icon" :style="iconStyle(xmarkIcon)" />
        </Button>
      </div>
    </header>

    <div class="app-body">
      <aside class="app-sidebar">
        <div class="sidebar-main">
          <nav aria-label="主要功能">
            <RouterLink v-for="item in navItems" :key="item.name" :to="{ name: item.name }" class="sidebar-link">
              <i class="icon" :style="iconStyle(item.icon)" />
              <span>{{ item.label }}</span>
            </RouterLink>
          </nav>
        </div>
        <div class="sidebar-footer">
          <RouterLink :to="{ name: 'settings' }" class="sidebar-link settings-link">
            <i class="icon" :style="iconStyle(settingsIcon)" />
            <span>设置</span>
          </RouterLink>
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
  </main>
</template>
