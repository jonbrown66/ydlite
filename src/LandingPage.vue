<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { gsap } from 'gsap'

const downloadUrl = '/downloads/YDLite_0.1.0_x64-setup.exe'
const msiUrl = '/downloads/YDLite_0.1.0_x64_en-US.msi'
const githubUrl = 'https://github.com/jonbrown66/ydlite'

const page = ref<HTMLElement | null>(null)
let context: gsap.Context | null = null
let observers: IntersectionObserver[] = []

const bentoCards = [
  {
    title: 'Parse first',
    body: 'Paste a URL and inspect title, author, duration, thumbnail, playlist entries, and available formats before downloading.',
    label: 'Metadata',
    className: 'wide',
  },
  {
    title: 'Compatible MP4',
    body: 'Defaults prefer MP4 video with M4A/AAC audio, avoiding common Windows playback issues caused by Opus in MP4.',
    label: 'Playback',
    className: 'tall',
  },
  {
    title: 'Local tools',
    body: 'Uses local yt-dlp and ffmpeg. Checks and installs are manual, so app startup stays fast.',
    label: 'Local',
    className: '',
  },
  {
    title: 'Clear progress',
    body: 'Shows percent, speed, ETA, total size, finished path, and expandable yt-dlp logs.',
    label: 'Feedback',
    className: '',
  },
  {
    title: 'Cookies only when needed',
    body: 'Use cookies.txt for private or login-gated links without making it part of the normal flow.',
    label: 'Access',
    className: '',
  },
  {
    title: 'Playlist control',
    body: 'Select individual playlist items and process them with a simple visible queue.',
    label: 'Batch',
    className: 'wide',
  },
]

const steps = [
  ['01', 'Paste', 'Drop in a supported URL.'],
  ['02', 'Parse', 'Confirm the video and format.'],
  ['03', 'Download', 'Choose a folder and track progress.'],
]

onMounted(() => {
  const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  context = gsap.context(() => {
    if (reduceMotion) {
      gsap.set('[data-animate], [data-reveal]', { autoAlpha: 1, y: 0 })
      return
    }

    gsap
      .timeline({ defaults: { ease: 'power3.out' } })
      .from('[data-animate="nav"]', { y: -14, autoAlpha: 0, duration: 0.45 })
      .from('[data-animate="hero"] > *', { y: 28, autoAlpha: 0, duration: 0.62, stagger: 0.07 }, '-=0.1')
      .from('[data-animate="mockup"]', { y: 34, autoAlpha: 0, duration: 0.7 }, '-=0.42')
      .from('.mock-line, .mock-chip, .mock-progress span', { scaleX: 0, transformOrigin: 'left center', duration: 0.42, stagger: 0.035 }, '-=0.32')

    const items = Array.from(page.value?.querySelectorAll<HTMLElement>('[data-reveal]') || [])
    items.forEach((element) => {
      gsap.set(element, { y: 24, autoAlpha: 0 })
      const observer = new IntersectionObserver(
        ([entry]) => {
          if (!entry.isIntersecting) return
          gsap.to(element, { y: 0, autoAlpha: 1, duration: 0.5, ease: 'power3.out' })
          observer.disconnect()
        },
        { threshold: 0.16 },
      )
      observer.observe(element)
      observers.push(observer)
    })
  }, page.value || undefined)
})

onBeforeUnmount(() => {
  observers.forEach((observer) => observer.disconnect())
  observers = []
  context?.revert()
})
</script>

