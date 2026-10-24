import type { DownloadFailure } from '@/types'

const messages: Record<DownloadFailure['kind'], string> = {
  network: 'Check your connection, then retry.',
  access: 'The site blocked this request. Update yt-dlp or reanalyze.',
  login: 'Sign-in required. Update cookies.txt in Settings.',
  rateLimit: 'The site is limiting requests. Wait, then retry.',
  disk: 'Not enough disk space.',
  permission: 'Cannot write to this folder.',
  unsupported: 'This link is invalid or unsupported.',
  format: 'Choose another format, then retry.',
  folder: 'Check the save folder and its permissions.',
  tools: 'Install the download tools in Settings.',
  unknown: 'Download failed. Reanalyze the link or update yt-dlp.',
}

export function classifyDownloadError(error: unknown): { message: string; detail: string; kind: DownloadFailure['kind'] } {
  const value = error && typeof error === 'object' ? error as { message?: string; detail?: string } : null
  const detail = value?.detail || String(error ?? '')
  const text = `${value?.message || ''} ${detail}`.toLowerCase()
  const kind: DownloadFailure['kind'] =
    /ffmpeg|yt-dlp/.test(text) && /not found|no such file|unable to execute|未检测到|missing/.test(text) ? 'tools'
      : /disk full|no space left|not enough space|errno 28|磁盘空间/.test(text) ? 'disk'
        : /permission denied|access is denied|errno 13|拒绝访问/.test(text) ? 'permission'
          : /429|too many requests|rate.?limit/.test(text) ? 'rateLimit'
            : /private video|private content|login required|log in|sign in|sign-in|cookies?.{0,35}(expired|invalid|required|not found)|authentication|http error 401|members.only/.test(text) ? 'login'
              : /requested format|format is not available|invalid format id/.test(text) ? 'format'
                : /403|412|cloudflare|anti.bot|impersonat/.test(text) ? 'access'
                  : /invalid directory|filename|no such file or directory|保存目录无效/.test(text) ? 'folder'
                    : /unsupported url|no suitable extractor|invalid url/.test(text) ? 'unsupported'
                      : /timeout|timed out|connection|network|http error|dns|ssl|tls|certificate|eof occurred|transporterror|remote end closed/.test(text) ? 'network'
                        : 'unknown'
  return { kind, message: value?.message && kind === 'unknown' ? value.message : messages[kind], detail }
}
