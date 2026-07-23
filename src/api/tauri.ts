import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import type { CacheStatus, CostEstimate, DependencyStatus, DownloadProgressEvent, DownloadRequest, GeminiSettings, MediaSubtitleAnalysis, ParseVideoRequest, SubtitleProgressEvent, SubtitleProject, SubtitleSegment, SubtitleStyle, ToolInstallEvent, VideoInfo, WhisperDownloadEvent, WhisperModelInfo, WhisperRuntimeInfo } from '../types'

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

export const mediaUrl = (path: string) => convertFileSrc(path)

export async function selectMediaFile() {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [{ name: 'Media', extensions: ['mp4', 'mkv', 'mov', 'webm', 'avi', 'm4v', 'mp3', 'm4a', 'wav'] }],
  })
  return typeof selected === 'string' ? selected : null
}

export async function selectOutputFile(defaultPath: string, extensions: string[]) {
  return save({ defaultPath, filters: [{ name: 'Output', extensions }] })
}

export const getGeminiSettings = () => invoke<GeminiSettings>('get_gemini_settings')
export const saveGeminiSettings = (request: {
  apiKey?: string | null
  defaultModel: string
  defaultTargetLanguage: string
  maxCostUsd: number
  maxConcurrency: number
  processingMode: string
  whisperModel: string
  whisperRuntime: string
  openaiApiKey?: string | null
  openaiApiBase: string
  openaiModel: string
}) => invoke<GeminiSettings>('save_gemini_settings', { request })
export const testGeminiConnection = () => invoke<string>('test_gemini_connection')
export const testOpenAiCompatibleConnection = () => invoke<string>('test_openai_compatible_connection')
export const getCacheStatus = () => invoke<CacheStatus>('get_cache_status')
export const clearAppCache = () => invoke<CacheStatus>('clear_app_cache')
export const estimateTranscriptionCost = (model: string, durationMs: number, withTranslation: boolean) =>
  invoke<CostEstimate>('estimate_transcription_cost', { model, durationMs, withTranslation })
export const createSubtitleProject = (request: {
  sourcePath: string; title?: string | null; durationMs?: number | null; model?: string; targetLanguage?: string
}) => invoke<SubtitleProject>('create_subtitle_project', { request })
export const analyzeSubtitleSource = (sourcePath: string) =>
  invoke<MediaSubtitleAnalysis>('analyze_subtitle_source', { request: { sourcePath } })
export const importSubtitleTrack = (request: { projectId: string; streamIndex: number; language?: string }) =>
  invoke<SubtitleProject>('import_subtitle_track', { request })
export const listSubtitleProjects = () => invoke<SubtitleProject[]>('list_subtitle_projects')
export const deleteSubtitleProject = (projectId: string) =>
  invoke<boolean>('delete_subtitle_project', { projectId })
export const clearSubtitleProjects = () => invoke<number>('clear_subtitle_projects')
export const startGeminiTranscription = (request: {
  projectId: string; translate: boolean; targetLanguage?: string; model?: string
}) => invoke<SubtitleProject>('start_gemini_transcription', { request })
export const startGeminiTranslation = (request: { projectId: string; targetLanguage?: string; model?: string }) =>
  invoke<SubtitleProject>('start_gemini_translation', { request })
export const startWhisperTranscription = (request: { projectId: string; model: string; runtime: string }) =>
  invoke<SubtitleProject>('start_whisper_transcription', { request })
export const startBingTranslation = (request: { projectId: string; targetLanguage?: string }) =>
  invoke<SubtitleProject>('start_bing_translation', { request })
export const startOpenAiCompatibleTranslation = (request: { projectId: string; targetLanguage?: string }) =>
  invoke<SubtitleProject>('start_openai_compatible_translation', { request })
export const startGeminiPolish = (request: { projectId: string; targetLanguage?: string; model?: string }) =>
  invoke<SubtitleProject>('start_gemini_polish', { request })
export const cancelSubtitleTask = (projectId: string) => invoke<boolean>('cancel_subtitle_task', { projectId })
export const saveSubtitleSegments = (projectId: string, segments: SubtitleSegment[]) =>
  invoke<SubtitleProject>('save_subtitle_segments', { request: { projectId, segments } })
export const exportSubtitles = (request: { projectId: string; outputPath: string; format: string; content: string }) =>
  invoke<string>('export_subtitles', { request })
export const burnSubtitles = (request: { projectId: string; outputPath: string; content: string; style: SubtitleStyle }) =>
  invoke<string>('burn_subtitles', { request })
export const onSubtitleProgress = (handler: (event: SubtitleProgressEvent) => void) =>
  listen<SubtitleProgressEvent>('subtitle://progress', event => handler(event.payload))
export const listWhisperModels = () => invoke<WhisperModelInfo[]>('list_whisper_models')
export const listWhisperRuntimes = () => invoke<WhisperRuntimeInfo[]>('list_whisper_runtimes')
export const downloadWhisperModel = (id: string) => invoke<void>('download_whisper_model', { request: { id } })
export const downloadWhisperRuntime = (id: string) => invoke<void>('download_whisper_runtime', { request: { id } })
export const deleteWhisperModel = (id: string) => invoke<void>('delete_whisper_model', { request: { id } })
export const onWhisperDownloadProgress = (handler: (event: WhisperDownloadEvent) => void) =>
  listen<WhisperDownloadEvent>('whisper://download-progress', event => handler(event.payload))