<template>
  <main ref="page" class="landing-page">
    <nav class="landing-nav" data-animate="nav">
      <a class="brand" href="#top" aria-label="YDLite home">
        <span class="brand-mark">Y</span>
        <span>YDLite</span>
      </a>
      <div class="nav-links">
        <a href="#features">Features</a>
        <a href="#download">Download</a>
        <a :href="githubUrl" target="_blank" rel="noreferrer">GitHub</a>
      </div>
      <a class="nav-download" :href="downloadUrl" download>Download</a>
    </nav>

    <section id="top" class="hero-section">
      <div class="hero-copy" data-animate="hero">
        <p class="eyebrow">Windows desktop downloader</p>
        <h1>YDLite keeps video downloads local and predictable.</h1>
        <p class="hero-lede">
          A compact Tauri app for links supported by yt-dlp. Parse the link, confirm the output,
          download with visible progress, and keep the workflow on your machine.
        </p>
        <div class="hero-actions">
          <a class="button primary" :href="downloadUrl" download>Download for Windows</a>
          <a class="button ghost" :href="githubUrl" target="_blank" rel="noreferrer">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path fill="currentColor" d="M12 .5a12 12 0 0 0-3.79 23.39c.6.11.82-.26.82-.58v-2.03c-3.34.73-4.04-1.41-4.04-1.41-.55-1.38-1.34-1.75-1.34-1.75-1.09-.75.08-.74.08-.74 1.21.09 1.85 1.25 1.85 1.25 1.07 1.84 2.82 1.31 3.51 1 .11-.78.42-1.31.76-1.61-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.12-.31-.54-1.53.12-3.18 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 0 1 6.01 0c2.29-1.55 3.3-1.23 3.3-1.23.66 1.65.24 2.87.12 3.18.77.84 1.24 1.91 1.24 3.22 0 4.61-2.81 5.63-5.48 5.93.43.37.81 1.1.81 2.22v3.29c0 .32.22.7.83.58A12 12 0 0 0 12 .5Z" />
            </svg>
            GitHub
          </a>
        </div>
        <div class="trust-row">
          <span>No cloud queue</span>
          <span>Manual dependency checks</span>
          <span>MP4 compatibility first</span>
        </div>
      </div>

      <div class="product-mockup" data-animate="mockup" aria-label="YDLite app preview">
        <div class="mock-titlebar">
          <span></span>
          <span></span>
          <strong>YDLite</strong>
        </div>
        <div class="mock-input">
          <span>https://video.example/watch...</span>
          <b>Parse</b>
        </div>
        <div class="mock-card">
          <div class="mock-thumb"></div>
          <div>
            <div class="mock-line wide"></div>
            <div class="mock-line mid"></div>
            <div class="mock-chip">mp4 + m4a</div>
          </div>
        </div>
        <div class="mock-progress"><span></span></div>
        <div class="mock-stats">
          <span>68.4%</span>
          <span>4.2 MB/s</span>
          <span>ETA 00:18</span>
        </div>
      </div>
    </section>

    <section id="features" class="section-shell">
      <div class="section-heading" data-reveal>
        <p class="eyebrow">What it does</p>
        <h2>Focused controls, clear outcomes.</h2>
      </div>
      <div class="bento-grid">
        <article v-for="card in bentoCards" :key="card.title" class="bento-card" :class="card.className" data-reveal>
          <span>{{ card.label }}</span>
          <h3>{{ card.title }}</h3>
          <p>{{ card.body }}</p>
        </article>
      </div>
    </section>

    <section class="flow-section" data-reveal>
      <div>
        <p class="eyebrow">Flow</p>
        <h2>Three steps, no guessing.</h2>
      </div>
      <div class="step-grid">
        <article v-for="step in steps" :key="step[0]" class="step-card">
          <span>{{ step[0] }}</span>
          <h3>{{ step[1] }}</h3>
          <p>{{ step[2] }}</p>
        </article>
      </div>
    </section>

    <section id="download" class="download-section" data-reveal>
      <div>
        <p class="eyebrow">Download</p>
        <h2>Windows build, ready to try.</h2>
        <p>
          The installer includes the latest app changes. Use the in-app Check button only when you want
          to verify yt-dlp and ffmpeg.
        </p>
      </div>
      <div class="download-card">
        <span>YDLite 0.1.0</span>
        <a class="button primary" :href="downloadUrl" download>Download .exe</a>
        <a class="text-link" :href="msiUrl" download>Download MSI</a>
      </div>
    </section>

    <footer class="site-footer">
      <div>
        <strong>YDLite</strong>
        <span>Local-first video downloads for Windows.</span>
      </div>
      <div class="footer-links">
        <a :href="githubUrl" target="_blank" rel="noreferrer" aria-label="YDLite on GitHub">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path fill="currentColor" d="M12 .5a12 12 0 0 0-3.79 23.39c.6.11.82-.26.82-.58v-2.03c-3.34.73-4.04-1.41-4.04-1.41-.55-1.38-1.34-1.75-1.34-1.75-1.09-.75.08-.74.08-.74 1.21.09 1.85 1.25 1.85 1.25 1.07 1.84 2.82 1.31 3.51 1 .11-.78.42-1.31.76-1.61-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.12-.31-.54-1.53.12-3.18 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 0 1 6.01 0c2.29-1.55 3.3-1.23 3.3-1.23.66 1.65.24 2.87.12 3.18.77.84 1.24 1.91 1.24 3.22 0 4.61-2.81 5.63-5.48 5.93.43.37.81 1.1.81 2.22v3.29c0 .32.22.7.83.58A12 12 0 0 0 12 .5Z" />
          </svg>
          GitHub
        </a>
        <a href="#download">Download</a>
      </div>
    </footer>
  </main>
</template>

