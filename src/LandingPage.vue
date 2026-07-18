<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { gsap } from 'gsap'
import { ScrollTrigger } from 'gsap/ScrollTrigger'

const downloadUrl = '/downloads/YDLite_0.1.0_x64-setup.exe'
const msiUrl = '/downloads/YDLite_0.1.0_x64_en-US.msi'
const githubUrl = 'https://github.com/jonbrown66/ydlite'
const heroImageUrl = '/landing/ydlite-app.png'

const pageRoot = ref<HTMLElement | null>(null)
let animationContext: gsap.Context | undefined

const workflowItems = [
  ['01', '选择视频', '粘贴视频链接，或导入本地文件。'],
  ['02', '自动分析', '检查字幕、音轨与语言，选择更省时的路线。'],
  ['03', '获得结果', '输出中文字幕，并按需生成字幕视频。'],
]

const modes = [
  {
    name: '本地免费',
    engine: 'Whisper + 免费翻译',
    note: '适合不配置 API 的日常使用',
  },
  {
    name: '自定义 AI',
    engine: 'Whisper + 兼容接口',
    note: '使用你已有的翻译服务',
  },
  {
    name: 'Gemini 云端',
    engine: '识别与翻译一体完成',
    note: '减少本地模型和硬件占用',
  },
]

const reliabilityItems = [
  ['本地与云端自由选择', '模型按需下载，不强制占用空间。'],
  ['输出经过完整校验', '确认字幕与视频可用后才显示完成。'],
  ['任务可以继续处理', '保留进度、结果和清理记录。'],
]

function iconPath(name: string) {
  const icons: Record<string, string> = {
    app: 'M7 3.8h10a3.2 3.2 0 0 1 3.2 3.2v10a3.2 3.2 0 0 1-3.2 3.2H7A3.2 3.2 0 0 1 3.8 17V7A3.2 3.2 0 0 1 7 3.8Zm0 2A1.2 1.2 0 0 0 5.8 7v10A1.2 1.2 0 0 0 7 18.2h10a1.2 1.2 0 0 0 1.2-1.2V7A1.2 1.2 0 0 0 17 5.8H7Zm3.1 3.1 5.2 3.1-5.2 3.1V8.9Z',
    arrow: 'm9 18 6-6-6-6 1.4-1.4 7.4 7.4-7.4 7.4L9 18Z',
    arrowUp: 'M12 4 5.5 10.5l1.4 1.4L11 7.8V20h2V7.8l4.1 4.1 1.4-1.4L12 4Z',
    check: 'm9.1 16.2-4.3-4.3 1.4-1.4 2.9 2.9 8.7-8.7 1.4 1.4-10.1 10.1Z',
  }
  return icons[name] || icons.arrow
}

