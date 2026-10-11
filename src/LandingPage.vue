<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import brandMark from '@/assets/brand-mark.svg'
import './styles/landing.css'

const downloadUrl = '/downloads/YDLite_0.1.0_x64-setup.exe'
const msiUrl = '/downloads/YDLite_0.1.0_x64_en-US.msi'
const githubUrl = 'https://github.com/jonbrown66/ydlite'
const activePreview = ref(0)
const captionPreset = ref('classic')
const bilingual = ref(true)
const menuOpen = ref(false)
const menuButton = ref<HTMLButtonElement | null>(null)
const previews = [
  { name: 'Downloads', src: '/landing/ydlite-app.png', alt: 'YDLite download workspace with video format and save location options' },
  { name: 'Subtitles', src: '/landing/ydlite-subtitles.png', alt: 'YDLite subtitle workspace with bilingual output and caption style controls' },
  { name: 'Settings', src: '/landing/ydlite-settings.png', alt: 'YDLite settings for local transcription, models, and AI providers' },
]
const presets = [{ id: 'classic', name: 'Classic' }, { id: 'contrast', name: 'High contrast' }, { id: 'yellow', name: 'Yellow highlight' }]
const routes = [
  { source: 'Chinese subtitles', action: 'Extract the subtitle track', detail: 'Keep the original timing.' },
  { source: 'Foreign subtitles', action: 'Translate to Chinese', detail: 'Use the existing subtitle track.' },
  { source: 'No subtitle track', action: 'Transcribe the audio', detail: 'Create subtitles with Whisper.' },
]
const workflow = [
  { title: 'Bring your video', text: 'Paste a video link or open a file already on your computer.' },
  { title: 'Create your subtitles', text: 'YDLite checks existing tracks, then extracts, translates, or transcribes.' },
  { title: 'Choose your output', text: 'Save an SRT file or an MP4 with Chinese or bilingual captions.' },
]
const faqs = [
  { question: 'Do I need an API key?', answer: 'No. Local mode uses Whisper for transcription and a free online translation service. If you choose Custom AI, you can connect your own OpenAI-compatible provider with an API key.' },
  { question: 'What can I export?', answer: 'Export SRT subtitle files or an MP4 with subtitles burned in. For captioned video, choose Chinese or bilingual text and adjust the font, size, color, and position.' },
  { question: 'What do I need to download first?', answer: 'Install YDLite, then choose a Whisper model and a CPU or NVIDIA CUDA runtime in Settings. Models are downloaded when you need them. Missing video tools can be installed from the app.' },
  { question: 'Can I use it offline?', answer: 'Once its model and runtime are installed, Whisper transcription and video processing run locally. Translation needs a connection to the free online service or your AI provider.' },
  { question: 'Will my original video be changed?', answer: 'Your original video is preserved. Subtitle files and captioned videos are saved as separate outputs.' },
]
function switchPreview(index: number, focus = false) {
  activePreview.value = index
  if (focus) void nextTick(() => document.getElementById(`preview-tab-${index}`)?.focus())
}
function handlePreviewKeys(event: KeyboardEvent) {
  let next = activePreview.value
  if (event.key === 'ArrowRight') next = (next + 1) % previews.length
  else if (event.key === 'ArrowLeft') next = (next + previews.length - 1) % previews.length
  else if (event.key === 'Home') next = 0
  else if (event.key === 'End') next = previews.length - 1
  else return
  event.preventDefault()
  switchPreview(next, true)
}
function closeMenuOnEscape(event: KeyboardEvent) {
  if (event.key !== 'Escape' || !menuOpen.value) return
  menuOpen.value = false
  menuButton.value?.focus()
}
onMounted(() => document.addEventListener('keydown', closeMenuOnEscape))
onBeforeUnmount(() => document.removeEventListener('keydown', closeMenuOnEscape))
</script>

