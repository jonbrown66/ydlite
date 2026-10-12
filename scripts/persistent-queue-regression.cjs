const assert = require('node:assert/strict');
const {setupQueue,tick} = require('./queue-test-harness.cjs');
const options = {dir:'C:/downloads',mode:'p720',cookieSource:{type:'file',path:'C:/cookies.txt'}};
const key = 'ydlite.downloadQueue.v1';

(async()=>{
  {
    const h = setupQueue(); h.controlParsing(true);
    const input = Array.from({length:20},(_,i)=>`https://example.com/${i}`).join('\n') + '\nhttps://example.com/0#duplicate\nnot-a-url\nfile:///private';
    const result = h.queue.importLinks(input,options);
    assert.equal(result.ids.length,20); assert.equal(result.duplicates,1); assert.equal(result.invalid,2);
    const analysis = h.queue.analyze(result.ids); await tick();
    for (let i=0;i<20;i++) {
      assert.equal(h.parses.length,i+1,'analyze links serially');
      if (i===4 || i===15) h.parses[i].reject({message:'Network error',detail:'timed out secret signed url'});
      else h.parses[i].resolve({title:`Clip ${i}`,resolvedUrl:`https://example.com/${i}`,cookieSource:options.cookieSource});
      await tick();
    }
    await analysis;
    assert.equal(h.queue.failed.length,2); assert.equal(h.queue.ready.length,18); assert.equal(h.calls.length,0,'analysis must not auto-download');
    assert.ok(!h.data.get(key).includes('secret signed url'),'diagnostics must not persist');
    const saved = Object.fromEntries(h.data);
    const restored = setupQueue(saved);
    assert.equal(restored.queue.jobs.length,20); assert.equal(restored.queue.failed.length,2); assert.equal(restored.queue.running,false); assert.equal(restored.calls.length,0);
    assert.equal(restored.queue.jobs[0].dir,options.dir); assert.equal(restored.queue.jobs[0].mode,'p720'); assert.equal(restored.queue.jobs[0].cookieSource.path,options.cookieSource.path);
    const retry = restored.queue.retry([restored.queue.failed[0].id]); await tick();
    assert.equal(restored.parses.length,1); assert.equal(restored.calls.length,1);
    assert.equal(restored.calls[0].request.url,'https://example.com/4');
    restored.emit({status:'finished',filePath:'C:/downloads/4.mp4'}); restored.calls[0].resolve(); await retry;
    assert.equal(restored.queue.failed.length,1); assert.equal(restored.history.downloads.length,1); assert.equal(restored.calls.length,1,'retry only selected failure');
  }
  {
    const h=setupQueue();
    const ids=h.queue.add([1,2,3].map(n=>({url:`https://example.com/${n}`,title:`Clip ${n}`})),options);
    const run=h.queue.run(); await tick();
    assert.equal(h.calls.length,1);
    h.emit({status:'error'}); await tick(); assert.equal(h.calls.length,1,'terminal event must not advance before IPC settles');
    h.calls[0].reject({message:'HTTP Error 403'}); await tick();
    assert.equal(h.calls.length,2); assert.equal(h.queue.jobs[0].error.kind,'access');
    h.emit({status:'finished',filePath:'C:/downloads/2.mp4'}); await tick(); assert.equal(h.calls.length,2);
    h.calls[1].resolve(); await tick(); h.calls[2].reject(new Error('spawn failed')); await run;
    assert.equal(h.queue.failed.length,2); assert.equal(h.queue.progress.status,'error'); assert.equal(h.history.downloads[0].title,'Clip 2');
    const retry=h.queue.retry([ids[2]]); await tick(); h.calls[3].resolve(); await retry;
    assert.equal(h.calls.length,4); assert.equal(h.calls[3].request.url,'https://example.com/3'); assert.equal(h.queue.failed.length,1);
  }
  {
    const h=setupQueue(); h.controlCancel(true);
    const ids=h.queue.add([1,2,3].map(n=>({url:`https://example.com/${n}`})),options);
    const run=h.queue.run(); await tick();
    await h.queue.cancel(ids[2]); assert.equal(h.queue.jobs[2].status,'cancelled');
    const cancel=h.queue.cancel(ids[0]); h.calls[0].reject(new Error('cancelled'));
    await tick(); assert.equal(h.calls.length,1,'wait for cancellation IPC before starting next process');
    h.cancels[0].resolve(); await cancel; await tick();
    assert.equal(h.calls.length,2,'cancel current task must continue next task');
    assert.equal(h.queue.jobs[0].status,'cancelled'); h.calls[1].resolve(); await run;
    assert.equal(h.calls.length,2,'cancelled waiting task must not start'); assert.equal(h.queue.failed.length,0);
  }
  {
    const h=setupQueue(); const ids=h.queue.add([1,2].map(n=>({url:`https://example.com/${n}`})),options);
    const run=h.queue.run(); await tick(); h.queue.pause(); h.emit({status:'finished',filePath:'C:/downloads/1.mp4'}); h.calls[0].resolve(); await run;
    assert.equal(h.queue.jobs[0].status,'completed'); assert.equal(h.queue.jobs[1].status,'queued'); assert.equal(h.calls.length,1);
    const restored=setupQueue(Object.fromEntries(h.data)); const resume=restored.queue.run(); await tick();
    assert.equal(restored.calls[0].request.url,'https://example.com/2'); restored.calls[0].resolve(); await resume;
    assert.equal(restored.calls.length,1,'completed task never repeats on continuation');
    const running=setupQueue(); running.queue.add([{url:'https://example.com/inflight'}],options); const active=running.queue.run(); await tick();
    const recovered=setupQueue(Object.fromEntries(running.data)); assert.equal(recovered.queue.jobs[0].status,'interrupted'); assert.equal(recovered.calls.length,0);
    running.calls[0].resolve(); await active;
  }
  {
    const h=setupQueue(); h.controlParsing(true);
    const result=h.queue.importLinks('https://example.com/list\nhttps://example.com/bad',options);
    const analysis=h.queue.analyze(result.ids); await tick();
    h.parses[0].resolve({title:'Playlist',isPlaylist:true,entries:[{url:'https://example.com/a',title:'A'},{url:'https://example.com/a',title:'Duplicate A'},{url:'https://example.com/b',title:'B'}]}); await tick();
    assert.equal(h.parses.length,2); h.parses[1].reject(new Error('unsupported url')); await analysis;
    assert.equal(h.queue.jobs.length,3); assert.equal(h.queue.ready.length,2); assert.equal(h.queue.failed.length,1);
    const retryId=h.queue.failed[0].id; const retry=h.queue.analyze([retryId],{type:'none'}); await tick(); await h.queue.cancel(retryId);
    h.parses[2].resolve({title:'late result',resolvedUrl:'https://example.com/bad'}); await retry;
    assert.equal(h.queue.jobs.find(job=>job.id===retryId).status,'cancelled','discard cancelled parser result');
    assert.equal(h.calls.length,0);
  }
  {
    const h=setupQueue(); h.queue.add([1,2].map(n=>({url:`https://example.com/${n}`})),options); const run=h.queue.run(); await tick();
    h.failStorage(true); h.emit({status:'finished',filePath:'C:/downloads/1.mp4'}); h.calls[0].resolve(); await run;
    assert.ok(h.queue.storageError); assert.equal(h.calls.length,1,'saving failure stops queue'); assert.equal(h.queue.jobs[0].status,'completed','retain real completed result in memory');
    h.failStorage(false); h.queue.saveAgain(); assert.equal(h.queue.storageError,'');
    assert.equal(setupQueue(Object.fromEntries(h.data)).queue.jobs[0].status,'completed');
    const blocked=setupQueue(); blocked.failStorage(true); assert.throws(()=>blocked.queue.add([{url:'https://example.com/new'}],options)); assert.equal(blocked.queue.jobs.length,0); assert.equal(blocked.calls.length,0);
  }
  {
    const h=setupQueue(); h.controlCancel(true);
    const ids=h.queue.add([1,2].map(n=>({url:`https://example.com/${n}`})),options);
    const run=h.queue.run(); await tick();
    const firstAttempt=h.calls[0].request.requestId;
    const cancellation=h.queue.cancel(ids[0]);
    h.calls[0].resolve({cancelled:false,filePath:'C:/downloads/1.mp4'});
    h.cancels[0].reject(new Error('Unable to cancel'));
    await assert.rejects(cancellation); await tick();
    assert.equal(h.queue.jobs[0].status,'completed','failed cancellation must preserve successful IPC result');
    assert.equal(h.history.downloads[0].filePath,'C:/downloads/1.mp4','IPC result supplies history even without completion event');
    h.emit({requestId:firstAttempt,status:'cancelled',filePath:'C:/downloads/wrong.mp4'});
    assert.equal(h.queue.progress.status,'starting','late previous task event must not overwrite the next task');
    const secondAttempt=h.calls[1].request.requestId;
    assert.notEqual(firstAttempt,secondAttempt);
    h.emit({requestId:secondAttempt,status:'downloading',percent:45}); assert.equal(h.queue.progress.percent,45);
    h.calls[1].resolve({cancelled:true,filePath:null}); await run;
    assert.equal(h.queue.jobs[1].status,'cancelled','cancelled IPC result works without terminal event');
    const retry=h.queue.retry([ids[1]]); await tick();
    assert.notEqual(h.calls[2].request.requestId,secondAttempt,'retry uses a fresh attempt identifier');
    h.emit({requestId:secondAttempt,status:'finished',filePath:'C:/downloads/stale.mp4'});
    h.calls[2].resolve({cancelled:false,filePath:'C:/downloads/2.mp4'}); await retry;
    assert.equal(h.queue.jobs[1].filePath,'C:/downloads/2.mp4');
  }
  {
    const h=setupQueue(); h.controlParsing(true);
    const ids=h.queue.importLinks('https://example.com/short\nhttps://example.com/full',options).ids;
    const analyze=h.queue.analyze(ids,{type:'none'}); await tick();
    h.parses[0].resolve({title:'Same clip',resolvedUrl:'https://example.com/full'}); await tick();
    h.parses[1].resolve({title:'Same clip',resolvedUrl:'https://example.com/full'}); await analyze;
    assert.equal(h.queue.jobs.length,1,'deduplicate resolved links as well as pasted input');
    assert.equal(h.parses[0].request.options.cookieSource.type,'none','reanalyze uses current cookie settings');
    assert.equal(h.queue.jobs[0].status,'queued');
  }
  {
    const h=setupQueue({}, {realActivity:true});
    const activity=h.load('@/stores/activity').useActivityStore(); await activity.startListening();
    h.queue.add([{url:'https://example.com/one'}],options); const run=h.queue.run(); await tick();
    const requestId=h.calls[0].request.requestId;
    h.emit({requestId,status:'downloading',percent:50}); assert.equal(activity.visibleTasks[0].percent,50);
    h.calls[0].resolve({cancelled:false,filePath:'C:/downloads/one.mp4'}); await run;
    assert.equal(activity.visibleTasks[0].state,'completed');
    h.emit({requestId,status:'downloading',percent:5}); assert.equal(activity.visibleTasks[0].state,'completed','late progress must not revive a finished global activity');
    activity.stopListening(); h.queue.dispose();
  }
  {
    const corrupt='{bad'; const h=setupQueue({[key]:corrupt}); assert.ok(h.queue.storageError);
    assert.throws(()=>h.queue.add([{url:'https://example.com/new'}],options)); h.queue.saveAgain(); assert.equal(h.data.get(key),corrupt);
    const full=setupQueue(); full.queue.add(Array.from({length:500},(_,n)=>({url:`https://example.com/${n}`})),options);
    assert.throws(()=>full.queue.add([{url:'https://example.com/extra'}],options)); assert.equal(full.queue.jobs.length,500);
  }
  console.log('PASS: 20-link independent analysis, deduplication, durable options/status, scoped retry, cancellation race, pause/restart, playlist expansion, cancelled parsing, storage failure and corrupt-data preservation.');
})().catch(error=>{console.error(error);process.exitCode=1});
