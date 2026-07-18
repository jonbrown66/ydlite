export type DownloadMode = 'best' | 'p720' | 'mp3' | 'custom'

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
  vcodec?: string | null
  acodec?: string | null
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
  url: string
  dir: string
  mode: DownloadMode
  formatId?: string | null
  options: ParseOptions
}

export type DownloadProgressEvent = {
  status: 'starting' | 'downloading' | 'processing' | 'finished' | 'cancelled' | 'error'
  percent?: number | null
  total?: string | null
  speed?: string | null
  eta?: string | null
  line?: string | null
  message?: string | null
  filePath?: string | null
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

export type SubtitleSegment = {
  id: string
  startMs: number
  endMs: number
  sourceText: string
  translatedText?: string | null
}

export type SubtitleUsage = {
  inputTokens: number
  outputTokens: number
  estimatedUsd: number
}

export type SubtitleProject = {
  id: string
  title: string
  sourcePath: string
  sourceFingerprint: string
  durationMs: number
  model: string
  sourceLanguage?: string | null
  targetLanguage?: string | null
  completedChunks: string[]
  segments: SubtitleSegment[]
  usage: SubtitleUsage
  status: string
  createdAt: number
  updatedAt: number
  lastError?: string | null
  asrProvider?: string | null
  translationProvider?: string | null
  artifacts: SubtitleArtifact[]
  performance: SubtitlePerformance
}

export type SubtitleStageRecord = {
  stage: string
  startedAtMs: number
  finishedAtMs?: number | null
  durationMs?: number | null
  status: string
  detail?: string | null
}

export type SubtitlePerformance = {
  stages: SubtitleStageRecord[]
  encoder?: string | null
  outputBytes?: number | null
  uploadedBytes: number
  retryCount: number
}

export type SubtitleArtifact = {
  kind: 'subtitle' | 'video'
  path: string
  format: string
  createdAt: number
}

export type GeminiSettings = {
  hasApiKey: boolean
  defaultModel: string
  defaultTargetLanguage: string
  maxCostUsd: number
  maxConcurrency: 1 | 2
  processingMode: 'local_free' | 'local_custom' | 'gemini'
  whisperModel: string
  whisperRuntime: 'cpu' | 'cuda'
  hasOpenaiApiKey: boolean
  openaiApiBase: string
  openaiModel: string
}

export type CleanupRecord = {
  id: string
  cleanedAt: number
  releasedBytes: number
  temporaryBytes: number
  webviewBytes: number
}

export type CacheStatus = {
  cacheBytes: number
  temporaryBytes: number
  webviewBytes: number
  modelBytes: number
  projectBytes: number
  records: CleanupRecord[]
}

export type WhisperModelInfo = {
  id: string
  name: string
  description: string
  downloadBytes: number
  downloadSize: string
  installed: boolean
  path?: string | null
}

export type WhisperRuntimeInfo = {
  id: string
  name: string
  description: string
  downloadBytes: number
  downloadSize: string
  installed: boolean
}

export type WhisperDownloadEvent = {
  assetType: 'model' | 'runtime'
  id: string
  status: string
  percent: number
  message: string
}

export type CostEstimate = {
  model: string
  durationMs: number
  inputTokens: number
  outputTokensLow: number
  outputTokensHigh: number
  estimatedUsdLow: number
  estimatedUsdHigh: number
  pricingEffectiveAt: string
}

export type SubtitleProgressEvent = {
  projectId: string
  stage: string
  percent: number
  chunkIndex?: number | null
  chunkTotal?: number | null
  message: string
  inputTokens: number
  outputTokens: number
  recoverable: boolean
}

export type SubtitleTrackInfo = {
  streamIndex: number
  codec: string
  language: string
  title?: string | null
  isDefault: boolean
  isForced: boolean
  isText: boolean
}

export type MediaSubtitleAnalysis = {
  sourcePath: string
  durationMs: number
  detectedLanguage?: string | null
  tracks: SubtitleTrackInfo[]
  recommendedTrack?: SubtitleTrackInfo | null
  strategy: 'extract_chinese' | 'translate_subtitle' | 'transcribe_audio'
  message: string
}
