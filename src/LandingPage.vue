<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { gsap } from 'gsap'

const downloadUrl = '/downloads/YDLite_0.1.0_x64-setup.exe'
const msiUrl = '/downloads/YDLite_0.1.0_x64_en-US.msi'
const githubUrl = 'https://github.com/jonbrown66/ydlite'
const heroImageUrl = '/landing/ydlite-app.png'

const page = ref<HTMLElement | null>(null)
let context: gsap.Context | null = null
let observers: IntersectionObserver[] = []

const bentoCards = [
  {
    key: 'parse',
    icon: 'link',
    label: 'Parse',
    title: 'Preview before download',
    body: 'Title, duration, source, thumbnail, playlist entries, and formats.',
  },
  {
    key: 'mp4',
    icon: 'play',
    label: 'MP4',
    title: 'Windows-friendly output',
    body: 'Prefers MP4 video with M4A/AAC audio to avoid Opus playback issues.',
  },
  {
    key: 'local',
    icon: 'shield',
    label: 'Local',
    title: 'No cloud queue',
    body: 'yt-dlp and ffmpeg run on your PC.',
  },
  {
    key: 'speed',
    icon: 'bolt',
    label: 'Fast',
    title: 'Opens quickly',
    body: 'No default dependency scan on launch.',
  },
  {
    key: 'cookies',
    icon: 'cookie',
    label: 'Access',
    title: 'Cookies when needed',
    body: 'Keep private-link support out of the primary path.',
  },
  {
    key: 'queue',
    icon: 'list',
    label: 'Queue',
    title: 'Playlist control',
    body: 'Pick items, then watch a simple serial queue.',
  },
  {
    key: 'logs',
    icon: 'terminal',
    label: 'Logs',
    title: 'Readable progress',
    body: 'Percent, speed, ETA, finished file, and expandable details.',
  },
]

const statItems = [
  ['3.6 MB', 'installer'],
  ['Manual', 'tool checks'],
  ['AAC', 'default audio'],
]

function iconPath(name: string) {
  const icons: Record<string, string> = {
    link: 'M10.6 13.4a1 1 0 0 1 0-1.4l3.4-3.4a3 3 0 1 1 4.2 4.2l-1.2 1.2a1 1 0 1 1-1.4-1.4l1.2-1.2a1 1 0 0 0-1.4-1.4L12 13.4a1 1 0 0 1-1.4 0Zm2.8-2.8a1 1 0 0 1 0 1.4L10 15.4a3 3 0 1 1-4.2-4.2L7 10a1 1 0 1 1 1.4 1.4l-1.2 1.2A1 1 0 1 0 8.6 14l3.4-3.4a1 1 0 0 1 1.4 0Z',
    play: 'M8 5.8c0-.8.9-1.3 1.6-.9l7 4.2c.7.4.7 1.4 0 1.8l-7 4.2A1.1 1.1 0 0 1 8 14.2V5.8Z',
    shield: 'M12 3 5.5 5.6v5.1c0 4.1 2.8 7.9 6.5 8.9 3.7-1 6.5-4.8 6.5-8.9V5.6L12 3Zm2.9 6.6-3.4 3.4-1.5-1.5a1 1 0 0 0-1.4 1.4l2.2 2.2c.4.4 1 .4 1.4 0l4.1-4.1a1 1 0 0 0-1.4-1.4Z',
    bolt: 'M13 2 5 13h6l-1 9 8-12h-6l1-8Z',
    cookie: 'M18.5 10.2A6.9 6.9 0 1 1 13.8 5a2.5 2.5 0 0 0 3.3 3.2 2.5 2.5 0 0 0 1.4 2ZM9 10.2a1 1 0 1 0 0-2 1 1 0 0 0 0 2Zm4.2 5.2a1 1 0 1 0 0-2 1 1 0 0 0 0 2Zm-4.7 1a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z',
    list: 'M7 6h12v2H7V6Zm0 5h12v2H7v-2Zm0 5h12v2H7v-2ZM4 6h1v2H4V6Zm0 5h1v2H4v-2Zm0 5h1v2H4v-2Z',
    terminal: 'M4 5h16v14H4V5Zm2 2v10h12V7H6Zm2 2.2 2.2 1.8L8 12.8l1.2 1.4 3.8-3.2-3.8-3.2L8 9.2Zm5 4.8h4v-2h-4v2Z',
    arrowUp: 'M12 4 5.5 10.5l1.4 1.4L11 7.8V20h2V7.8l4.1 4.1 1.4-1.4L12 4Z',
    app: 'M7 3.8h10a3.2 3.2 0 0 1 3.2 3.2v10a3.2 3.2 0 0 1-3.2 3.2H7A3.2 3.2 0 0 1 3.8 17V7A3.2 3.2 0 0 1 7 3.8Zm0 2A1.2 1.2 0 0 0 5.8 7v10A1.2 1.2 0 0 0 7 18.2h10a1.2 1.2 0 0 0 1.2-1.2V7A1.2 1.2 0 0 0 17 5.8H7Zm3.1 3.1 5.2 3.1-5.2 3.1V8.9Z',
  }
  return icons[name] || icons.link
}