onMounted(() => {
  if (!pageRoot.value || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return

  gsap.registerPlugin(ScrollTrigger)
  animationContext = gsap.context(() => {
    const introTimeline = gsap.timeline({ defaults: { ease: 'power4.out' } })
    introTimeline
      .from('.landing-nav', { y: -16, opacity: 0, duration: 0.55 })
      .from('.hero-intro > *', { y: 28, opacity: 0, duration: 0.72, stagger: 0.08 }, '-=0.25')
      .from('.hero-stage', { y: 36, opacity: 0, scale: 0.985, duration: 0.85 }, '-=0.45')

    gsap.utils.toArray<HTMLElement>('[data-reveal]').forEach((element) => {
      gsap.from(element, {
        scrollTrigger: {
          trigger: element,
          start: 'top 86%',
          once: true,
        },
        y: 30,
        opacity: 0,
        duration: 0.7,
        ease: 'power4.out',
      })
    })

    gsap.to('.hero-shot', {
      yPercent: -1.8,
      ease: 'none',
      scrollTrigger: {
        trigger: '.hero-stage',
        start: 'top bottom',
        end: 'bottom top',
        scrub: 0.7,
      },
    })
  }, pageRoot.value)
})

onBeforeUnmount(() => {
  animationContext?.revert()
  ScrollTrigger.getAll().forEach((trigger) => trigger.kill())
})
</script>

<template>
  <main id="top" ref="pageRoot" class="landing-page">
    <nav class="landing-nav">
      <a class="brand" href="#top" aria-label="YDLite 首页">
        <span class="brand-mark">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path fill="currentColor" :d="iconPath('app')" />
          </svg>
        </span>
        <span>YDLite</span>
      </a>

      <div class="nav-links" aria-label="主要导航">
        <a href="#workflow">流程</a>
        <a href="#features">功能</a>
        <a href="#modes">处理方式</a>
      </div>

      <div class="nav-actions">
        <a class="text-link" :href="githubUrl" target="_blank" rel="noreferrer">GitHub</a>
        <a class="nav-download" :href="downloadUrl" download>下载 Windows 版</a>
      </div>
    </nav>

    <section class="hero-section">
      <div class="hero-intro">
        <p class="eyebrow">Windows 视频与字幕工具</p>
        <h1>从视频到中文字幕，<br />交给 YDLite。</h1>
        <p class="hero-lede">
          粘贴链接或选择本地视频。自动检查字幕、识别语言、翻译成中文，
          并按需生成可以直接播放的字幕视频。
        </p>
        <div class="hero-actions">
          <a class="button button-primary" :href="downloadUrl" download>
            下载 Windows 版
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path fill="currentColor" :d="iconPath('arrow')" />
            </svg>
          </a>
          <a class="button button-secondary" href="#workflow">了解处理流程</a>
        </div>
      </div>

      <figure class="hero-stage">
        <div class="stage-bar">
          <span>YDLite for Windows</span>
          <span>下载 · 转录 · 翻译 · 字幕视频</span>
        </div>
        <div class="stage-viewport">
          <img class="hero-shot" :src="heroImageUrl" alt="YDLite Windows 应用新版界面" />
        </div>
        <figcaption>
          <span>一个入口完成视频下载与字幕处理</span>
          <span>本地优先 · 原视频不覆盖</span>
        </figcaption>
      </figure>
    </section>

    <section id="workflow" class="workflow-section" data-reveal>
      <header class="section-intro">
        <p class="eyebrow">简单的一条链路</p>
        <h2>不需要先理解模型和参数。</h2>
        <p>选择视频之后，YDLite 会判断该提取字幕、翻译文本，还是重新识别音轨。</p>
      </header>

      <ol class="workflow-list">
        <li v-for="item in workflowItems" :key="item[0]">
          <span class="step-number">{{ item[0] }}</span>
          <strong>{{ item[1] }}</strong>
          <p>{{ item[2] }}</p>
        </li>
      </ol>
    </section>

    <section id="features" class="feature-section">
      <article class="route-feature" data-reveal>
        <div class="route-copy">
          <p class="eyebrow">自动选择处理路线</p>
          <h2>先看有没有字幕，<br />再决定下一步。</h2>
          <p>
            中文字幕直接提取，外语字幕只翻译文本；没有可用字幕时，才从音轨重新识别。
          </p>
        </div>

        <div class="route-map" aria-label="字幕自动处理路线">
          <div class="route-source">
            <span>输入</span>
            <strong>视频文件</strong>
          </div>
          <div class="route-branches">
            <div>
              <span>已有中文字幕</span>
              <strong>直接提取</strong>
            </div>
            <div>
              <span>已有外语字幕</span>
              <strong>只翻译文本</strong>
            </div>
            <div>
              <span>没有字幕</span>
              <strong>识别音轨</strong>
            </div>
          </div>
        </div>
      </article>

      <div class="reliability-panel" data-reveal>
        <div class="panel-heading">
          <p class="eyebrow">处理过程更可靠</p>
          <h2>看得见进度，也拿得到结果。</h2>
        </div>
        <div class="reliability-list">
          <article v-for="item in reliabilityItems" :key="item[0]">
            <span class="check-mark">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path fill="currentColor" :d="iconPath('check')" />
              </svg>
            </span>
            <div>
              <h3>{{ item[0] }}</h3>
              <p>{{ item[1] }}</p>
            </div>
          </article>
        </div>
      </div>
    </section>

    <section id="modes" class="modes-section" data-reveal>
      <header class="section-intro compact">
        <p class="eyebrow">三种处理方式</p>
        <h2>选择适合你的方式。</h2>
      </header>

      <div class="mode-table">
        <article v-for="(mode, index) in modes" :key="mode.name">
          <span>0{{ index + 1 }}</span>
          <div>
            <h3>{{ mode.name }}</h3>
            <strong>{{ mode.engine }}</strong>
            <p>{{ mode.note }}</p>
          </div>
        </article>
      </div>
    </section>

    <section id="download" class="download-section" data-reveal>
      <div>
        <p class="eyebrow">Windows x64</p>
        <h2>现在开始处理你的视频。</h2>
        <p>应用本体保持轻量，本地模型和加速组件都由你按需下载。</p>
      </div>
      <div class="download-actions">
        <a class="button download-primary" :href="downloadUrl" download>
          下载 Windows 版
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path fill="currentColor" :d="iconPath('arrow')" />
          </svg>
        </a>
        <a class="msi-link" :href="msiUrl" download>下载 MSI 安装包</a>
      </div>
    </section>

    <footer class="site-footer">
      <strong>YDLite</strong>
      <p>视频下载与字幕处理工具</p>
      <div>
        <a :href="githubUrl" target="_blank" rel="noreferrer">GitHub</a>
        <a class="top-link" href="#top" aria-label="返回顶部">
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
  --surface: #fffdf8;
  --surface-soft: #ece6dc;
  --ink: #3f4044;
  --ink-deep: #2f3033;
  --muted: #6c6964;
  --soft: #98928a;
  --line: #d8d1c6;
  --line-strong: #bfb6aa;
  --accent: #806d56;
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
  color: var(--ink);
  background: var(--page);
  font-family: "Aptos", "Segoe UI", "Microsoft YaHei UI", system-ui, sans-serif;
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
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
}

