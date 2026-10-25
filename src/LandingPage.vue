<script setup lang="ts">
import { nextTick, ref } from 'vue'
import './styles/landing.css'

const setupUrl = '/downloads/YDLite_0.2.0_x64-setup.exe'
const portableUrl = '/downloads/YDLite_0.2.0_x64-portable.zip'
const msiUrl = '/downloads/YDLite_0.2.0_x64_en-US.msi'
const githubUrl = 'https://github.com/jonbrown66/ydlite'

const activePreview = ref(0)
const activeFaq = ref<number | null>(null)
const menuOpen = ref(false)
const menuButton = ref<HTMLButtonElement | null>(null)
const imageDialog = ref<HTMLDialogElement | null>(null)
const previews = [
  { name: 'Download', src: '/landing/ydlite-app.png', alt: 'YDLite download page with format selection and transfer progress' },
  { name: 'History', src: '/landing/ydlite-history.png', alt: 'YDLite download history with saved files and folder actions' },
  { name: 'Settings', src: '/landing/ydlite-settings.png', alt: 'YDLite settings for appearance, folders and file naming' },
]
const features = [
  { title: 'Video or audio', description: 'Choose the available quality or save audio as MP3.' },
  { title: 'Pause and resume', description: 'Continue from where you stopped on sites that support resuming.' },
  { title: 'Download history', description: 'Find a saved file and open it or its folder.' },
  { title: 'Readable filenames', description: 'Keep original titles and choose how to handle existing files.' },
]
const steps = [
  { title: 'Paste a link', description: 'Add a video or playlist link.' },
  { title: 'Choose a format', description: 'Select the output and save folder.' },
  { title: 'Save your file', description: 'Download, then open the result.' },
]
const faqs = [
  { question: 'What does YDLite need to run?', answer: 'Windows x64, WebView2, yt-dlp and FFmpeg. YDLite checks for the download tools on first launch and can install them for you.' },
  { question: 'Which sites are supported?', answer: 'YDLite uses yt-dlp. Site support depends on the installed version and your access to the video. You can update yt-dlp in Settings.' },
  { question: 'Can I resume a paused download?', answer: 'Yes, if the site supports resuming. YDLite keeps the partial file when you pause.' },
  { question: 'Can I download a playlist?', answer: 'Yes. Choose the entries to save. YDLite downloads them in order and continues if an entry fails.' },
  { question: 'Can I download videos that require a login?', answer: 'You can select your own cookies.txt file in Settings. Your account must have access to the video.' },
  { question: 'Does clearing history delete my files?', answer: 'No. Clearing history removes the records. Your downloaded files stay on disk.' },
]

function handlePreviewKeys(event: KeyboardEvent) {
  let next = activePreview.value
  if (event.key === 'ArrowRight') next = (next + 1) % previews.length
  else if (event.key === 'ArrowLeft') next = (next + previews.length - 1) % previews.length
  else if (event.key === 'Home') next = 0
  else if (event.key === 'End') next = previews.length - 1
  else return
  event.preventDefault()
  activePreview.value = next
  void nextTick(() => document.getElementById(`preview-tab-${next}`)?.focus())
}

function closeMenu(event: KeyboardEvent) {
  if (event.key !== 'Escape' || !menuOpen.value) return
  menuOpen.value = false
  menuButton.value?.focus()
}
</script>