<style>
:root {
  --page: #f4efe7;
  --surface: #fffdfa;
  --surface-muted: #fbf6ef;
  --line: #ddd3c7;
  --ink: #2d2f34;
  --muted: #696f78;
  --soft: #9da3af;
  --blue: #2e77e5;
  --green: #4f7458;
  --rose: #a24e73;
  --yellow: #8a741f;
  --shadow: 0 18px 55px rgba(42, 35, 26, 0.08), 0 2px 8px rgba(42, 35, 26, 0.05);
  color: var(--ink);
  background: var(--page);
  font-family: "Aptos", "Segoe UI", system-ui, sans-serif;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
}

* {
  box-sizing: border-box;
}

html {
  scroll-behavior: smooth;
  background: var(--page);
}

body {
  margin: 0;
  background: var(--page);
}

a {
  color: inherit;
  text-decoration: none;
}

svg {
  width: 17px;
  height: 17px;
}

.landing-page {
  min-height: 100vh;
  overflow-x: hidden;
  background: var(--page);
}

.landing-nav,
.hero-section,
.section-shell,
.flow-section,
.download-section,
.site-footer {
  width: min(1160px, calc(100% - 40px));
  margin: 0 auto;
}

.landing-nav {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  min-height: 74px;
  gap: 24px;
}

.brand,
.nav-links,
.hero-actions,
.trust-row,
.footer-links,
.button,
.nav-download {
  display: inline-flex;
  align-items: center;
}

.brand {
  gap: 10px;
  width: fit-content;
  font-weight: 850;
}

.brand-mark {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: var(--ink);
  color: #fffdfa;
  font-size: 14px;
}

.nav-links {
  gap: 26px;
  color: var(--muted);
  font-size: 14px;
  font-weight: 700;
}

.nav-links a:hover,
.text-link:hover,
.footer-links a:hover {
  color: var(--blue);
}

.nav-download {
  justify-self: end;
  min-height: 38px;
  padding: 0 18px;
  border-radius: 999px;
  background: var(--ink);
  color: #fffdfa;
  font-size: 14px;
  font-weight: 850;
}

.hero-section {
  display: grid;
  grid-template-columns: minmax(0, 0.95fr) minmax(410px, 1.05fr);
  gap: clamp(36px, 6vw, 76px);
  align-items: center;
  padding: clamp(54px, 7vw, 92px) 0 58px;
}

.hero-copy {
  max-width: 650px;
}

.eyebrow {
  margin: 0;
  color: var(--blue);
  font-size: 13px;
  font-weight: 900;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.hero-copy h1,
.section-heading h2,
.flow-section h2,
.download-section h2 {
  margin: 18px 0 0;
  color: var(--ink);
  font-family: Georgia, "Times New Roman", serif;
  font-weight: 720;
  letter-spacing: 0;
}

.hero-copy h1 {
  max-width: 660px;
  font-size: clamp(46px, 6.8vw, 80px);
  line-height: 1.02;
}

.hero-lede {
  max-width: 590px;
  margin: 22px 0 0;
  color: var(--muted);
  font-size: 18px;
  line-height: 1.7;
}

.hero-actions {
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 30px;
}

.button {
  justify-content: center;
  gap: 8px;
  min-height: 46px;
  padding: 0 22px;
  border: 1px solid var(--ink);
  border-radius: 999px;
  color: var(--ink);
  font-size: 14px;
  font-weight: 850;
  transition: transform 160ms ease-out, background 160ms ease-out, color 160ms ease-out;
}

.button.primary {
  background: var(--ink);
  color: #fffdfa;
}

.button.ghost {
  background: transparent;
}

.button:hover,
.nav-download:hover {
  transform: translateY(-2px);
}

.trust-row {
  gap: 18px;
  flex-wrap: wrap;
  margin-top: 22px;
  color: var(--soft);
  font-size: 13px;
  font-weight: 700;
}

.trust-row span::before {
  content: "";
  display: inline-block;
  width: 7px;
  height: 7px;
  margin-right: 8px;
  border-radius: 50%;
  background: var(--green);
}

.product-mockup,
.bento-card,
.step-card,
.download-card {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: var(--shadow);
}

.product-mockup {
  padding: 18px;
}

.mock-titlebar {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 28px;
  color: var(--soft);
  font-size: 12px;
}

.mock-titlebar span {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: #e3d8cb;
}

.mock-titlebar strong {
  margin-left: auto;
}

.mock-input {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 54px;
  margin-top: 14px;
  padding: 8px 8px 8px 18px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--soft);
  font-size: 13px;
}

