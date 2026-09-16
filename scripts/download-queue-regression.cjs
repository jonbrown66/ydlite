// Exercise the actual Vue setup logic with controlled IPC completion timing.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const ts = require('typescript');
const vue = require('vue');

function setup() {
  const calls = [], finishes = [], history = [];
  const api = {
    startDownload(request) {
      return new Promise((resolve, reject) => calls.push({ request, resolve, reject }));
    },
    cancelDownload: async () => {},
  };
  const activity = new Proxy({}, { get: (_, name) => name === 'finishDownload'
    ? value => finishes.push(value) : () => {} });
  const source = fs.readFileSync('src/pages/DownloadPage.vue', 'utf8').split('<script setup lang="ts">')[1].split('</script>')[0];
  const code = ts.transpileModule(source + `
    globalThis.test = { video, saveDir, selectedEntries, queueActive, downloading, cancelling,
      queueCompleted, queueFailed, canDownload, handleBatchDownload, handleProgress, cancelQueue };
  `, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  const context = {
    exports: {}, console,
    window: { requestAnimationFrame: () => 1, cancelAnimationFrame() {} },
    localStorage: { setItem() {} },
    require(name) {
      if (name === 'vue') return { ...vue, onMounted() {}, onBeforeUnmount() {} };
      if (name === 'vue-router') return { useRouter: () => ({}) };
      if (name.includes('api/tauri')) return api;
      if (name === '@/stores/tasks') return { useTasksStore: () => ({ downloads: [], addDownload: item => history.push(item) }) };
      if (name === '@/stores/activity') return { useActivityStore: () => activity };
      return {};
    },
  };
  vm.runInNewContext(code, context);
  const h = context.test;
  h.video.value = { title: 'Playlist', isPlaylist: true, entries: [1, 2, 3].map(n => ({ title: `Video ${n}`, url: `https://example.com/${n}` })) };
  h.saveDir.value = 'C:/test';
  h.selectedEntries.value = Object.fromEntries(h.video.value.entries.map(e => [e.url, true]));
  return { h, calls, finishes, history };
}
const tick = () => new Promise(resolve => setImmediate(resolve));

(async () => {
  {
    const { h, calls, finishes, history } = setup();
    const run = h.handleBatchDownload();
    assert.equal(calls.length, 1);
    h.handleProgress({ status: 'error' });
    await tick();
    assert.equal(calls.length, 1, 'terminal event must not advance queue before IPC settles');
    calls[0].reject(new Error('first failed'));
    await tick();
    assert.equal(calls.length, 2);
    h.handleProgress({ status: 'finished', filePath: 'C:/test/second.mp4' });
    await tick();
    assert.equal(calls.length, 2);
    calls[1].resolve();
    await tick();
    assert.equal(calls.length, 3);
    // Spawn/validation errors may return without emitting any progress event.
    calls[2].reject(new Error('spawn failed'));
    await run;
    assert.equal(h.queueCompleted.value, 1);
    assert.equal(h.queueFailed.value, 2);
    assert.equal(finishes[0].state, 'failed');
    assert.equal(history[0].title, 'Video 2');
    assert.equal(h.queueActive.value, false);
  }
  {
    const { h, calls, finishes } = setup();
    const run = h.handleBatchDownload();
    h.cancelQueue();
    h.handleProgress({ status: 'cancelled' });
    await tick();
    assert.equal(h.queueActive.value, true, 'keep queue locked while cancellation is pending');
    assert.equal(h.canDownload.value, false);
    assert.equal(h.cancelling.value, true);
    calls[0].resolve();
    await run;
    assert.equal(calls.length, 1);
    assert.equal(h.queueCompleted.value, 0);
    assert.equal(h.queueFailed.value, 0);
    assert.equal(finishes[0].state, 'cancelled');
    assert.equal(h.cancelling.value, false);
    assert.equal(h.canDownload.value, true);
  }
  console.log('Download queue regression checks passed');
})().catch(error => { console.error(error); process.exitCode = 1; });