.landing-page {
  min-height: 100vh;
  overflow-x: hidden;
  background:
    radial-gradient(circle at 50% 8%, rgba(255, 253, 248, 0.76), transparent 34rem),
    var(--page);
}

.landing-nav,
.hero-section,
.workflow-section,
.feature-section,
.modes-section,
.download-section,
.site-footer {
  width: min(1180px, calc(100% - 48px));
  margin-inline: auto;
}

.landing-nav {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  min-height: 76px;
  border-bottom: 1px solid color-mix(in srgb, var(--line) 72%, transparent);
}

.brand,
.nav-links,
.nav-actions,
.hero-actions,
.button,
.download-actions,
.site-footer > div {
  display: flex;
  align-items: center;
}

.brand {
  width: fit-content;
  gap: 10px;
  font-size: 1rem;
  font-weight: 800;
}

.brand-mark {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--ink-deep);
  color: var(--surface);
}

.brand-mark svg {
  width: 17px;
  height: 17px;
}

.nav-links {
  justify-self: center;
  gap: 28px;
  color: var(--muted);
  font-size: 0.84rem;
  font-weight: 700;
}

.nav-links a,
.text-link,
.msi-link,
.site-footer a {
  transition:
    color 180ms var(--ease-out),
    opacity 180ms var(--ease-out);
}

.nav-links a:hover,
.text-link:hover,
.msi-link:hover,
.site-footer a:hover {
  color: var(--ink-deep);
}

.nav-actions {
  justify-self: end;
  gap: 18px;
}

.text-link {
  color: var(--muted);
  font-size: 0.84rem;
  font-weight: 700;
}

.nav-download {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 40px;
  padding-inline: 17px;
  border-radius: 999px;
  background: var(--ink-deep);
  color: var(--surface);
  font-size: 0.84rem;
  font-weight: 750;
  transition:
    transform 180ms var(--ease-out),
    background-color 180ms var(--ease-out);
}

.nav-download:hover {
  transform: translateY(-1px);
  background: #222326;
}

.hero-section {
  padding: clamp(64px, 9vw, 112px) 0 clamp(72px, 9vw, 116px);
}

.hero-intro {
  max-width: 900px;
}

