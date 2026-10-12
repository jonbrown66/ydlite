<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import brandMark from '@/assets/brand-mark.svg'
import './styles/landing.css'
const downloadUrl = '/downloads/YDLite_0.2.0_x64-setup.exe'
const msiUrl = '/downloads/YDLite_0.2.0_x64_en-US.msi'
const githubUrl = 'https://github.com/jonbrown66/ydlite'
const activePreview = ref(0)
const menuOpen = ref(false)
const menuButton = ref<HTMLButtonElement | null>(null)
const imageDialog = ref<HTMLDialogElement | null>(null)
const previews = [
  { name: 'Downloads', src: '/landing/ydlite-app.png', alt: 'YDLite download workspace with format, file size and save location' },
  { name: 'History', src: '/landing/ydlite-history.png', alt: 'Searchable download history with file and folder actions' },
  { name: 'Settings', src: '/landing/ydlite-settings.png', alt: 'Download preferences and yt-dlp and FFmpeg tool management' },
]
const faqs = [
  { question: 'What do I need to get started?', answer: 'Install YDLite on Windows x64. The app checks yt-dlp and FFmpeg and can install missing tools. Downloads require an internet connection; no AI account or model download is needed.' },
  { question: 'Which websites are supported?', answer: 'YDLite uses yt-dlp to access supported video and audio sites. Availability depends on the site, your account permissions and the installed yt-dlp version. Updating yt-dlp can resolve changes in a site.' },
  { question: 'Can I save audio or a playlist?', answer: 'Choose Audio only to convert to MP3, or choose a video format. For a recognized playlist, select the entries you want and download them in order. Failed entries can be retried separately.' },
  { question: 'What if a video requires sign-in?', answer: 'Select your own cookies.txt file in the app. YDLite passes it to yt-dlp locally. Your account must have permission to access the media.' },
  { question: 'Does clearing history delete my files?', answer: 'No. History actions remove records only. Downloaded files remain in the folder you chose.' },
  { question: 'How are filenames and duplicates handled?', answer: 'Choose title, author + title or upload date + title. Chinese titles are preserved, with a short media and format ID to avoid name collisions. Keep an existing download, save a numbered copy or confirm replacement before starting.' },
]
function handlePreviewKeys(event: KeyboardEvent) {
  let next = activePreview.value
  if (event.key === 'ArrowRight') next = (next + 1) % previews.length
  else if (event.key === 'ArrowLeft') next = (next + previews.length - 1) % previews.length
  else if (event.key === 'Home') next = 0
  else if (event.key === 'End') next = previews.length - 1
  else return
  event.preventDefault(); activePreview.value = next
  void nextTick(() => document.getElementById(`preview-tab-${next}`)?.focus())
}
function closeMenuOnEscape(event: KeyboardEvent) { if (event.key === 'Escape' && menuOpen.value) { menuOpen.value = false; menuButton.value?.focus() } }
onMounted(() => document.addEventListener('keydown', closeMenuOnEscape))
onBeforeUnmount(() => document.removeEventListener('keydown', closeMenuOnEscape))
</script>

