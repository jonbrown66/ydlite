const fs = require('node:fs');
const regression = fs.readFileSync('scripts/frontend-regression.cjs', 'utf8');
const start = regression.indexOf('body: `') + 7;
const end = regression.indexOf('` }));', start);
if (start < 7 || end < 0) throw Error('Missing IPC fixture');
let mock = regression.slice(start, end).replace('/src/main-app.ts?regression-real', '/src/main-app.ts');
mock = mock.replace('let finishDownload, requestId;', 'let finishDownload, rejectDownload, requestId;');
mock = mock.replace('finishDownload?.({cancelled:', "if (payload.status === 'error') rejectDownload?.(new Error(payload.message || 'Download failed')); else finishDownload?.({cancelled:");
mock = mock.replace('new Promise(resolve => { finishDownload = resolve; })', 'new Promise((resolve, reject) => { finishDownload = resolve; rejectDownload = reject; })');
mock = mock.replace("if (cmd === 'check_dependencies') return", "if (cmd === 'check_dependencies' && new URL(location.href).searchParams.get('scenario') === 'missing') return {ytdlp_ok:false,ffmpeg_ok:false};\n        if (cmd === 'check_dependencies') return");
const fixture = `<script type="module">
  window.themeAtBoot = document.documentElement.dataset.theme;
  window.startupBackground = getComputedStyle(document.querySelector('.startup-shell')).backgroundColor;
  localStorage.setItem('ydlite.defaultDir', 'C:/test-downloads');
  ${mock}
</script>`;
const html = fs.readFileSync('index.html', 'utf8').replace('<script type="module" src="/src/main-web.ts"></script>', fixture);
fs.writeFileSync('artifacts/kumo-themes/preview.html', html);
console.log('Preview now uses the actual startup shell and early theme bootstrap.');