.eyebrow {
  margin: 0;
  color: var(--accent);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.hero-intro h1,
.section-intro h2,
.route-copy h2,
.panel-heading h2,
.download-section h2 {
  color: var(--ink-deep);
  font-family: Georgia, "Noto Serif SC", "Songti SC", serif;
  font-weight: 700;
  letter-spacing: -0.025em;
  text-wrap: balance;
}

.hero-intro h1 {
  max-width: 12ch;
  margin: 18px 0 0;
  font-size: clamp(3.25rem, 7.2vw, 6.4rem);
  line-height: 1.04;
}

.hero-lede {
  max-width: 660px;
  margin: 24px 0 0;
  color: var(--muted);
  font-size: clamp(1rem, 1.6vw, 1.14rem);
  line-height: 1.72;
}

.hero-actions {
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 30px;
}

.button {
  justify-content: center;
  min-height: 48px;
  padding: 0 21px;
  border: 1px solid var(--ink-deep);
  border-radius: 999px;
  font-size: 0.88rem;
  font-weight: 750;
  transition:
    transform 180ms var(--ease-out),
    box-shadow 180ms var(--ease-out),
    background-color 180ms var(--ease-out);
}

.button:hover {
  transform: translateY(-2px);
}

.button:active,
.nav-download:active {
  transform: translateY(0) scale(0.98);
}

.button-primary {
  gap: 12px;
  background: var(--ink-deep);
  color: var(--surface);
}

.button-primary:hover {
  background: #222326;
  box-shadow: 0 12px 30px rgba(38, 35, 31, 0.12);
}

.button-secondary {
  background: transparent;
  color: var(--ink-deep);
}

.button-secondary:hover {
  background: rgba(255, 253, 248, 0.48);
}

.hero-stage {
  margin: clamp(48px, 6vw, 72px) 0 0;
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 18px;
  background: var(--ink-deep);
  box-shadow: 0 30px 80px rgba(50, 45, 39, 0.13);
}

.stage-bar,
.hero-stage figcaption {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  color: #d8d1c7;
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.02em;
}

.stage-bar {
  padding: 14px 18px;
}

.stage-viewport {
  overflow: hidden;
  margin-inline: 10px;
  border-radius: 10px;
  background: #e8edf3;
}

.hero-shot {
  display: block;
  width: 100%;
  height: auto;
  transform: scale(1.012);
  transform-origin: center top;
}

.hero-stage figcaption {
  margin: 0;
  padding: 14px 18px 16px;
  color: #aaa49c;
}

.workflow-section,
.feature-section,
.modes-section,
.download-section {
  scroll-margin-top: 30px;
}

.workflow-section {
  display: grid;
  grid-template-columns: minmax(260px, 0.8fr) minmax(0, 1.2fr);
  gap: clamp(48px, 8vw, 112px);
  padding: clamp(72px, 9vw, 112px) 0;
  border-top: 1px solid var(--line);
}

.section-intro h2,
.route-copy h2,
.panel-heading h2 {
  margin: 16px 0 0;
  font-size: clamp(2.5rem, 5vw, 4.6rem);
  line-height: 1.08;
}

.section-intro > p:last-child,
.route-copy > p:last-child,
.download-section > div > p:last-child {
  max-width: 50ch;
  margin: 20px 0 0;
  color: var(--muted);
  line-height: 1.7;
}

.workflow-list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.workflow-list li {
  display: grid;
  grid-template-columns: 44px minmax(120px, 0.55fr) minmax(0, 1fr);
  gap: 18px;
  align-items: baseline;
  padding: 25px 0;
  border-bottom: 1px solid var(--line);
}

.workflow-list li:first-child {
  padding-top: 0;
}

.step-number {
  color: var(--accent);
  font-size: 0.75rem;
  font-weight: 850;
  letter-spacing: 0.08em;
}

.workflow-list strong {
  color: var(--ink-deep);
  font-family: Georgia, "Noto Serif SC", "Songti SC", serif;
  font-size: 1.45rem;
}

.workflow-list p {
  margin: 0;
  color: var(--muted);
  font-size: 0.92rem;
  line-height: 1.6;
}

.feature-section {
  padding: 0 0 clamp(80px, 10vw, 128px);
}

.route-feature {
  display: grid;
  grid-template-columns: minmax(0, 0.9fr) minmax(420px, 1.1fr);
  gap: clamp(48px, 8vw, 108px);
  align-items: center;
  padding: clamp(38px, 6vw, 72px);
  border-radius: 20px;
  background: var(--ink-deep);
  color: var(--surface);
}

.route-copy h2 {
  color: var(--surface);
}

.route-copy > p:last-child {
  color: #bbb5ad;
}

.route-feature .eyebrow {
  color: #b9a68d;
}

.route-map {
  position: relative;
  display: grid;
  grid-template-columns: 0.72fr 1.28fr;
  gap: 28px;
  align-items: center;
}

.route-map::before {
  content: "";
  position: absolute;
  left: calc(36% - 15px);
  width: 28px;
  height: 1px;
  background: #6c6862;
}

.route-source,
.route-branches > div {
  padding: 18px;
  border: 1px solid #5a5752;
  border-radius: 12px;
  background: #38393c;
}

.route-branches {
  display: grid;
  gap: 10px;
}

.route-map span,
.route-map strong {
  display: block;
}

.route-map span {
  color: #aaa49c;
  font-size: 0.72rem;
}

.route-map strong {
  margin-top: 6px;
  color: #f4efe7;
  font-size: 0.96rem;
}

.reliability-panel {
  display: grid;
  grid-template-columns: minmax(280px, 0.8fr) minmax(0, 1.2fr);
  gap: clamp(44px, 8vw, 110px);
  padding: clamp(72px, 9vw, 112px) 0 0;
}

.panel-heading h2 {
  max-width: 11ch;
}

.reliability-list {
  border-top: 1px solid var(--line);
}

.reliability-list article {
  display: grid;
  grid-template-columns: 40px minmax(0, 1fr);
  gap: 18px;
  padding: 25px 0;
  border-bottom: 1px solid var(--line);
}

.check-mark {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border: 1px solid var(--line-strong);
  border-radius: 50%;
  color: var(--accent);
}

.check-mark svg {
  width: 16px;
  height: 16px;
}

.reliability-list h3 {
  margin: 0;
  color: var(--ink-deep);
  font-size: 1.05rem;
}

.reliability-list p {
  margin: 7px 0 0;
  color: var(--muted);
  font-size: 0.9rem;
  line-height: 1.55;
}

.modes-section {
  padding: clamp(72px, 9vw, 108px) 0;
  border-top: 1px solid var(--line);
}

.section-intro.compact {
  max-width: 720px;
}

.mode-table {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  margin-top: clamp(38px, 5vw, 58px);
  border-top: 1px solid var(--line-strong);
  border-bottom: 1px solid var(--line-strong);
}

.mode-table article {
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr);
  gap: 16px;
  min-height: 210px;
  padding: 28px;
  border-left: 1px solid var(--line);
}