<template>
  <div id="top" class="landing-page" @keydown="closeMenu">
    <svg class="symbol-library" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
      <symbol id="site-brand" viewBox="0 0 64 64"><rect width="64" height="64" rx="16" fill="currentColor" /><g fill="none" stroke="var(--white)" stroke-linecap="round" stroke-linejoin="round"><path d="m20 17 12 12 12-12M32 29v12m-7-7 7 7 7-7" stroke-width="5" /><path d="M19 50h12m6 0h8" stroke-width="4" /></g></symbol>
    </svg>
    <a class="skip-link" href="#main-content">Skip to content</a>
    <header class="site-header">
      <nav class="landing-nav site-width" aria-label="Main navigation">
        <a class="brand" href="#top" aria-label="YDLite home"><svg width="32" height="32" aria-hidden="true"><use href="#site-brand" /></svg><span>YDLite</span></a>
        <button ref="menuButton" class="menu-toggle" type="button" :aria-expanded="menuOpen" aria-controls="site-menu" :aria-label="menuOpen ? 'Close navigation' : 'Open navigation'" @click="menuOpen = !menuOpen"><svg viewBox="0 0 24 24" aria-hidden="true"><path v-if="menuOpen" d="m6 6 12 12M18 6 6 18" /><path v-else d="M4 7h16M4 12h16M4 17h16" /></svg></button>
        <div id="site-menu" class="nav-links" :class="{ 'is-open': menuOpen }">
          <a href="#features" @click="menuOpen = false">Features</a>
          <a href="#workflow" @click="menuOpen = false">How it works</a>
          <a href="#faq" @click="menuOpen = false">FAQ</a>
        </div>
        <a class="button primary nav-download" :href="setupUrl" download>Download for Windows</a>
      </nav>
    </header>

    <main id="main-content">
      <section class="hero-section site-width" aria-labelledby="hero-title">
        <div class="hero-intro">
          <p class="eyebrow">Video and audio downloader</p>
          <h1 id="hero-title">From link<br />to <span>your folder.</span></h1>
          <p class="hero-lede">Download videos and audio in the format you need. Save them where you want.</p>
          <div class="hero-actions">
            <a class="button primary" :href="setupUrl" download>Download for Windows<svg viewBox="0 0 20 20" aria-hidden="true"><path d="M10 3v12m-5-5 5 5 5-5M4 18h12" /></svg></a>
            <a class="text-link" :href="githubUrl" target="_blank" rel="noreferrer noopener">GitHub <span aria-hidden="true">↗</span></a>
          </div>
          <p class="hero-platform">Windows x64 <span>Version 0.2.0</span></p>
          <ul class="hero-facts"><li>No account required</li><li>Open source</li></ul>
        </div>
        <div class="product-preview">
          <div class="preview-tabs" role="tablist" aria-label="Product previews" @keydown="handlePreviewKeys">
            <button v-for="(preview, index) in previews" :id="`preview-tab-${index}`" :key="preview.name" type="button" role="tab" :aria-selected="activePreview === index" :tabindex="activePreview === index ? 0 : -1" aria-controls="product-preview-panel" @click="activePreview = index">{{ preview.name }}</button>
          </div>
          <div id="product-preview-panel" class="hero-stage" role="tabpanel" :aria-labelledby="`preview-tab-${activePreview}`">
            <button class="stage-viewport" type="button" aria-label="Enlarge app screenshot" @click="imageDialog?.showModal()">
              <img v-for="(preview, index) in previews" :key="preview.name" class="hero-shot" :class="{ 'is-active': activePreview === index }" :src="preview.src" :alt="activePreview === index ? preview.alt : ''" :aria-hidden="activePreview !== index" width="1180" height="780" />
              <span class="enlarge-label" aria-hidden="true">↗</span>
            </button>
          </div>
        </div>
      </section>

      <section id="features" class="features-section site-width" aria-labelledby="features-title">
        <div class="section-heading"><p class="eyebrow">Features</p><h2 id="features-title">Control what you save.</h2></div>
        <div class="feature-grid"><article v-for="(feature, index) in features" :key="feature.title"><span class="feature-index" aria-hidden="true">0{{ index + 1 }}</span><h3>{{ feature.title }}</h3><p>{{ feature.description }}</p></article></div>
      </section>

      <section id="workflow" class="workflow-section" aria-labelledby="workflow-title">
        <div class="site-width"><div class="section-heading"><p class="eyebrow">How it works</p><h2 id="workflow-title">Three steps to your file.</h2></div>
          <ol class="workflow-list"><li v-for="(step, index) in steps" :key="step.title"><span class="step" aria-hidden="true">{{ index + 1 }}</span><div><h3>{{ step.title }}</h3><p>{{ step.description }}</p></div></li></ol>
        </div>
      </section>

      <section id="faq" class="faq-section site-width" aria-labelledby="faq-title">
        <div class="section-heading"><p class="eyebrow">FAQ</p><h2 id="faq-title">Before you download.</h2></div>
        <div class="faq-list"><div v-for="(faq, index) in faqs" :key="faq.question" class="faq-item" :class="{ 'is-open': activeFaq === index }">
          <h3><button :id="`faq-question-${index}`" type="button" :aria-expanded="activeFaq === index" :aria-controls="`faq-answer-${index}`" @click="activeFaq = activeFaq === index ? null : index">{{ faq.question }}<svg viewBox="0 0 20 20" aria-hidden="true"><path d="m5 8 5 5 5-5" /></svg></button></h3>
          <div :id="`faq-answer-${index}`" class="faq-answer" :inert="activeFaq !== index" :aria-hidden="activeFaq !== index" role="region" :aria-labelledby="`faq-question-${index}`"><div><p>{{ faq.answer }}</p></div></div>
        </div></div>
      </section>

      <section id="download" class="download-section site-width" aria-labelledby="download-title">
        <div><p class="eyebrow">Windows x64</p><h2 id="download-title">Ready to download?</h2></div>
        <div class="download-options"><div class="download-links"><a class="button primary" :href="setupUrl" download>Download installer</a><a class="button secondary" :href="portableUrl" download>Portable ZIP</a></div><a class="text-link download-alt" :href="msiUrl" download>MSI installer <span aria-hidden="true">↗</span></a></div>
      </section>
    </main>

    <footer class="site-footer site-width"><a class="brand" href="#top"><svg width="24" height="24" aria-hidden="true"><use href="#site-brand" /></svg>YDLite</a><span>MIT license</span><a class="text-link" :href="githubUrl" target="_blank" rel="noreferrer noopener">GitHub <span aria-hidden="true">↗</span></a></footer>
    <dialog ref="imageDialog" class="screenshot-dialog" aria-label="Enlarged app screenshot" @click="($event.target === imageDialog) && imageDialog?.close()"><button class="button secondary" type="button" autofocus @click="imageDialog?.close()">Close preview</button><img :src="previews[activePreview]!.src" :alt="previews[activePreview]!.alt" width="1180" height="780" /></dialog>
  </div>
</template>
