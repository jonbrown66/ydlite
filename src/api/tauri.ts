import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import type { DependencyStatus, DownloadProgressEvent, DownloadRequest, ParseVideoRequest, ToolInstallEvent, VideoInfo } from '../types'

export function checkDependencies() {
  return invoke<DependencyStatus>('check_dependencies')
}

export function checkYtdlpUpdate() {
  return invoke<Pick<DependencyStatus, 'ytdlp_latest_version' | 'ytdlp_update_available'>>('check_ytdlp_update')
}

export function parseVideo(request: ParseVideoRequest) {
  return invoke<VideoInfo>('parse_video', { request })
}

export function startDownload(request: DownloadRequest) {
  return invoke<void>('start_download', { request })
}

export function cancelDownload() {
  return invoke<void>('cancel_download')
}

export function openPath(path: string) {
  return invoke<void>('open_path', { path })
}

export function openParentFolder(path: string) {
  return invoke<void>('open_parent_folder', { path })
}

export function getToolsDirectory() {
  return invoke<string>('get_tools_directory')
}

export function installMissingTools() {
  return invoke<void>('install_missing_tools')
}

export function updateYtdlp() {
  return invoke<void>('update_ytdlp')
}

export async function selectDirectory() {
  const selected = await open({ directory: true, multiple: false })
  return typeof selected === 'string' ? selected : null
}

export async function selectCookiesFile() {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [{ name: 'Cookies', extensions: ['txt'] }],
  })
  return typeof selected === 'string' ? selected : null
}

export function onDownloadProgress(handler: (event: DownloadProgressEvent) => void) {
  return listen<DownloadProgressEvent>('download://progress', (event) => handler(event.payload))
}

export function onToolInstallProgress(handler: (event: ToolInstallEvent) => void) {
  return listen<ToolInstallEvent>('tools://install-progress', (event) => handler(event.payload))
}
