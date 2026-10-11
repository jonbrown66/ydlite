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
      queueCompleted, queueFailed, canDownload, handleBatchDownload, handleProgress, cancelQueue,
      handleDownload, progress, progressPercent, progressIndeterminate, progressTrackPercent, downloadSizeText, remainingTimeText, selectedFormatId,
      logs, hasLogs, logsOpen, handleLogsToggle, flushLogs, resetLogs };
  `, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  const context = {
    exports: {}, console,
    window: { requestAnimationFrame: () => 1, cancelAnimationFrame() {}, setTimeout, clearTimeout },
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
    const { h } = setup();
    let published = 0;
    const stop = vue.watch(h.logs, () => published++, {flush:'sync'});
    for (let i = 0; i < 1005; i++) h.handleProgress({status:'log',line:`log ${i}`});
    assert.equal(h.hasLogs.value,true);
    assert.equal(h.logs.value.length,0,'closed logs must not update the reactive array');
    assert.equal(published,0,'log bursts must not cause per-line log renders');
    h.handleProgress({status:'downloading',percent:45});
    assert.equal(h.progressPercent.value,45,'batching logs must not delay actual progress');
    h.handleLogsToggle({target:{open:true}});
    assert.equal(h.logs.value.length,1000);
    assert.equal(h.logs.value[0],'log 5');
    assert.equal(h.logs.value.at(-1),'log 1004');
    for (let i = 0; i < 100; i++) h.handleProgress({status:'log',line:`more ${i}`});
    assert.equal(published,1,'opened logs coalesce a burst until the next flush');
    h.flushLogs();
    assert.equal(published,2);
    assert.equal(h.logs.value.at(-1),'more 99');
    h.resetLogs();
    assert.equal(h.hasLogs.value,false);
    assert.equal(h.logs.value.length,0);
    assert.equal(h.logsOpen.value,false);
    stop();
  }
  {
    const { h, calls } = setup();
    h.video.value = {title:'Video',isPlaylist:false,resolvedUrl:'https://example.com/video'};
    const run = h.handleDownload();
    calls[0].reject(new Error('Controlled startup failure'));
    await run;
    assert.equal(h.progress.value.status,'error','IPC failure must not leave Preparing visible');
    assert.equal(h.downloading.value,false);
    assert.equal(h.remainingTimeText.value,'—');
  }
  {
    const { h } = setup();
    h.handleProgress({status:'starting'});
    assert.equal(h.progressTrackPercent.value,0,'preparing starts at zero');
    h.handleProgress({ status:'downloading',percent:70 });
    assert.equal(h.progressPercent.value,70,'progress must update without waiting for a frame');
    h.handleProgress({ status:'downloading',percent:40 });
    assert.equal(h.progressPercent.value,40,'corrected estimate must not remain frozen at high watermark');
    h.handleProgress({ status:'downloading',percent:null,speed:'1 MiB/s',metrics:{downloadedBytes:1024,totalBytes:null,totalIsEstimate:true} });
    assert.equal(h.progressIndeterminate.value,true,'unknown total must show activity, not stale percentage');
    assert.equal(h.progressTrackPercent.value,0);
    assert.equal(h.downloadSizeText.value,'1.0 KiB / Unknown size');
    h.handleProgress({ status:'processing' });
    assert.equal(h.progressIndeterminate.value,true);
    assert.equal(h.downloadSizeText.value,'1.0 KiB / Unknown size','processing retains downloaded bytes');
    assert.equal(h.remainingTimeText.value,'Finishing…');
    h.handleProgress({ status:'finished' });
    assert.equal(h.progressPercent.value,100);
    assert.equal(h.progressIndeterminate.value,false);
  }
  {
    const { h } = setup();
    h.video.value = {duration:80,formats:[{formatId:'audio',ext:'m4a',vcodec:'none',acodec:'aac',bitrateKbps:100},{formatId:'video',ext:'mp4',vcodec:'h264',acodec:'none',filesize:1048576}]};
    h.selectedFormatId.value = 'video';
    assert.equal(h.downloadSizeText.value,'≈ 2.0 MiB','video preview includes the estimated audio size');
    h.video.value.formats[1].acodec = 'aac';
    assert.equal(h.downloadSizeText.value,'≈ 1.0 MiB','a format with audio needs no additional stream');
  }
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
