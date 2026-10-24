<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import brandMark from '@/assets/brand-mark.svg'
import './styles/landing.css'

const setupUrl = '/downloads/YDLite_0.2.0_x64-setup.exe'
const portableUrl = '/downloads/YDLite_0.2.0_x64-portable.zip'
const msiUrl = '/downloads/YDLite_0.2.0_x64_en-US.msi'
const githubUrl = 'https://github.com/jonbrown66/ydlite'

const activePreview = ref(0)
const menuOpen = ref(false)
const menuButton = ref<HTMLButtonElement | null>(null)
const imageDialog = ref<HTMLDialogElement | null>(null)
const previews = [
  { name: 'Download', src: '/landing/ydlite-app.png', alt: 'YDLite download page showing a video preview, format choice and an active transfer' },
  { name: 'History', src: '/landing/ydlite-history.png', alt: 'Download history with completed files and folder actions' },
  { name: 'Settings', src: '/landing/ydlite-settings.png', alt: 'Appearance, defaults, file naming and tool settings' },
]

const faqs = [
  { question: 'Do I need to install anything else?', answer: 'YDLite needs yt-dlp and FFmpeg. On first run it checks for them and can install both into the app folder. No account or subscription is involved; WebView2 ships with Windows 10 and 11.' },
  { question: 'Which sites does it work with?', answer: 'Anything yt-dlp supports. Availability depends on the site, your account and the installed yt-dlp version, so updating yt-dlp from Settings is the first thing to try when a site changes.' },
  { question: 'Can I pause and continue a download?', answer: 'Yes. Pausing stops the transfer and keeps the partial file. Starting again continues from where it stopped whenever the site supports resuming, which most do.' },
  { question: 'How are playlists handled?', answer: 'Paste the playlist link, choose the entries you want, then download. Entries run one after another and a failure does not stop the rest.' },
  { question: 'What if a video needs a login?', answer: 'Point Settings at your own cookies.txt file. YDLite passes it to yt-dlp locally. Your account still needs permission to view the media.' },
  { question: 'Does clearing history delete my files?', answer: 'No. History is only a record of what you downloaded. Removing an entry never touches the file on disk.' },
]

// Sections fade in as they enter the viewport. The directive marks each element
// itself, which avoids resolving the component root through $el.
let observer: IntersectionObserver | undefined
/** True once mounting decided that no observation will happen. */
let ready = false
const pending = new Set<Element>()

const vReveal = {
  mounted(element: Element) {
    element.setAttribute('data-reveal', '')
    // Directive hooks run before the component's onMounted, so the collector is
    // always available; the observer picks these up once it exists.
    pending.add(element)
    if (observer) observer.observe(element)
    else if (ready) element.classList.add('revealed')
  },
  unmounted(element: Element) {
    pending.delete(element)
    observer?.unobserve(element)
  },
}

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

function closeMenuOnEscape(event: KeyboardEvent) {
  if (event.key === 'Escape' && menuOpen.value) {
    menuOpen.value = false
    menuButton.value?.focus()
  }
}

onMounted(() => {
  document.addEventListener('keydown', closeMenuOnEscape)
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    ready = true
    for (const target of pending) target.classList.add('revealed')
    return
  }
  if (!('IntersectionObserver' in window)) {
    ready = true
    for (const target of pending) target.classList.add('revealed')
    return
  }
  document.documentElement.classList.add('reveal-active')
  const instance = new IntersectionObserver(entries => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue
      entry.target.classList.add('revealed')
      observer?.unobserve(entry.target)
    }
  }, { rootMargin: '0px 0px -8% 0px', threshold: 0.04 })
  observer = instance
  for (const target of pending) instance.observe(target)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', closeMenuOnEscape)
  observer?.disconnect()
  observer = undefined
  pending.clear()
  document.documentElement.classList.remove('reveal-active')
})
</script>