.mock-input b {
  padding: 10px 16px;
  border-radius: 999px;
  background: var(--ink);
  color: #fffdfa;
  font-size: 12px;
}

.mock-card {
  display: grid;
  grid-template-columns: 132px minmax(0, 1fr);
  gap: 16px;
  align-items: center;
  margin-top: 18px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
}

.mock-thumb {
  aspect-ratio: 16 / 10;
  border-radius: 7px;
  background: linear-gradient(135deg, rgba(46, 119, 229, 0.16), transparent 58%), #efc0cf;
}

.mock-line {
  height: 10px;
  margin-bottom: 13px;
  border-radius: 999px;
  background: #e6ded4;
}

.mock-line.wide {
  width: 88%;
}

.mock-line.mid {
  width: 62%;
}

.mock-chip {
  width: fit-content;
  padding: 6px 10px;
  border-radius: 999px;
  background: #f5edc9;
  color: var(--yellow);
  font-size: 12px;
  font-weight: 850;
}

.mock-progress {
  height: 10px;
  margin-top: 20px;
  border-radius: 999px;
  background: #e6ded4;
  overflow: hidden;
}

.mock-progress span {
  display: block;
  width: 68%;
  height: 100%;
  border-radius: inherit;
  background: var(--blue);
}

.mock-stats {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-top: 12px;
  color: var(--muted);
  font-size: 12px;
  font-weight: 800;
}

.section-shell,
.flow-section,
.download-section {
  padding: 72px 0;
}

.section-heading {
  max-width: 680px;
}

.section-heading h2,
.flow-section h2,
.download-section h2 {
  font-size: clamp(34px, 4.8vw, 56px);
  line-height: 1.08;
}

.bento-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  grid-auto-rows: minmax(190px, auto);
  gap: 16px;
  margin-top: 32px;
}

.bento-card {
  padding: 24px;
}

.bento-card.wide {
  grid-column: span 2;
}

.bento-card.tall {
  grid-row: span 2;
}

.bento-card span,
.download-card > span,
.step-card span {
  color: var(--blue);
  font-size: 12px;
  font-weight: 900;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.bento-card h3,
.step-card h3 {
  margin: 18px 0 0;
  font-family: Georgia, "Times New Roman", serif;
  font-size: 24px;
}

.bento-card p,
.step-card p,
.download-section p {
  margin: 12px 0 0;
  color: var(--muted);
  font-size: 15px;
  line-height: 1.66;
}

.flow-section {
  display: grid;
  grid-template-columns: 0.72fr 1.28fr;
  gap: 40px;
}

.step-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
}

.step-card {
  padding: 24px;
}

.download-section {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  gap: 44px;
  align-items: center;
}

.download-card {
  padding: 26px;
}

.download-card .button {
  width: 100%;
  margin-top: 20px;
}

.text-link {
  display: block;
  width: fit-content;
  margin: 16px auto 0;
  color: var(--blue);
  font-size: 14px;
  font-weight: 850;
}

.site-footer {
  display: flex;
  justify-content: space-between;
  gap: 24px;
  align-items: center;
  padding: 28px 0 44px;
  border-top: 1px solid var(--line);
  color: var(--muted);
}

.site-footer strong,
.site-footer span {
  display: block;
}

.site-footer strong {
  color: var(--ink);
}

.site-footer span {
  margin-top: 4px;
  font-size: 13px;
}

.footer-links {
  gap: 18px;
  font-size: 14px;
  font-weight: 800;
}

.footer-links a {
  display: inline-flex;
  align-items: center;
  gap: 7px;
}

@media (max-width: 980px) {
  .landing-nav {
    grid-template-columns: 1fr auto;
  }

  .nav-links {
    display: none;
  }

  .hero-section,
  .flow-section,
  .download-section {
    grid-template-columns: 1fr;
  }

  .bento-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .step-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 680px) {
  .landing-nav,
  .hero-section,
  .section-shell,
  .flow-section,
  .download-section,
  .site-footer {
    width: min(100% - 28px, 1160px);
  }

  .nav-download {
    display: none;
  }

  .hero-copy h1 {
    font-size: clamp(40px, 13vw, 62px);
  }

  .hero-actions .button,
  .download-card {
    width: 100%;
  }

  .product-mockup {
    padding: 12px;
  }

  .mock-card,
  .bento-grid {
    grid-template-columns: 1fr;
  }

  .bento-card.wide,
  .bento-card.tall {
    grid-column: auto;
    grid-row: auto;
  }

  .site-footer {
    align-items: flex-start;
    flex-direction: column;
  }
}

@media (prefers-reduced-motion: reduce) {
  html {
    scroll-behavior: auto;
  }

  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
</style>