onMounted(() => {
  const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  context = gsap.context(() => {
    if (reduceMotion) {
      gsap.set('[data-animate], [data-reveal], .hero-media', { autoAlpha: 1, y: 0, scale: 1 })
      return
    }

    const intro = gsap
      .timeline({ defaults: { ease: 'power3.out' } })
      .from('[data-animate="nav"]', { y: -14, autoAlpha: 0, duration: 0.42 })
      .from('[data-animate="hero"] > *', { y: 26, autoAlpha: 0, duration: 0.58, stagger: 0.06 }, '-=0.08')
      .from('[data-animate="media"]', { y: 34, autoAlpha: 0, scale: 0.98, duration: 0.72 }, '-=0.38')

    intro.call(() => {
      gsap.to('.hero-media', {
        y: -8,
        duration: 3.2,
        repeat: -1,
        yoyo: true,
        ease: 'sine.inOut',
      })
    })

    const items = Array.from(page.value?.querySelectorAll<HTMLElement>('[data-reveal]') || [])
    items.forEach((element) => {
      gsap.set(element, { y: 22, autoAlpha: 0 })
      if (element.getBoundingClientRect().top < window.innerHeight * 1.35) {
        gsap.to(element, { y: 0, autoAlpha: 1, duration: 0.48, ease: 'power3.out', delay: 0.12 })
        return
      }

      const observer = new IntersectionObserver(
        ([entry]) => {
          if (!entry.isIntersecting) return
          gsap.to(element, { y: 0, autoAlpha: 1, duration: 0.48, ease: 'power3.out' })
          observer.disconnect()
        },
        { threshold: 0.14 },
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
        <span class="brand-mark">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path fill="currentColor" :d="iconPath('app')" />
          </svg>
        </span>
        <span>YDLite</span>
      </a>

      <div class="nav-links" aria-label="Primary navigation">
        <a href="#top">Demo</a>
        <a href="#features">Features</a>
        <a href="#download">Download</a>
      </div>

      <div class="nav-actions">
        <a class="icon-link github-link" :href="githubUrl" target="_blank" rel="noreferrer" aria-label="GitHub repository">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path fill="currentColor" d="M12 .5a12 12 0 0 0-3.79 23.39c.6.11.82-.26.82-.58v-2.03c-3.34.73-4.04-1.41-4.04-1.41-.55-1.38-1.34-1.75-1.34-1.75-1.09-.75.08-.74.08-.74 1.21.09 1.85 1.25 1.85 1.25 1.07 1.84 2.82 1.31 3.51 1 .11-.78.42-1.31.76-1.61-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.12-.31-.54-1.53.12-3.18 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 0 1 6.01 0c2.29-1.55 3.3-1.23 3.3-1.23.66 1.65.24 2.87.12 3.18.77.84 1.24 1.91 1.24 3.22 0 4.61-2.81 5.63-5.48 5.93.43.37.81 1.1.81 2.22v3.29c0 .32.22.7.83.58A12 12 0 0 0 12 .5Z" />
          </svg>
        </a>
        <a class="nav-download" :href="downloadUrl" download>Download</a>
      </div>
    </nav>

    <section id="top" class="hero-section">
      <div class="hero-copy" data-animate="hero">
        <p class="eyebrow">Local video downloads</p>
        <h1>Paste. Preview. Download.</h1>
        <p class="hero-lede">
          YDLite wraps yt-dlp in a fast Windows app with visible progress, compatible MP4 defaults,
          and tools that stay on your machine.
        </p>
        <div class="hero-actions">
          <a class="button primary" :href="downloadUrl" download>Windows setup</a>
          <a class="button ghost" href="#features">Features</a>
        </div>
        <div class="stat-strip" aria-label="Product highlights">
          <div v-for="item in statItems" :key="item[1]">
            <strong>{{ item[0] }}</strong>
            <span>{{ item[1] }}</span>
          </div>
        </div>
      </div>

      <figure class="hero-media" data-animate="media">
        <img class="hero-shot" :src="heroImageUrl" alt="YDLite Windows app with a URL input, manual tool checks, and parse button" />
      </figure>
    </section>

    <section id="features" class="section-shell">
      <div class="section-heading" data-reveal>
        <p class="eyebrow">Clear by default</p>
        <h2>Everything important is visible.</h2>
      </div>

      <div class="bento-grid">
        <article v-for="card in bentoCards" :key="card.key" class="bento-card" :class="`card-${card.key}`" data-reveal>
          <div class="card-icon">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path fill="currentColor" :d="iconPath(card.icon)" />
            </svg>
          </div>
          <span>{{ card.label }}</span>
          <h3>{{ card.title }}</h3>
          <p>{{ card.body }}</p>
        </article>
      </div>
    </section>

    <section id="download" class="download-section" data-reveal>
      <div class="download-copy">
        <p class="eyebrow">Windows build</p>
        <h2>Small installer. Local workflow.</h2>
      </div>
      <div class="download-card">
        <a class="button download-option" :href="downloadUrl" download>.exe</a>
        <a class="button download-option" :href="msiUrl" download>.msi</a>
      </div>
    </section>

    <footer class="site-footer">
      <strong>YDLite</strong>
      <div class="footer-links">
        <a :href="githubUrl" target="_blank" rel="noreferrer" aria-label="GitHub repository">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path fill="currentColor" d="M12 .5a12 12 0 0 0-3.79 23.39c.6.11.82-.26.82-.58v-2.03c-3.34.73-4.04-1.41-4.04-1.41-.55-1.38-1.34-1.75-1.34-1.75-1.09-.75.08-.74.08-.74 1.21.09 1.85 1.25 1.85 1.25 1.07 1.84 2.82 1.31 3.51 1 .11-.78.42-1.31.76-1.61-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.12-.31-.54-1.53.12-3.18 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 0 1 6.01 0c2.29-1.55 3.3-1.23 3.3-1.23.66 1.65.24 2.87.12 3.18.77.84 1.24 1.91 1.24 3.22 0 4.61-2.81 5.63-5.48 5.93.43.37.81 1.1.81 2.22v3.29c0 .32.22.7.83.58A12 12 0 0 0 12 .5Z" />
          </svg>
        </a>
        <a class="top-link" href="#top" aria-label="Back to top">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path fill="currentColor" :d="iconPath('arrowUp')" />
          </svg>
        </a>
      </div>
    </footer>
  </main>
</template>

<style>
:root {
  --page: #f4efe7;
  --surface: #fffdfa;
  --surface-2: #fbf6ef;
  --line: #d9d0c4;
  --ink: #2d2f34;
  --muted: #687078;
  --soft: #a4a9b1;
  --blue: #2e77e5;
  --green: #4f7458;
  --rose: #a24e73;
  --yellow: #8a741f;
  --shadow: 0 18px 54px rgba(41, 34, 25, 0.08), 0 2px 8px rgba(41, 34, 25, 0.04);
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

button {
  font: inherit;
}

svg {
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
}

.landing-page {
  min-height: 100vh;
  overflow-x: hidden;
  background: var(--page);
}

.landing-nav,
.hero-section,
.section-shell,
.download-section,
.site-footer {
  width: min(1160px, calc(100% - 40px));
  margin: 0 auto;
}

.landing-nav {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  min-height: 72px;
  gap: 20px;
}

.brand,
.nav-links,
.nav-actions,
.hero-actions,
.button,
.icon-link,
.stat-strip,
.footer-links {
  display: inline-flex;
  align-items: center;
}

.brand {
  gap: 10px;
  justify-self: start;
  font-weight: 850;
}

.brand-mark {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: var(--ink);
  color: var(--surface);
}

.brand-mark svg {
  width: 17px;
  height: 17px;
}

.nav-links {
  justify-self: center;
  gap: 22px;
  min-height: 40px;
  padding: 0 16px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: color-mix(in srgb, var(--surface) 82%, transparent);
  color: var(--muted);
  font-size: 13px;
  font-weight: 850;
}

.nav-links a {
  transition: color 140ms ease-out;
}

.nav-links a:hover {
  color: var(--blue);
}

.nav-actions {
  justify-self: end;
  gap: 10px;
  align-items: center;
}

.icon-link {
  justify-content: center;
  width: 40px;
  height: 40px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink);
  transition: transform 160ms ease-out, border-color 160ms ease-out;
}

.github-link svg {
  width: 20px;
  height: 20px;
  transform: translateY(-0.5px);
}

.nav-download {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 40px;
  padding: 0 18px;
  border-radius: 999px;
  background: var(--ink);
  color: var(--surface);
  font-size: 14px;
  font-weight: 850;
  transition: transform 160ms ease-out;
}

.icon-link:hover,
.nav-download:hover,
.button:hover {
  transform: translateY(-2px);
}

.hero-section {
  display: grid;
  grid-template-columns: minmax(0, 0.9fr) minmax(430px, 1.1fr);
  gap: clamp(34px, 6vw, 76px);
  align-items: center;
  padding: clamp(46px, 7vw, 86px) 0 54px;
}

.hero-copy {
  max-width: 650px;
}

.eyebrow {
  margin: 0;
  color: var(--blue);
  font-size: 12px;
  font-weight: 900;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.hero-copy h1,
.section-heading h2,
.download-section h2 {
  margin: 16px 0 0;
  font-family: Georgia, "Times New Roman", serif;
  font-weight: 720;
  letter-spacing: 0;
}

.hero-copy h1 {
  max-width: 620px;
  font-size: clamp(54px, 7vw, 86px);
  line-height: 0.96;
}

.hero-lede {
  max-width: 550px;
  margin: 22px 0 0;
  color: var(--muted);
  font-size: 17px;
  line-height: 1.64;
}

.hero-actions {
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 30px;
}

.button {
  justify-content: center;
  min-height: 46px;
  padding: 0 22px;
  border: 1px solid var(--ink);
  border-radius: 999px;
  color: var(--ink);
  font-size: 14px;
  font-weight: 850;
  transition: transform 160ms ease-out, background 160ms ease-out;
}

.button.primary {
  background: var(--ink);
  color: var(--surface);
}

.button.ghost {
  background: transparent;
}

.stat-strip {
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 26px;
}

.stat-strip div {
  min-width: 112px;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
}

.stat-strip strong,
.stat-strip span {
  display: block;
}

.stat-strip strong {
  font-size: 15px;
}

.stat-strip span {
  margin-top: 2px;
  color: var(--muted);
  font-size: 12px;
  font-weight: 750;
}

.hero-media,
.bento-card,
.download-card {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: var(--shadow);
}

.hero-media {
  position: relative;
  margin: 0;
  overflow: hidden;
  padding: 10px;
  border-radius: 16px;
  background: var(--surface);
  contain: paint;
}

.hero-media::after {
  content: "";
  position: absolute;
  inset: 10px;
  border: 1px solid rgba(45, 47, 52, 0.08);
  border-radius: 11px;
  pointer-events: none;
  box-shadow: inset 0 1px 0 rgba(255, 253, 250, 0.72);
}

.hero-shot {
  display: block;
  width: 100%;
  height: auto;
  border-radius: 11px;
}

.section-shell {
  padding: 62px 0 76px;
}

.section-heading {
  display: grid;
  grid-template-columns: 0.9fr 1.1fr;
  gap: 30px;
  align-items: end;
}

.section-heading h2,
.download-section h2 {
  font-size: clamp(46px, 6vw, 76px);
  line-height: 0.98;
}

.bento-grid {
  display: grid;
  grid-template-columns: repeat(12, minmax(0, 1fr));
  grid-auto-flow: dense;
  gap: 12px;
  margin-top: 30px;
}

.bento-card {
  min-height: 190px;
  padding: 22px;
  box-shadow: none;
}

.card-parse {
  grid-column: span 5;
}

.card-mp4 {
  grid-column: span 4;
}

.card-local {
  grid-column: span 3;
}

.card-speed,
.card-cookies,
.card-queue,
.card-logs {
  grid-column: span 3;
}

.card-icon {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border-radius: 999px;
  background: var(--surface-2);
  color: var(--blue);
}

.card-mp4 .card-icon {
  color: var(--green);
}

.card-cookies .card-icon {
  color: var(--rose);
}

.card-logs .card-icon {
  color: var(--yellow);
}

.bento-card > span {
  display: block;
  margin-top: 18px;
  color: var(--soft);
  font-size: 11px;
  font-weight: 900;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.bento-card h3 {
  margin: 8px 0 0;
  font-family: Georgia, "Times New Roman", serif;
  font-size: 23px;
  line-height: 1.1;
}

.bento-card p {
  margin: 10px 0 0;
  color: var(--muted);
  font-size: 14px;
  line-height: 1.55;
}

.download-section {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 300px;
  gap: 34px;
  align-items: center;
  padding: 34px 0 62px;
}

.download-card {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  padding: 14px;
  align-items: center;
}

.download-option {
  width: 100%;
  min-height: 46px;
  line-height: 1;
  background: var(--surface-2);
}

.site-footer {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  align-items: center;
  padding: 26px 0 42px;
  border-top: 1px solid var(--line);
  color: var(--muted);
}

.site-footer strong {
  color: var(--ink);
}

.footer-links {
  gap: 16px;
  font-size: 14px;
  font-weight: 850;
}

.footer-links a {
  display: inline-flex;
  align-items: center;
}

.top-link {
  justify-content: center;
  width: 36px;
  height: 36px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface);
}

@media (max-width: 980px) {
  .hero-section,
  .section-heading,
  .download-section {
    grid-template-columns: 1fr;
  }

  .landing-nav {
    grid-template-columns: 1fr auto;
  }

  .nav-links {
    order: 3;
    grid-column: 1 / -1;
    justify-self: stretch;
    justify-content: center;
  }

  .bento-grid {
    grid-template-columns: repeat(6, minmax(0, 1fr));
  }

  .card-parse,
  .card-mp4,
  .card-local,
  .card-speed,
  .card-cookies,
  .card-queue,
  .card-logs {
    grid-column: span 3;
  }
}

@media (max-width: 680px) {
  .landing-nav,
  .hero-section,
  .section-shell,
  .download-section,
  .site-footer {
    width: min(100% - 28px, 1160px);
  }

  .nav-download {
    display: none;
  }

  .nav-links {
    gap: 14px;
    padding-inline: 12px;
  }

  .hero-copy h1 {
    font-size: clamp(46px, 15vw, 68px);
  }

  .hero-actions .button,
  .download-card {
    width: 100%;
  }

  .download-card {
    grid-template-columns: 1fr;
  }

  .hero-media {
    min-height: 300px;
  }

  .hero-shot {
    object-position: left top;
  }

  .bento-grid {
    grid-template-columns: 1fr;
  }

  .card-parse,
  .card-mp4,
  .card-local,
  .card-speed,
  .card-cookies,
  .card-queue,
  .card-logs {
    grid-column: auto;
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