<template>
  <div id="top" class="landing-page">
    <a class="skip-link" href="#main-content">Skip to content</a>
    <header class="site-header">
      <nav class="landing-nav site-width" aria-label="Main navigation">
        <a class="brand" href="#top" aria-label="YDLite home" @click="menuOpen = false">
          <span class="brand-mark">
            <img :src="brandMark" width="30" height="30" alt="" />
          </span>
          <span>YDLite</span>
        </a>
        <div id="site-menu" class="nav-links" :class="{ 'is-open': menuOpen }">
          <a href="#features" @click="menuOpen = false">Features</a>
          <a href="#workflow" @click="menuOpen = false">How it works</a>
          <a href="#modes" @click="menuOpen = false">Local &amp; AI</a>
          <a href="#faq" @click="menuOpen = false">FAQ</a>
        </div>
        <div class="nav-actions">
          <a class="button primary nav-download" :href="downloadUrl" download>Download <span class="nav-platform">for Windows</span>
          </a>
          <button ref="menuButton" class="menu-toggle" type="button" :aria-expanded="menuOpen" aria-controls="site-menu" :aria-label="menuOpen ? 'Close navigation' : 'Open navigation'" @click="menuOpen = !menuOpen">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path v-if="menuOpen" d="m6 6 12 12M18 6 6 18" />
              <path v-else d="M4 7h16M4 12h16M4 17h16" />
            </svg>
          </button>
        </div>
      </nav>
    </header>
    <main id="main-content">
      <section class="hero-section site-width" aria-labelledby="hero-title">
        <div class="hero-intro">
          <p class="eyebrow">
            <span class="eyebrow-line" /> Video downloads &amp; subtitles</p>
          <h1 id="hero-title">Your videos,<br />
            <span>with subtitles.</span>
          </h1>
          <p class="hero-lede">Download a video or open your own. Transcribe, translate, and export Chinese or bilingual subtitles in one Windows app.</p>
          <div class="hero-actions">
            <a class="button primary" :href="downloadUrl" download>
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 3v12m-5-5 5 5 5-5M5 17v4h14v-4" />
              </svg>Download for Windows</a>
            <a class="hero-secondary" href="#workflow">See how it works <span aria-hidden="true">↗</span>
            </a>
          </div>
          <p class="hero-platform">Windows x64 <span aria-hidden="true">·</span> v0.1.0 <span aria-hidden="true">·</span>
            <a :href="githubUrl" target="_blank" rel="noreferrer noopener">Open source</a>
          </p>
          <ul class="hero-benefits" aria-label="Highlights">
            <li>
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="m5 12 4 4L19 6" />
              </svg>Local transcription</li>
            <li>
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="m5 12 4 4L19 6" />
              </svg>SRT &amp; MP4 export</li>
          </ul>
        </div>
        <div class="product-preview">
          <div class="preview-tabs" role="tablist" aria-label="Product previews" @keydown="handlePreviewKeys">
            <button v-for="(preview, index) in previews" :id="`preview-tab-${index}`" :key="preview.name" type="button" role="tab" :aria-selected="activePreview === index" :tabindex="activePreview === index ? 0 : -1" aria-controls="product-preview-panel" @click="switchPreview(index)">{{ preview.name }}</button>
          </div>
          <figure class="hero-stage">
            <div id="product-preview-panel" class="stage-viewport" role="tabpanel" :aria-labelledby="`preview-tab-${activePreview}`" tabindex="0">
              <img v-for="(preview, index) in previews" v-show="activePreview === index" :key="preview.name" class="hero-shot" :src="preview.src" :alt="preview.alt" width="1180" height="780" />
            </div>
          </figure>
          <div class="preview-caption">
            <span>
              <span class="preview-dot" />A look inside YDLite</span>
            <span>Example project</span>
          </div>
        </div>
      </section>
      <section id="features" class="features-section site-width" aria-labelledby="features-title">
        <div class="section-topline">
          <span class="section-label">Made for the whole process</span>
          <span class="section-index" aria-hidden="true">01 / Features</span>
        </div>
        <article class="feature-row caption-feature">
          <div class="feature-copy">
            <p class="feature-kicker">Subtitles, your way</p>
            <h2 id="features-title">Make every word<br />easy to follow.</h2>
            <p>Keep the original language beside the translation. Choose a caption style, then save a subtitle file or a video ready to share.</p>
            <ul class="feature-points">
              <li>Chinese or bilingual captions</li>
              <li>Adjust fonts, colors, and position</li>
              <li>Separate SRT or captioned MP4</li>
            </ul>
            <a class="inline-link" href="#workflow">See the three steps <span aria-hidden="true">→</span>
            </a>
          </div>
          <div class="caption-demo">
            <div class="caption-scene" :class="`caption-${captionPreset}`">
              <img src="/landing/subtitle-poster.svg" width="1600" height="900" alt="Illustrated mountains and a path beside a river" loading="lazy" />
              <span class="scene-label">Caption style preview</span>
              <div class="demo-caption" aria-live="polite">
                <span v-if="bilingual" class="caption-original">The trail follows the river.</span>
                <span class="caption-translated" lang="zh-CN">这条小径沿着河流延伸。</span>
              </div>
              <div class="scene-footer" aria-hidden="true">
                <svg viewBox="0 0 24 24">
                  <path d="m8 5 11 7-11 7z" />
                </svg>
                <span>00:12</span>
                <span class="scene-progress">
                  <i />
                </span>
                <span>02:04</span>
              </div>
            </div>
            <div class="caption-controls">
              <fieldset class="caption-presets">
                <legend class="sr-only">Caption style</legend>
                <label v-for="preset in presets" :key="preset.id" class="caption-preset" :class="{ selected: captionPreset === preset.id }">
                  <input v-model="captionPreset" type="radio" name="caption-style" :value="preset.id" />
                  <span>{{ preset.name }}</span>
                </label>
              </fieldset>
              <label class="bilingual-toggle">
                <input v-model="bilingual" type="checkbox" />
                <span>Bilingual</span>
              </label>
            </div>
          </div>
        </article>
        <article class="feature-row route-feature">
          <div class="route-map" aria-label="Subtitle processing routes">
            <div class="route-map-header">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M5 3h10l4 4v14H5zM14 3v5h5M8 12h8M8 16h5" />
              </svg>
              <span>One video. The right starting point.</span>
            </div>
            <div v-for="(route, index) in routes" :key="route.source" class="route-option">
              <span class="route-number" aria-hidden="true">0{{ index + 1 }}</span>
              <div>
                <span class="route-condition">{{ route.source }}</span>
                <strong>{{ route.action }}</strong>
                <p>{{ route.detail }}</p>
              </div>
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M5 12h14m-5-5 5 5-5 5" />
              </svg>
            </div>
            <div class="route-output">
              <span>SRT subtitles</span>
              <span aria-hidden="true">+</span>
              <span>Captioned video</span>
            </div>
          </div>
          <div class="feature-copy">
            <p class="feature-kicker">Less starting over</p>
            <h2>Use existing subtitles.<br />Or start with the audio.</h2>
            <p>YDLite checks your video for subtitle tracks first. It can extract Chinese subtitles, translate an existing foreign track, or transcribe the audio with Whisper.</p>
            <p class="feature-note">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 3 4 6v6c0 5 8 9 8 9s8-4 8-9V6zM8 12l3 3 5-6" />
              </svg>Your original video is preserved.</p>
          </div>
        </article>
      </section>
      <section id="workflow" class="workflow-section site-width" aria-labelledby="workflow-title">
        <div class="section-topline">
          <span class="section-label">From start to saved</span>
          <span class="section-index" aria-hidden="true">02 / Workflow</span>
        </div>
        <header class="section-intro">
          <h2 id="workflow-title">Three steps. One workspace.</h2>
          <p>Start with a link or a file. Finish with subtitles you can use.</p>
        </header>
        <ol class="workflow-list">
          <li v-for="(step, index) in workflow" :key="step.title">
            <span class="step-number">0{{ index + 1 }}</span>
            <h3>{{ step.title }}</h3>
            <p>{{ step.text }}</p>
          </li>
        </ol>
      </section>
      <section id="modes" class="modes-section site-width" aria-labelledby="modes-title">
        <div class="modes-copy">
          <p class="feature-kicker">Your setup, your choice</p>
          <h2 id="modes-title">Start local.<br />Bring your own AI.</h2>
          <p>Use local Whisper transcription with free translation, or connect an OpenAI-compatible provider you already use.</p>
          <a class="inline-link" :href="githubUrl" target="_blank" rel="noreferrer noopener">Explore the project <span aria-hidden="true">↗</span>
          </a>
        </div>
        <div class="mode-sheet">
          <article class="mode-row">
            <span class="mode-symbol" aria-hidden="true">
              <svg viewBox="0 0 24 24">
                <path d="M3 4h18v12H3zM8 20h8m-4-4v4" />
              </svg>
            </span>
            <div>
              <div class="mode-heading">
                <h3>Local &amp; free</h3>
                <span class="mode-tag">No API key</span>
              </div>
              <p>Whisper + free translation</p>
              <p class="mode-detail">Download a model and runtime when you need them.</p>
            </div>
          </article>
          <article class="mode-row">
            <span class="mode-symbol" aria-hidden="true">
              <svg viewBox="0 0 24 24">
                <path d="m8 4-4 8 4 8m8-16 4 8-4 8m-3-15-2 14" />
              </svg>
            </span>
            <div>
              <div class="mode-heading">
                <h3>Custom AI</h3>
                <span class="mode-tag neutral">Your provider</span>
              </div>
              <p>Whisper + an OpenAI-compatible API</p>
              <p class="mode-detail">Choose the provider, endpoint, and model.</p>
            </div>
          </article>
          <div class="privacy-note">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M6 10h12v11H6zM8 10V6a4 4 0 0 1 8 0v4" />
            </svg>
            <p>Transcription and video processing run on your device. Translation uses the free online service or your AI provider.</p>
          </div>
        </div>
      </section>
      <section id="faq" class="faq-section site-width" aria-labelledby="faq-title">
        <header class="section-intro">
          <p class="feature-kicker">A few useful answers</p>
          <h2 id="faq-title">Before you get started.</h2>
          <a class="inline-link" :href="`${githubUrl}/issues`" target="_blank" rel="noreferrer noopener">Ask on GitHub <span aria-hidden="true">↗</span>
          </a>
        </header>
        <div class="faq-list">
          <details v-for="faq in faqs" :key="faq.question">
            <summary>{{ faq.question }}<svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="m7 10 5 5 5-5" />
              </svg>
            </summary>
            <p>{{ faq.answer }}</p>
          </details>
        </div>
      </section>
      <section id="download" class="download-section site-width" aria-labelledby="download-title">
        <div class="download-copy">
          <img class="download-logo" :src="brandMark" width="48" height="48" alt="" />
          <div>
            <p class="feature-kicker">YDLite · v0.1.0</p>
            <h2 id="download-title">Get your next video ready.</h2>
            <p>Video downloads and subtitles, together on Windows.</p>
          </div>
        </div>
        <div class="download-options">
          <a class="button primary download-primary" :href="downloadUrl" download>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 3v12m-5-5 5 5 5-5M5 17v4h14v-4" />
            </svg>Download for Windows</a>
          <div class="download-alternatives">
            <span>Windows x64</span>
            <span aria-hidden="true">·</span>
            <a class="msi-link" :href="msiUrl" download>MSI package <span aria-hidden="true">↗</span>
            </a>
          </div>
        </div>
      </section>
    </main>
    <footer class="site-footer site-width">
      <div class="footer-brand">
        <a class="brand" href="#top" aria-label="YDLite home">
          <span class="brand-mark">
            <img :src="brandMark" width="24" height="24" alt="" />
          </span>
          <span>YDLite</span>
        </a>
        <p>A Windows app for video and subtitles.</p>
      </div>
      <div class="footer-links">
        <a :href="githubUrl" target="_blank" rel="noreferrer noopener">Source code <span aria-hidden="true">↗</span>
        </a>
        <a class="top-link" href="#top">Back to top <span aria-hidden="true">↑</span>
        </a>
      </div>
    </footer>
  </div>
</template>
