// Single-download state machine: pause, resume, cancel, failures and history.
const assert = require('node:assert/strict');
const { setupDownload, tick } = require('./download-test-harness.cjs');

(async () => {
  {
    const h = setupDownload();
    const started = h.download.start([{ url: 'https://example.com/one', title: 'One' }], h.request('https://example.com/one'));
    await tick();
    assert.equal(h.download.running, true);
    assert.equal(h.download.state, 'running');
    assert.equal(h.calls.length, 1, 'a single link starts exactly one download');
    assert.equal(h.calls[0].request.requestId.length > 0, true);

    h.download.pause(); await tick();
    assert.equal(h.pauses.length, 1);
    h.calls[0].resolve({ cancelled: false, paused: true, filePath: null, skipped: false });
    await started;
    assert.equal(h.download.state, 'paused', 'the paused outcome comes from the IPC result');
    assert.equal(h.download.filePath, '');

    const resumed = h.download.resume();
    await tick();
    assert.equal(h.calls.length, 2, 'resume reuses the same request and starts once');
    assert.equal(h.calls[1].request.url, h.calls[0].request.url);
    assert.notEqual(h.calls[1].request.requestId, h.calls[0].request.requestId, 'each attempt has its own id');
    h.calls[1].resolve({ cancelled: false, paused: false, filePath: 'C:/downloads/one.mp4', skipped: false });
    await resumed;
    assert.equal(h.download.state, 'completed');
    assert.equal(h.download.filePath, 'C:/downloads/one.mp4');
    assert.equal(h.history.downloads.length, 1);
    assert.equal(h.history.downloads[0].filePath, 'C:/downloads/one.mp4');
  }

  {
    const h = setupDownload();
    const started = h.download.start([{ url: 'https://example.com/cancel' }], h.request('https://example.com/cancel'));
    await tick();
    h.download.cancel(); await tick();
    assert.equal(h.cancels.length, 1);
    h.calls[0].resolve({ cancelled: true, paused: false, filePath: null, skipped: false });
    await started;
    assert.equal(h.download.state, 'cancelled');
    assert.equal(h.history.downloads.length, 0);
  }

  {
    const h = setupDownload();
    const started = h.download.start([{ url: 'https://example.com/fail' }], h.request('https://example.com/fail'));
    await tick();
    h.calls[0].reject({ message: 'Download failed', detail: 'HTTP Error 403: Forbidden' });
    await started;
    assert.equal(h.download.state, 'failed');
    assert.equal(h.download.error.kind, 'access', 'the failure is classified for the user');
    assert.equal(h.download.running, false);
  }

  {
    // A playlist downloads every selected entry, then reports one completion.
    const h = setupDownload();
    const items = [1, 2, 3].map(n => ({ url: `https://example.com/${n}`, title: `Clip ${n}` }));
    h.controlStart(false);
    await h.download.start(items, h.request('https://example.com/1'));
    assert.equal(h.download.state, 'completed');
    assert.equal(h.download.total, 3);
    assert.equal(h.history.downloads.length, 3);
    assert.equal(h.download.percent, 100);
  }

  {
    // A failed entry does not stop the next one, and only the failures are retried.
    const h = setupDownload();
    const items = [{ url: 'https://example.com/a' }, { url: 'https://example.com/b' }];
    const run = h.download.start(items, h.request('https://example.com/a'));
    await tick();
    h.calls[0].reject({ message: 'Download failed', detail: 'Connection timed out' });
    await tick();
    await tick();
    assert.equal(h.calls.length, 2, 'the queue continues after a failed entry');
    h.calls[1].resolve({ cancelled: false, paused: false, filePath: 'C:/downloads/b.mp4', skipped: false });
    await run;
    assert.equal(h.download.state, 'failed');
    assert.equal(h.download.failed, 1);
    assert.equal(h.history.downloads.length, 1);

    const retry = h.download.retryFailed();
    await tick();
    assert.equal(h.calls.length, 3);
    assert.equal(h.calls[2].request.url, 'https://example.com/a', 'retry only repeats failed entries');
    h.calls[2].resolve({ cancelled: false, paused: false, filePath: 'C:/downloads/a.mp4', skipped: false });
    await retry;
    await tick();
    assert.equal(h.download.state, 'completed');
  }

  {
    // Stale events from a previous attempt must not move the current one.
    const h = setupDownload();
    const run = h.download.start([{ url: 'https://example.com/late' }], h.request('https://example.com/late'));
    await tick();
    h.emit({ requestId: 'stale-attempt', status: 'downloading', percent: 90 });
    assert.equal(h.download.progress, null, 'events from another attempt are ignored');
    h.emit({ requestId: h.calls[0].request.requestId, status: 'downloading', percent: 42, metrics: { downloadedBytes: 42, totalBytes: 100, totalIsEstimate: false } });
    assert.equal(h.download.percent, 42);
    h.calls[0].resolve({ cancelled: false, paused: false, filePath: 'C:/downloads/late.mp4', skipped: false });
    await run;
    assert.equal(h.download.state, 'completed');
  }

  console.log('PASS: single download, pause/resume, cancel, classified failures, playlist continuation, retry of failed entries, stale event filtering.');
})().catch(error => { console.error(error); process.exitCode = 1; });