<template>
  <div id="top" class="landing-page">
    <a class="skip-link" href="#main-content">Skip to content</a>
    <header class="site-header"><nav class="landing-nav site-width" aria-label="Main navigation">
      <a class="brand" href="#top" aria-label="YDLite home"><span class="brand-mark"><img :src="brandMark" width="30" height="30" alt="" /></span><span>YDLite</span></a>
      <button ref="menuButton" class="menu-toggle" type="button" :aria-expanded="menuOpen" aria-controls="site-menu" :aria-label="menuOpen ? 'Close navigation' : 'Open navigation'" @click="menuOpen = !menuOpen"><svg viewBox="0 0 24 24" aria-hidden="true"><path v-if="menuOpen" d="m6 6 12 12M18 6 6 18" /><path v-else d="M4 7h16M4 12h16M4 17h16" /></svg></button>
      <div id="site-menu" class="nav-links" :class="{ 'is-open': menuOpen }"><a href="#features" @click="menuOpen = false">Features</a><a href="#workflow" @click="menuOpen = false">How it works</a><a href="#faq" @click="menuOpen = false">FAQ</a></div>
      <a class="button primary nav-download" :href="downloadUrl" download>Download for Windows</a>
    </nav></header>
    <main id="main-content">
      <section class="hero-section site-width" aria-labelledby="hero-title">
        <div class="hero-intro"><p class="eyebrow">A video downloader for Windows</p><h1 id="hero-title">From link<br />to your folder.</h1><p class="hero-lede">Choose the video or audio you want. Save it with a clear view of the format, file size and download progress.</p><div class="hero-actions"><a class="button primary" :href="downloadUrl" download>Download for Windows <span aria-hidden="true">↓</span></a><a class="hero-secondary" :href="githubUrl" target="_blank" rel="noreferrer noopener">View on GitHub ↗</a></div><p class="hero-platform">Windows x64 · v0.2.0 · Open source</p><p class="first-run">First run: the app checks yt-dlp and FFmpeg and offers to install missing tools.</p></div>
        <div class="product-preview"><div class="preview-tabs" role="tablist" aria-label="Product previews" @keydown="handlePreviewKeys"><button v-for="(preview, index) in previews" :id="`preview-tab-${index}`" :key="preview.name" type="button" role="tab" :aria-selected="activePreview === index" :tabindex="activePreview === index ? 0 : -1" aria-controls="product-preview-panel" @click="activePreview = index">{{ preview.name }}</button></div><figure class="hero-stage"><button id="product-preview-panel" class="stage-viewport" role="tabpanel" :aria-labelledby="`preview-tab-${activePreview}`" type="button" title="Enlarge screenshot" @click="imageDialog?.showModal()"><img class="hero-shot" :src="previews[activePreview]!.src" :alt="previews[activePreview]!.alt" width="1180" height="780" /><span class="enlarge-label">Enlarge screenshot ↗</span></button></figure><p class="preview-caption">Actual app interface with example download data</p></div>
      </section>
      <section id="features" class="features-section site-width"><div class="section-topline"><span>Made for the download</span><span>01 / Features</span></div><div class="feature-list"><article><span>01</span><div><h2>Choose before you download.</h2><p>Best available, up to 720p, MP3 audio or a specific video format. See an estimated size when the source provides enough information.</p></div></article><article><span>02</span><div><h2>Keep track of the whole queue.</h2><p>Select playlist entries, follow the current file and review the completed and failed counts. Retry failed downloads without repeating successful ones.</p></div></article><article><span>03</span><div><h2>Find what you saved.</h2><p>Search the latest 100 completed downloads by name, source or path. Open a file or its folder directly from history.</p></div></article></div></section>
      <section id="workflow" class="workflow-section site-width"><div class="section-topline"><span>A short path to your file</span><span>02 / How it works</span></div><ol class="workflow-list"><li><span>1</span><h2>Paste a link</h2><p>Analyze a supported video, audio or playlist URL.</p></li><li><span>2</span><h2>Choose your output</h2><p>Pick a format, select playlist entries and choose a save folder.</p></li><li><span>3</span><h2>Download and open</h2><p>Follow progress, retry failures and open the result.</p></li></ol></section>
      <section id="faq" class="faq-section site-width"><div><p class="eyebrow">Before you start</p><h2>A few useful answers.</h2><p>Download support follows yt-dlp and the access available to your account.</p></div><div class="faq-list"><details v-for="faq in faqs" :key="faq.question"><summary>{{ faq.question }}<span aria-hidden="true">+</span></summary><p>{{ faq.answer }}</p></details></div></section>
      <section id="download" class="download-section site-width"><div><p class="eyebrow">YDLite for Windows</p><h2>Keep your next download simple.</h2></div><div class="download-links"><a class="button primary" :href="downloadUrl" download>Download installer · EXE</a><a class="button secondary" :href="msiUrl" download>Windows installer · MSI</a></div></section>
    </main>
    <footer class="site-footer site-width"><a class="brand" href="#top"><span class="brand-mark"><img :src="brandMark" width="24" height="24" alt="" /></span>YDLite</a><span>Video and audio downloads. Saved locally.</span><a :href="githubUrl" target="_blank" rel="noreferrer noopener">GitHub ↗</a></footer>
    <dialog ref="imageDialog" class="screenshot-dialog" aria-label="Enlarged app screenshot" @click="($event.target === imageDialog) && imageDialog?.close()"><button class="button secondary" type="button" autofocus @click="imageDialog?.close()">Close preview</button><img :src="previews[activePreview]!.src" :alt="previews[activePreview]!.alt" width="1180" height="780" /></dialog>
  </div>
</template>