.mode-table article:first-child {
  border-left: 0;
}

.mode-table article > span {
  color: var(--soft);
  font-size: 0.72rem;
  font-weight: 800;
  letter-spacing: 0.08em;
}

.mode-table h3 {
  margin: 0;
  color: var(--ink-deep);
  font-family: Georgia, "Noto Serif SC", "Songti SC", serif;
  font-size: 1.65rem;
}

.mode-table strong {
  display: block;
  margin-top: 24px;
  font-size: 0.92rem;
}

.mode-table p {
  margin: 8px 0 0;
  color: var(--muted);
  font-size: 0.86rem;
  line-height: 1.55;
}

.download-section {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 40px;
  align-items: end;
  margin-bottom: 54px;
  padding: clamp(44px, 7vw, 76px);
  border-radius: 20px;
  background: var(--surface);
  box-shadow: 0 24px 70px rgba(54, 48, 41, 0.07);
}

.download-section h2 {
  max-width: 12ch;
  margin: 15px 0 0;
  font-size: clamp(2.8rem, 5vw, 4.8rem);
  line-height: 1.05;
}

.download-actions {
  flex-direction: column;
  align-items: stretch;
  min-width: 230px;
  gap: 16px;
}

.download-primary {
  gap: 12px;
  background: var(--ink-deep);
  color: var(--surface);
}

