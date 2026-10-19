const fs = require('node:fs');
function edit(file, transform) {
  const before = fs.readFileSync(file, 'utf8');
  const after = transform(before);
  if (after === before) throw new Error(`No changes: ${file}`);
  fs.writeFileSync(file, after);
}
function replace(text, from, to) {
  if (!text.includes(from)) throw new Error(`Missing edit target: ${from.slice(0, 70)}`);
  return text.replace(from, to);
}
edit('src/styles.css', text => {
  text = replace(text, 'background: #e4e4e7;', 'background: var(--window-pressed);');
  text = replace(text, 'background: rgb(24 24 27 / 36%);', 'background: var(--backdrop);');
  text = replace(text, '0 16px 48px rgb(24 24 27 / 16%)', '0 16px 48px rgb(var(--shadow-rgb) / 24%)');
  text = replace(text, 'transition: width 320ms linear;', 'transform-origin: left; transition: transform 240ms linear;');
  text = replace(text, '.progress-panel.is-finished .progress-fill { background: var(--success); }', `.progress-panel.is-finished .progress-fill { background: var(--success); }
.progress-panel.is-finished .progress-status { color: var(--success); }
.progress-panel[data-state=error] .progress-status { color: var(--danger); }
.progress-panel[data-state=cancelled] .progress-status, .progress-panel[data-state=cancelling] .progress-status { color: var(--muted); }
.progress-panel[data-state=error] .progress-fill, .progress-panel[data-state=cancelled] .progress-fill { background: var(--subtle); }`);
  text = replace(text, '.progress-fill, .queue-progress-fill, .sidebar-progress-track i { transition: width 140ms linear !important; }', '.progress-fill, .queue-progress-fill { transition: none !important; }\n  .sidebar-progress-track i { transition: width 140ms linear !important; }');
  text = replace(text, '.sidebar-link, .sidebar-collapse-toggle {', '.sidebar-link, .sidebar-collapse-toggle {'); // shared shell additions below
  text = replace(text, '.sidebar-footer {', '.appearance-toggle { width: 100%; border: 0; background: transparent; text-align: left; }\n.sidebar-footer {');
  text = replace(text, '.progress-panel, .queue-status-panel { padding-top: 16px; margin-top: 20px; border-top: 1px solid var(--border); }', '.progress-panel, .queue-status-panel { padding-top: 20px; margin-top: 24px; border-top: 1px solid var(--border); }');
  text = replace(text, '.progress-foot { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 9rem), 1fr)); gap: 12px 20px; margin-block: 12px 0; }', '.progress-foot { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 9rem), 1fr)); gap: 16px 24px; margin-block: 20px 0; }\n.progress-foot > div { min-width: 0; padding-left: 16px; border-left: 1px solid var(--border); }\n.progress-foot > div:first-child { padding-left: 0; border-left: 0; }\n.progress-head { min-height: 24px; align-items: center; font-weight: 500; }\n.progress-head strong { color: var(--primary); }\n.progress-panel.is-finished .progress-head strong { color: var(--success); }');
  text = text.replaceAll('box-shadow: inset 0 0 0 1px rgb(24 24 27 / 4%)', 'box-shadow: inset 0 0 0 1px var(--border)');
  return text;
});
edit('src/pages/DownloadPage.vue', text => {
  text = replace(text, 'transition: background-color 120ms ease, color 120ms ease, box-shadow 120ms ease;', 'transition: transform 100ms var(--ease-out);');
  text = replace(text, 'border-color: var(--border); box-shadow: 0 1px 2px rgb(24 24 27 / 8%);', 'border-color: transparent; box-shadow: var(--control-shadow);');
  return text.replace(/font-size: (12|13)px/g, 'font-size: 14px');
});
for (const file of ['src/components/DownloadOptionsFields.vue', 'src/pages/SettingsPage.vue']) {
  edit(file, text => text.replace(/font-size: (12|13)px/g, 'font-size: 14px'));
}
// The preview uses the existing regression IPC fixture with the actual application modules.
const regression = fs.readFileSync('scripts/frontend-regression.cjs', 'utf8');
const start = regression.indexOf('body: `') + 7;
const end = regression.indexOf('` }));', start);
if (start < 7 || end < 0) throw new Error('Regression fixture not found');
const body = regression.slice(start, end).replace('/src/main-app.ts?regression-real', '/src/main-app.ts');
fs.writeFileSync('artifacts/kumo-themes/preview.html', `<!doctype html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>YDLite theme preview</title></head><body><div id="app"></div><script type="module">localStorage.setItem('ydlite.defaultDir','C:/test-downloads');${body}</script></body></html>`);
console.log('Updated shared styles and created isolated browser fixture.');
