// File options reach the backend unchanged, replace needs confirmation, failures classify.
const assert = require('node:assert/strict');
const { setupDownload, tick } = require('./download-test-harness.cjs');

(async () => {
  const h = setupDownload();
  const fileOptions = { naming: 'authorTitle', conflict: 'overwrite', quality: 'quality', audioFormatId: 'audio-en' };
  const run = h.download.start([{ url: 'https://example.com/video' }], h.request('https://example.com/video', { fileOptions: { ...fileOptions } }));
  await tick();
  assert.deepEqual(JSON.parse(JSON.stringify(h.calls[0].request.fileOptions)), fileOptions, 'file options reach the backend unchanged');
  assert.equal(h.calls[0].request.overwriteConfirmed, false, 'replace never starts without confirmation');
  h.calls[0].resolve({ cancelled: false, paused: false, filePath: 'C:/downloads/v.mp4', skipped: true });
  await run;

  const confirmed = setupDownload();
  const approved = confirmed.download.start(
    [{ url: 'https://example.com/video' }],
    confirmed.request('https://example.com/video', { fileOptions: { ...fileOptions }, overwriteConfirmed: true }),
  );
  await tick();
  assert.equal(confirmed.calls[0].request.overwriteConfirmed, true, 'the confirmed replace flag is forwarded');
  confirmed.calls[0].resolve({ cancelled: false, paused: false, filePath: 'C:/downloads/v.mp4', skipped: false });
  await approved;

  const classify = h.load('@/lib/download').classifyDownloadError;
  assert.equal(classify({ message: 'Could not parse. If it needs login, check cookies.', detail: 'Attempt: clean URL + no cookies\nSSL: UNEXPECTED_EOF_WHILE_READING' }).kind, 'network', 'generic login wording must not hide a transport failure');
  for (const [text, kind] of [
    ['HTTP Error 429: Too Many Requests', 'rateLimit'], ['cookies expired, login required', 'login'],
    ['HTTP Error 403: Forbidden', 'access'], ['No space left on device', 'disk'],
    ['Permission denied', 'permission'], ['Invalid directory', 'folder'],
    ['Requested format is not available', 'format'], ['Connection timed out', 'network'],
    ['yt-dlp executable not found', 'tools'],
  ]) {
    assert.equal(classify({ detail: text, message: 'Download failed' }).kind, kind, text);
  }
  assert.match(classify({ message: '', detail: 'No space left on device' }).message, /disk space/i, 'classified failures carry an actionable message');

  console.log('PASS: file options are forwarded, replacement needs confirmation, failures are classified with actionable messages.');
})().catch(error => { console.error(error); process.exitCode = 1 });
