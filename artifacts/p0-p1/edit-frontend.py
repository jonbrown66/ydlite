from pathlib import Path
p=Path('src/pages/DownloadPage.vue')
s=p.read_text(encoding='utf-8')
a=s.index('const availableFormats = computed(')
b=s.index('\nconst selectedFormat = computed',a)
s=s[:a]+'''const availableFormats = computed(() => (video.value?.formats || [])
  .filter(fmt => fmt.vcodec && fmt.vcodec !== 'none')
  .map(fmt => ({ label: formatLabel(fmt), formatId: fmt.formatId, height: fmt.height || 0 }))
  .sort((a, b) => b.height - a.height))
'''+s[b:]
s=s.replace('...availableFormats.value.map(format => ({','...(showDetailedFormats.value ? availableFormats.value : []).map(format => ({')
s=s.replace('cookieSource: video.value?.cookieSource || selectedCookieSource() }','cookieSource: video.value?.cookieSource || selectedCookieSource(), fileOptions: { ...fileOptions.value } }')
s=s.replace("    video.value = result", "    video.value = result\n    fileOptions.value.audioFormatId = null")
a=s.index('  const text = `${message}\\n${detail}`.toLowerCase()', s.index('function friendlyErrorMessage'))
b=s.index('\n}\n',a)
s=s[:a]+'''  const failure = classifyQueueError({ message, detail })
  return failure.kind === 'unknown' ? message : failure.message'''+s[b:]
s=s.replace('''          <div class="format-select-row save-location-row">''','''          <button v-if="video.formats?.length && !video.isPlaylist" class="link-button compact format-details-toggle" type="button" :disabled="downloading" :aria-expanded="showDetailedFormats" @click="showDetailedFormats = !showDetailedFormats">{{ showDetailedFormats ? 'Use simple format choices' : 'Choose a specific format' }}</button>
          <div v-if="audioOptions.length > 1 && !video.isPlaylist" class="format-select-row">
            <span>Audio track</span>
            <AppSelect v-model="fileOptions.audioFormatId" :options="audioOptions" :disabled="downloading || (!!selectedFormat && selectedFormat.acodec !== 'none')" aria-label="Audio track" />
          </div>
          <p v-if="selectedFormat && selectedFormat.acodec !== 'none'" class="format-help">This format includes audio. Choose a video-only format to select a separate audio track.</p>
          <p v-if="selectedFormat" class="format-help">The selected format is required. If it becomes unavailable, reanalyze or choose another format.</p>
          <details class="file-options-details"><summary>Files and format preference</summary><DownloadOptionsFields v-model="fileOptions" :disabled="downloading || queueActive" /></details>

          <div class="format-select-row save-location-row">''')
a=s.index('.download-tabs {')
b=s.index('.download-input-panel {',a)
s=s[:a]+'''.download-tabs { display: inline-flex; width: fit-content; max-width: 100%; gap: 4px; padding: 4px; align-items: stretch; border: 1px solid var(--border); border-radius: 10px; background: var(--surface-hover); }
.download-tab { min-height: 36px; min-width: 126px; padding: 7px 18px; border: 1px solid transparent; border-radius: 6px; background: transparent; color: var(--muted); font: inherit; font-size: 14px; font-weight: 500; cursor: pointer; transition: background-color 120ms ease, color 120ms ease, box-shadow 120ms ease; }
.download-tab:hover { background: color-mix(in srgb, var(--surface) 55%, transparent); color: var(--ink); }
.download-tab[aria-selected=true] { background: var(--surface); color: var(--ink); border-color: var(--border); box-shadow: 0 1px 2px rgb(24 24 27 / 8%); }
.download-tab:focus-visible { outline: 2px solid var(--primary); outline-offset: 1px; }
.file-options-details { margin-block: 14px; padding: 12px 0; border-block: 1px solid var(--border); }
.file-options-details > summary { cursor: pointer; color: var(--muted); font-size: 13px; }
.file-options-details[open] > summary { margin-bottom: 14px; color: var(--ink); }
.format-details-toggle { margin: 6px 0 10px; }
.format-help { font-size: 12px; color: var(--muted); margin-block: 6px 12px; }
@media (prefers-reduced-motion: reduce) { .download-tab { transition: none; } }
'''+s[b:]
s=s.replace('@media (forced-colors: active) { .download-tab[aria-selected=true]::after { background: Highlight; } .download-tab[aria-selected=true] { color: Highlight; } }','@media (forced-colors: active) { .download-tab[aria-selected=true] { background: Highlight; color: HighlightText; border-color: Highlight; } }')
p.write_text(s,encoding='utf-8')
p=Path('src-tauri/src/downloader.rs'); s=p.read_text(encoding='utf-8');s=s.replace('format_id: None, options: Default::default(),','format_id: None, options: Default::default(), file_options: Default::default(), overwrite_confirmed: false,');p.write_text(s,encoding='utf-8')
p=Path('scripts/download-storage-regression.cjs');s=p.read_text(encoding='utf-8');s=s.replace("    if (name === 'vue') return vue;", "    if (name.startsWith('@/lib/')) return load('src/lib/' + name.slice(6) + '.ts').module;\n    if (name === 'vue') return vue;");p.write_text(s,encoding='utf-8')
