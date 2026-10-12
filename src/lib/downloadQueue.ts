import type { CookieSource, DownloadMode, FileOptions } from '@/types'
import { readFileOptions } from '@/lib/downloadOptions'

export type QueueStatus = 'pending' | 'analyzing' | 'queued' | 'downloading' | 'cancelling' | 'interrupted' | 'completed' | 'failed' | 'cancelled'
export type QueueFailure = { kind: 'network' | 'access' | 'login' | 'rateLimit' | 'disk' | 'permission' | 'unsupported' | 'format' | 'folder' | 'tools' | 'unknown'; message: string }
export type QueueJob = {
  id: string
  url: string
  title: string
  dir: string
  mode: DownloadMode
  formatId: string | null
  cookieSource: CookieSource
  fileOptions: FileOptions
  skipped?: boolean
  status: QueueStatus
  phase: 'analyze' | 'download'
  extractor?: string | null
  filePath?: string
  error?: QueueFailure
  createdAt: string
  updatedAt: string
}
export const queueStorageKey = 'ydlite.downloadQueue.v1'
export const queueLimit = 500
const statuses: QueueStatus[] = ['pending', 'analyzing', 'queued', 'downloading', 'cancelling', 'interrupted', 'completed', 'failed', 'cancelled']

export function normalizeQueueUrl(value: string): string | null {
  try {
    const url = new URL(value.trim())
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) return null
    url.hash = ''
    return url.href
  } catch { return null }
}

export function collectQueueLinks(text: string, existing: string[] = []) {
  const seen = new Set(existing.map(normalizeQueueUrl).filter(Boolean))
  const urls: string[] = [], invalid: string[] = []
  let duplicates = 0
  for (const line of text.split(/\r?\n/).map(line => line.trim()).filter(Boolean)) {
    const url = normalizeQueueUrl(line)
    if (!url) { invalid.push(line); continue }
    if (seen.has(url)) { duplicates++; continue }
    seen.add(url)
    urls.push(url)
  }
  return { urls, invalid, duplicates }
}

export function classifyQueueError(error: unknown): QueueFailure {
  const value = error && typeof error === 'object' ? error as { message?: string; detail?: string } : null
  const text = (value?.detail || value?.message || String(error)).toLowerCase()
  const toolText = `${value?.message || ''} ${text}`.toLowerCase()
  if (/ffmpeg|yt-dlp/.test(toolText) && /not found|no such file|unable to execute|未检测到|missing/.test(toolText)) return { kind: 'tools', message: 'Install or update the download tools in Settings.' }
  if (/disk full|no space left|not enough space|errno 28|磁盘空间/.test(text)) return classifyStoredFailure('disk')
  if (/permission denied|access is denied|errno 13|拒绝访问/.test(text)) return classifyStoredFailure('permission')
  if (/429|too many requests|rate.?limit/.test(text)) return classifyStoredFailure('rateLimit')
  if (/private video|private content|login required|log in|sign in|sign-in|cookies?.{0,35}(expired|invalid|required|not found|does not look)|expired.{0,15}cookies?|not a netscape|authentication|http error 401|members.only/.test(text)) return classifyStoredFailure('login')
  if (/requested format|format is not available|invalid format id/.test(text)) return { kind: 'format', message: 'Choose another format, then retry.' }
  if (/403|412|cloudflare|anti.bot|impersonat/.test(text)) return classifyStoredFailure('access')
  if (/invalid directory|filename|no such file or directory|保存目录无效/.test(text)) return classifyStoredFailure('folder')
  if (/unsupported url|no suitable extractor|invalid url|empty playlist/.test(text)) return { kind: 'unsupported', message: 'This link is invalid, unsupported or has no downloadable entries.' }
  if (/timeout|timed out|connection|network|http error|dns|ssl|tls|certificate|eof occurred|transporterror|failed to perform|remote end closed/.test(text)) return { kind: 'network', message: 'Check your connection, then retry this task.' }
  // Persist only a safe summary; upstream diagnostics can contain signed URLs.
  return { kind: 'unknown', message: 'The task failed. Reanalyze the link or update yt-dlp, then retry.' }
}

export function readQueue(raw: string | null): QueueJob[] {
  if (!raw) return []
  const envelope: unknown = JSON.parse(raw)
  if (!envelope || typeof envelope !== 'object' || !('version' in envelope) || envelope.version !== 1 || !('jobs' in envelope) || !Array.isArray(envelope.jobs) || envelope.jobs.length > queueLimit) throw new Error('Unsupported or damaged queue data.')
  const seen = new Set<string>()
  return envelope.jobs.map((item): QueueJob => {
    if (!item || typeof item !== 'object' || typeof item.id !== 'string' || !item.id || seen.has(item.id)
      || typeof item.url !== 'string' || !normalizeQueueUrl(item.url) || typeof item.title !== 'string'
      || typeof item.dir !== 'string' || !['best', 'p720', 'mp3', 'custom'].includes(item.mode)
      || (item.formatId !== null && typeof item.formatId !== 'string') || !statuses.includes(item.status)
      || !['analyze', 'download'].includes(item.phase) || typeof item.createdAt !== 'string' || typeof item.updatedAt !== 'string') throw new Error('Damaged queue entry.')
    const source = item.cookieSource
    if (!source || !['none', 'file', 'browser'].includes(source.type)
      || (source.type === 'file' && typeof source.path !== 'string')
      || (source.type === 'browser' && !['chrome', 'edge', 'firefox'].includes(source.browser))) throw new Error('Damaged queue options.')
    seen.add(item.id)
    // Reconstruct allowed fields rather than retaining unknown storage properties.
    return {
      id: item.id, url: item.url, title: item.title, dir: item.dir, mode: item.mode, formatId: item.formatId,
      cookieSource: source.type === 'file' ? { type: 'file', path: source.path } : source.type === 'browser' ? { type: 'browser', browser: source.browser } : { type: 'none' },
      fileOptions: readFileOptions(item.fileOptions),
      skipped: item.skipped === true,
      status: ['analyzing', 'downloading', 'cancelling'].includes(item.status) ? 'interrupted' : item.status,
      phase: item.phase, createdAt: item.createdAt, updatedAt: item.updatedAt,
      extractor: typeof item.extractor === 'string' ? item.extractor : undefined,
      filePath: typeof item.filePath === 'string' ? item.filePath : undefined,
      error: item.error && ['network', 'access', 'login', 'rateLimit', 'disk', 'permission', 'unsupported', 'format', 'folder', 'tools', 'unknown'].includes(item.error.kind)
        ? classifyStoredFailure(item.error.kind) : undefined,
    }
  })
}

function classifyStoredFailure(kind: QueueFailure['kind']): QueueFailure {
  const messages: Record<QueueFailure['kind'], string> = {
    network: 'Check your connection, then retry this task.', access: 'The site blocked this request. Check access or update yt-dlp, then reanalyze.',
    login: 'Sign-in is required or cookies have expired. Replace cookies.txt in Settings, then reanalyze.',
    rateLimit: 'The site is limiting requests. Wait before retrying this task.',
    disk: 'Not enough disk space. Free space or choose another save folder.',
    permission: 'Cannot write to this folder. Choose a folder you can write to.',
    unsupported: 'This link is invalid, unsupported or has no downloadable entries.', format: 'Choose another format, then retry.',
    folder: 'Check the save folder, free space and write permission.', tools: 'Install or update the download tools in Settings.',
    unknown: 'The task failed. Reanalyze the link or update yt-dlp, then retry.',
  }
  return { kind, message: messages[kind] }
}