<template>
  <div id="top" class="landing-page">
    <a class="skip-link" href="#main-content">Skip to content</a>

    <header class="site-header">
      <nav class="landing-nav site-width" aria-label="Main navigation">
        <a class="brand" href="#top" aria-label="YDLite home">
          <span class="brand-mark"><img :src="brandMark" width="30" height="30" alt="" /></span>
          <span>YDLite</span>
        </a>
        <button
          ref="menuButton"
          class="menu-toggle"
          type="button"
          :aria-expanded="menuOpen"
          aria-controls="site-menu"
          :aria-label="menuOpen ? 'Close navigation' : 'Open navigation'"
          @click="menuOpen = !menuOpen"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path v-if="menuOpen" d="m6 6 12 12M18 6 6 18" />
            <path v-else d="M4 7h16M4 12h16M4 17h16" />
          </svg>
        </button>
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
          <p class="eyebrow"><span class="pulse" aria-hidden="true" />A video downloader for Windows</p>
          <h1 id="hero-title">From link<br />to your folder.</h1>
          <p class="hero-lede">
            Paste a link, choose a format, and keep an eye on the transfer. YDLite saves the file
            where you want it and remembers what you downloaded.
          </p>
          <div class="hero-actions">
            <a class="button primary" :href="setupUrl" download>
              Download for Windows
              <span aria-hidden="true">↓</span>
            </a>
            <a class="hero-secondary" :href="githubUrl" target="_blank" rel="noreferrer noopener">View on GitHub ↗</a>
          </div>
          <p class="hero-platform">Windows x64 · v0.2.0 · MIT licence</p>
          <ul class="hero-facts">
            <li>No account</li>
            <li>Portable option</li>
            <li>Files stay local</li>
          </ul>
        </div>

        <div class="product-preview">
          <div class="preview-tabs" role="tablist" aria-label="Product previews" @keydown="handlePreviewKeys">
            <button
              v-for="(preview, index) in previews"
              :id="`preview-tab-${index}`"
              :key="preview.name"
              type="button"
              role="tab"
              :aria-selected="activePreview === index"
              :tabindex="activePreview === index ? 0 : -1"
              aria-controls="product-preview-panel"
              @click="activePreview = index"
            >
              {{ preview.name }}
            </button>
          </div>
          <figure class="hero-stage">
            <button
              id="product-preview-panel"
              class="stage-viewport"
              role="tabpanel"
              :aria-labelledby="`preview-tab-${activePreview}`"
              type="button"
              title="Enlarge screenshot"
              @click="imageDialog?.showModal()"
            >
              <img
                class="hero-shot"
                :src="previews[activePreview]!.src"
                :alt="previews[activePreview]!.alt"
                width="1180"
                height="780"
              />
              <span class="enlarge-label">Enlarge <span aria-hidden="true">↗</span></span>
            </button>
          </figure>
          <p class="preview-caption">Actual app interface with example data</p>
        </div>
      </section>

      <section id="features" class="features-section site-width">
        <div class="section-topline">
          <span>Made for one job</span>
          <span>01 / Features</span>
        </div>
        <div class="feature-grid">
          <article v-reveal>
            <span class="feature-index">01</span>
            <h2>Choose before you wait.</h2>
            <p>Best available, up to 720p, MP3 audio, or a specific format with its own audio track. The estimated size appears before the transfer starts.</p>
          </article>
          <article v-reveal>
            <span class="feature-index">02</span>
            <h2>Pause without losing progress.</h2>
            <p>Stop a transfer and start it again later. The partial file is kept, so work already done is not repeated when the site allows resuming.</p>
          </article>
          <article v-reveal>
            <span class="feature-index">03</span>
            <h2>Find what you saved.</h2>
            <p>Every finished download is listed with its path. Search by name or location, then open the file or its folder directly.</p>
          </article>
          <article v-reveal>
            <span class="feature-index">04</span>
            <h2>Names you can read.</h2>
            <p>Chinese titles survive intact. A short media and format tag keeps unrelated files from colliding, and duplicates can skip, number or replace.</p>
          </article>
        </div>
      </section>

      <section id="workflow" class="workflow-section site-width">
        <div class="section-topline">
          <span>A short path to your file</span>
          <span>02 / How it works</span>
        </div>
        <ol class="workflow-list">
          <li v-reveal>
            <span class="step">1</span>
            <h2>Paste a link</h2>
            <p>One video, one track, or a playlist. YDLite reads it and shows what it found.</p>
          </li>
          <li v-reveal>
            <span class="step">2</span>
            <h2>Choose the output</h2>
            <p>Pick a format, select playlist entries, and confirm the folder.</p>
          </li>
          <li v-reveal>
            <span class="step">3</span>
            <h2>Download and open</h2>
            <p>Watch the progress bar, pause if you need to, then open the result.</p>
          </li>
        </ol>
      </section>

      <section id="faq" class="faq-section site-width">
        <div>
          <p class="eyebrow">Before you start</p>
          <h2>A few honest answers.</h2>
          <p>Download support follows yt-dlp and whatever access your account already has.</p>
        </div>
        <div class="faq-list">
          <details v-for="faq in faqs" :key="faq.question">
            <summary>{{ faq.question }}<span aria-hidden="true">+</span></summary>
            <p>{{ faq.answer }}</p>
          </details>
        </div>
      </section>

      <section id="download" class="download-section site-width">
        <div>
          <p class="eyebrow">YDLite for Windows</p>
          <h2>Keep your next download simple.</h2>
          <p class="download-note">Setup is the usual choice. The portable build is a folder you can move or delete.</p>
        </div>
        <div class="download-links">
          <a class="button primary" :href="setupUrl" download>Installer · EXE</a>
          <a class="button secondary" :href="portableUrl" download>Portable · ZIP</a>
          <a class="download-alt" :href="msiUrl" download>MSI for managed installs</a>
        </div>
      </section>
    </main>

    <footer class="site-footer site-width">
      <a class="brand" href="#top">
        <span class="brand-mark"><img :src="brandMark" width="24" height="24" alt="" /></span>
        YDLite
      </a>
      <span>Video and audio downloads. Saved locally.</span>
      <a :href="githubUrl" target="_blank" rel="noreferrer noopener">GitHub ↗</a>
    </footer>

    <dialog
      ref="imageDialog"
      class="screenshot-dialog"
      aria-label="Enlarged app screenshot"
      @click="($event.target === imageDialog) && imageDialog?.close()"
    >
      <button class="button secondary" type="button" autofocus @click="imageDialog?.close()">Close preview</button>
      <img :src="previews[activePreview]!.src" :alt="previews[activePreview]!.alt" width="1180" height="780" />
    </dialog>
  </div>
</template>
