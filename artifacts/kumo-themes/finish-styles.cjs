const fs = require('node:fs');
const crypto = require('node:crypto');
const indexBefore = fs.readFileSync('index.html', 'utf8');
const hashes = JSON.parse(fs.readFileSync('artifacts/kumo-themes/before/hashes.json', 'utf8').replace(/^\uFEFF/, ''));
if (crypto.createHash('sha256').update(indexBefore).digest('hex').toUpperCase() !== hashes.find(item => item.path === 'index.html').sha256) throw new Error('index.html changed after backup; inspect before continuing');
let html = indexBefore.replace('    <style>', `    <script>
      // Resolve the desktop theme before the first paint; the public website stays independent.
      if (document.documentElement.classList.contains('tauri-shell-pending')) {
        let appearance = 'system';
        try { appearance = localStorage.getItem('ydlite.appearance') || 'system'; } catch {}
        const theme = appearance === 'light' || appearance === 'dark' ? appearance : matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        document.documentElement.dataset.theme = theme;
        document.documentElement.style.colorScheme = theme;
        document.querySelector('meta[name="theme-color"]')?.setAttribute('content', theme === 'dark' ? '#17191d' : '#f7f7f8');
      }
    </script>
    <style>
      :root { --startup-canvas: #f7f7f8; --startup-surface: #ffffff; --startup-sidebar: #fafafa; --startup-hover: #f0f0f2; --startup-ink: #202023; --startup-muted: #626269; --startup-border: rgb(24 24 27 / 12%); }
      :root[data-theme="dark"] { --startup-canvas: #17191d; --startup-surface: #22252b; --startup-sidebar: #1b1d22; --startup-hover: #2d3139; --startup-ink: #eef0f4; --startup-muted: #b1b8c4; --startup-border: rgb(224 231 244 / 13%); }`);
const styleStart = html.indexOf('    <style>');
const styleEnd = html.indexOf('    </style>', styleStart);
let style = html.slice(styleStart, styleEnd);
// Preserve the palette declarations; make the startup shell consume them.
const declarationsEnd = style.indexOf('      html,');
let rules = style.slice(declarationsEnd);
for (const [color, token] of Object.entries({'#f7f7f8':'canvas','#ffffff':'surface','#fafafa':'sidebar','#f0f0f2':'hover','#e4e4e7':'hover','#202023':'ink','#626269':'muted','rgb(24 24 27 / 12%)':'border'})) rules = rules.replaceAll(color, `var(--startup-${token})`);
style = style.slice(0, declarationsEnd) + rules;
html = html.slice(0, styleStart) + style + html.slice(styleEnd);
fs.writeFileSync('index.html', html);

let queue = fs.readFileSync('src/components/DownloadQueue.vue', 'utf8').replace(/font-size: (11|12|13)px/g, 'font-size: 14px');
// These controls now use the shared keyboard-accessible ActionMenu.
queue = queue.split('\n').filter(line => !line.startsWith('.queue-item-menu')).join('\n');
queue = queue.replace('font-size: 14px; font-weight: 600; overflow: hidden;', 'font-size: 14px; font-weight: 500; overflow: hidden;');
queue = queue.replace('align-items: start; gap: 12px; padding: 14px 2px;', 'align-items: start; gap: 12px; padding: 16px;');
queue = queue.replace('scrollbar-gutter: stable; border-block: 1px solid var(--border);', 'scrollbar-gutter: stable; border: 0; border-radius: var(--radius-panel); background: var(--surface); box-shadow: 0 0 0 1px var(--border);');
queue = queue.replace('.queue-state { font-size: 14px; color: var(--muted); flex-shrink: 0; }', '.queue-state { font-size: 14px; color: var(--muted); flex-shrink: 0; padding: 2px 8px; border-radius: var(--radius-option); background: var(--surface-hover); }');
queue = queue.replace('.queue-state.failed { color: var(--danger); }.queue-state.completed { color: var(--success); }.queue-state.downloading, .queue-state.analyzing { color: var(--primary); }', '.queue-state.failed { color: var(--danger); background: var(--danger-soft); }.queue-state.completed { color: var(--success); background: var(--success-soft); }.queue-state.downloading, .queue-state.analyzing { color: var(--primary); background: var(--primary-soft); }');
queue = queue.replace('background: var(--primary); transform-origin: left;', 'background: var(--primary); transform-origin: left; transition: transform 240ms linear;');
queue = queue.replace('</style>', `.queue-item-symbol { display: grid; place-items: center; flex: 0 0 32px; width: 32px; height: 32px; border-radius: var(--radius-control); background: var(--surface-hover); color: var(--muted); }
.queue-item-symbol svg { width: 20px; height: 20px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
.saved-queue-item[data-status=downloading] .queue-item-symbol, .saved-queue-item[data-status=analyzing] .queue-item-symbol { color: var(--primary); background: var(--primary-soft); }
.saved-queue-item[data-status=completed] .queue-item-symbol { color: var(--success); background: var(--success-soft); }
.saved-queue-item[data-status=failed] .queue-item-symbol { color: var(--danger); background: var(--danger-soft); }
.saved-queue-item:focus-within { background: var(--surface-subtle); }
.queue-item-actions { flex-shrink: 0; }
@container workspace (max-width: 560px) { .queue-item-main { flex-basis: calc(100% - 44px); } .queue-item-actions { margin-left: 44px; } }
@media (prefers-reduced-motion: reduce) { .queue-item-progress span { transition: none; } }
</style>`);
fs.writeFileSync('src/components/DownloadQueue.vue', queue);
console.log('Startup theme and queue styling updated.');