.download-primary:hover {
  background: #222326;
  box-shadow: 0 12px 30px rgba(38, 35, 31, 0.12);
}

.msi-link {
  color: var(--muted);
  font-size: 0.8rem;
  font-weight: 700;
  text-align: center;
  text-decoration: underline;
  text-decoration-color: var(--line-strong);
  text-underline-offset: 4px;
}

.site-footer {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 24px;
  align-items: center;
  padding: 30px 0 44px;
  border-top: 1px solid var(--line);
  color: var(--muted);
  font-size: 0.82rem;
}

.site-footer strong {
  color: var(--ink-deep);
}

.site-footer p {
  margin: 0;
}

.site-footer > div {
  gap: 18px;
  font-weight: 700;
}

.top-link {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border: 1px solid var(--line);
  border-radius: 50%;
  background: var(--surface);
}

.top-link svg {
  width: 17px;
  height: 17px;
}

a:focus-visible {
  outline: 2px solid var(--ink-deep);
  outline-offset: 4px;
}

@media (max-width: 900px) {
  .landing-nav {
    grid-template-columns: 1fr auto;
  }

  .nav-links {
    display: none;
  }

  .hero-intro h1 {
    max-width: 14ch;
  }

  .workflow-section,
  .route-feature,
  .reliability-panel,
  .download-section {
    grid-template-columns: 1fr;
  }

  .route-map {
    max-width: 640px;
  }

  .mode-table {
    grid-template-columns: 1fr;
  }

  .mode-table article {
    min-height: auto;
    border-top: 1px solid var(--line);
    border-left: 0;
  }

  .mode-table article:first-child {
    border-top: 0;
  }

  .download-actions {
    align-items: flex-start;
  }
}

@media (max-width: 620px) {
  .landing-nav,
  .hero-section,
  .workflow-section,
  .feature-section,
  .modes-section,
  .download-section,
  .site-footer {
    width: min(100% - 28px, 1180px);
  }

  .landing-nav {
    min-height: 66px;
  }

  .text-link {
    display: none;
  }

  .nav-download {
    min-height: 38px;
    padding-inline: 14px;
    font-size: 0.78rem;
  }

  .hero-section {
    padding-top: 52px;
  }

  .hero-intro h1 {
    margin-top: 14px;
    font-size: clamp(2.75rem, 13vw, 4.2rem);
  }

  .hero-lede {
    margin-top: 20px;
    font-size: 1rem;
  }

  .hero-actions,
  .hero-actions .button {
    width: 100%;
  }

  .hero-actions {
    display: grid;
  }

  .hero-stage {
    margin-top: 38px;
    border-radius: 13px;
  }

  .stage-bar,
  .hero-stage figcaption {
    padding: 11px 12px;
    font-size: 0.67rem;
  }

  .stage-bar span:last-child,
  .hero-stage figcaption span:last-child {
    display: none;
  }

  .stage-viewport {
    margin-inline: 6px;
    border-radius: 8px;
  }

  .workflow-section {
    gap: 42px;
  }

  .section-intro h2,
  .route-copy h2,
  .panel-heading h2 {
    font-size: clamp(2.4rem, 12vw, 3.4rem);
  }

  .workflow-list li {
    grid-template-columns: 36px 1fr;
    gap: 12px;
  }

  .workflow-list p {
    grid-column: 2;
  }

  .route-feature {
    gap: 42px;
    padding: 34px 24px;
    border-radius: 16px;
  }

  .route-map {
    grid-template-columns: 1fr;
  }

  .route-map::before {
    display: none;
  }

  .reliability-panel {
    gap: 34px;
  }

  .mode-table article {
    grid-template-columns: 28px 1fr;
    padding: 24px 8px;
  }

  .download-section {
    width: calc(100% - 28px);
    gap: 32px;
    padding: 34px 24px;
    border-radius: 16px;
  }

  .download-section h2 {
    font-size: clamp(2.5rem, 12vw, 3.6rem);
  }

  .download-actions,
  .download-primary {
    width: 100%;
  }

  .site-footer {
    grid-template-columns: 1fr auto;
  }

  .site-footer p {
    display: none;
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
