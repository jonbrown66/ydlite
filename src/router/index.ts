import { createMemoryHistory, createRouter } from 'vue-router'
import DownloadPage from '@/pages/DownloadPage.vue'

const router = createRouter({
  history: createMemoryHistory(),
  routes: [
    {
      path: '/',
      redirect: '/download',
    },
    {
      path: '/download',
      name: 'download',
      component: DownloadPage,
      meta: { title: '视频下载', eyebrow: 'DOWNLOAD' },
    },
    {
      path: '/subtitles',
      name: 'subtitles',
      component: () => import('@/pages/SubtitlePage.vue'),
      meta: { title: '转录翻译', eyebrow: 'SUBTITLES' },
    },
    {
      path: '/tasks',
      name: 'tasks',
      component: () => import('@/pages/TasksPage.vue'),
      meta: { title: '任务记录', eyebrow: 'ACTIVITY' },
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/pages/SettingsPage.vue'),
      meta: { title: '设置', eyebrow: 'SETTINGS' },
    },
  ],
})

export default router
