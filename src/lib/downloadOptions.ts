import type { FileOptions, VideoFormat } from '@/types'

export function readFileOptions(value?: Partial<FileOptions> | null): FileOptions {
  return {
    naming: ['title', 'authorTitle', 'dateTitle'].includes(value?.naming || '') ? value!.naming! : 'title',
    conflict: ['skip', 'number', 'overwrite'].includes(value?.conflict || '') ? value!.conflict! : 'skip',
    quality: value?.quality === 'quality' ? 'quality' : 'compatible',
    audioFormatId: typeof value?.audioFormatId === 'string' ? value.audioFormatId : null,
  }
}

export function formatLabel(format: VideoFormat): string {
  const video = format.vcodec && format.vcodec !== 'none'
  return [video ? format.height ? `${format.height}p` : format.resolution || 'Video' : 'Audio',
    video && format.fps ? `${format.fps} fps` : '', format.ext.toUpperCase(),
    video ? format.vcodec : format.acodec, format.language,
    !video && format.bitrateKbps ? `${Math.round(format.bitrateKbps)} kbps` : '',
    format.filesize || format.filesizeApprox ? `${((format.filesize || format.filesizeApprox)! / 1048576).toFixed(1)} MiB${!format.filesize ? ' ≈' : ''}` : '',
    `#${format.formatId}`].filter(Boolean).join(' · ')
}
