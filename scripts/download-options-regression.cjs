const assert = require('node:assert/strict');
const {setupQueue, tick} = require('./queue-test-harness.cjs');

(async () => {
  const h = setupQueue();
  const options = {dir:'C:/downloads', mode:'custom', formatId:'video-h264', cookieSource:{type:'none'}, fileOptions:{naming:'authorTitle',conflict:'overwrite',quality:'quality',audioFormatId:'audio-en'}};
  const [id] = h.queue.add([{url:'https://example.com/video'}], options);
  const saved = Object.fromEntries(h.data);
  const restored = setupQueue(saved);
  assert.equal(restored.queue.jobs[0].fileOptions.audioFormatId, 'audio-en');
  const declined = restored.queue.run(); await tick();
  assert.equal(restored.queue.overwriteCount,1);
  assert.equal(restored.calls.length,0,'no download may start before replacement confirmation');
  restored.queue.confirmOverwrite(false); await declined;
  assert.equal(restored.queue.jobs[0].status,'queued','declining preserves the saved task');
  const approved = restored.queue.run(); await tick();
  restored.queue.confirmOverwrite(true); await tick();
  assert.equal(restored.calls.length,1);
  assert.deepEqual(JSON.parse(JSON.stringify(restored.calls[0].request.fileOptions)),options.fileOptions);
  assert.equal(restored.calls[0].request.overwriteConfirmed,true);
  restored.calls[0].resolve({cancelled:false,filePath:'C:/downloads/existing.mp4',skipped:true});
  await approved;
  assert.equal(restored.queue.jobs[0].skipped,true);
  assert.equal(setupQueue(Object.fromEntries(restored.data)).queue.jobs[0].skipped,true);
  const classify = h.load('@/lib/downloadQueue').classifyQueueError;
  assert.equal(classify({message:'Could not parse. If it needs login, check cookies.',detail:'Attempt: clean URL + no cookies\nSSL: UNEXPECTED_EOF_WHILE_READING'}).kind,'network','generic login wording must not hide a transport failure');
  for (const [text, kind] of [['HTTP Error 429: Too Many Requests','rateLimit'],['cookies expired, login required','login'],['HTTP Error 403: Forbidden','access'],['No space left on device','disk'],['Permission denied','permission'],['Invalid directory','folder'],['Requested format is not available','format'],['Connection timed out','network'],['yt-dlp executable not found','tools']]) {
    assert.equal(classify({detail:text,message:'Download failed'}).kind,kind);
  }
  const page = h.page(`globalThis.test = {video, availableFormats, showDetailedFormats, formatOptions, fileOptions, selectedFormatId, queueOptions, audioOptions};`);
  page.video.value = {title:'Test',formats:[{formatId:'avc',height:1080,fps:30,ext:'mp4',vcodec:'avc1',acodec:'none'},{formatId:'vp9',height:1080,fps:60,ext:'webm',vcodec:'vp9',acodec:'none'},{formatId:'audio-en',ext:'m4a',vcodec:'none',acodec:'aac',language:'en',bitrateKbps:128}]};
  assert.equal(page.availableFormats.value.length,2,'same resolution must not hide different codecs or frame rates');
  assert.equal(page.formatOptions.value.length,3,'simple choices remain the default');
  page.showDetailedFormats.value = true;
  assert.equal(page.formatOptions.value.length,5);
  assert.match(page.availableFormats.value[1].label,/60 fps/);
  assert.match(page.audioOptions.value[1].label,/en/);
  page.selectedFormatId.value = 'avc'; page.showDetailedFormats.value = false;
  assert.ok(page.formatOptions.value.some(f=>f.value==='avc'),'collapsing details must keep the selected custom format visible');
  console.log('PASS: file/format options survive restart; explicit overwrite confirmation; skipped outcome; distinct codecs/fps/audio language; actionable error categories.');
})().catch(error => {console.error(error);process.exitCode=1});
