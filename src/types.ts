export type DownloadMode = 'best' | 'p720' | 'mp3' | 'custom'
export type FileOptions = {
  naming: 'title' | 'authorTitle' | 'dateTitle'
  conflict: 'skip' | 'number' | 'overwrite'
  quality: 'compatible' | 'quality'
  audioFormatId?: string | null
}

export type DependencyStatus = {
  ytdlp_ok: boolean
  ytdlp_version?: string | null
  ytdlp_latest_version?: string | null
  ytdlp_update_available: boolean
  ytdlp_path: string
  ffmpeg_ok: boolean
  ffmpeg_version?: string | null
  ffmpeg_path: string
}

export type VideoFormat = {
  formatId: string
  ext: string
  resolution?: string | null
  height?: number | null
  note?: string | null
  filesize?: number | null
  filesizeApprox?: number | null
  bitrateKbps?: number | null
  vcodec?: string | null
  acodec?: string | null
  fps?: number | null
  language?: string | null
}

export type PlaylistEntry = {
  id: string
  title: string
  url: string
  duration?: number | null
  uploader?: string | null
}

export type VideoInfo = {
  title: string
  uploader?: string | null
  duration?: number | null
  thumbnail?: string | null
  extractor?: string | null
  originalUrl: string
  resolvedUrl: string
  site: string
  parseStrategy: string
  isPlaylist?: boolean
  entries?: PlaylistEntry[] | null
  formats?: VideoFormat[] | null
  cookieSource?: CookieSource
}

export type CookieSource =
  | { type: 'none' }
  | { type: 'browser'; browser: 'firefox' | 'chrome' | 'edge' }
  | { type: 'file'; path: string }

export type ParseOptions = {
  cookieSource: CookieSource
}

export type ParseVideoRequest = {
  url: string
  options?: ParseOptions
}

export type DownloadRequest = {
  requestId?: string
  url: string
  dir: string
  mode: DownloadMode
  formatId?: string | null
  options: ParseOptions
  fileOptions?: FileOptions
  overwriteConfirmed?: boolean
}

export type DownloadResult = { cancelled: boolean; paused: boolean; filePath: string | null; skipped?: boolean }

export type DownloadFailure = { kind: 'network' | 'access' | 'login' | 'rateLimit' | 'disk' | 'permission' | 'unsupported' | 'format' | 'folder' | 'tools' | 'unknown'; message: string }

export type DownloadProgressEvent = {
  requestId?: string | null
  status: 'starting' | 'downloading' | 'processing' | 'finished' | 'paused' | 'cancelled' | 'error' | 'log'
  percent?: number | null
  total?: string | null
  speed?: string | null
  eta?: string | null
  line?: string | null
  message?: string | null
  filePath?: string | null
  metrics?: {
    downloadedBytes?: number | null
    totalBytes?: number | null
    totalIsEstimate: boolean
    speedBytes?: number | null
  } | null
}

export type ToolInstallEvent = {
  tool: 'yt-dlp' | 'ffmpeg'
  status: 'pending' | 'downloading' | 'extracting' | 'installed' | 'error'
  percent?: number | null
  message?: string | null
}

export type DownloadHistoryItem = {
  id: string
  title: string
  extractor?: string | null
  filePath: string
  url: string
  completedAt: string
}
