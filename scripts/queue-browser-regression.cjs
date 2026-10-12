const {chromium} = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');

(async()=>{
  const browser=await chromium.launch({headless:true,channel:process.env.PLAYWRIGHT_CHANNEL || 'chrome'});
  try {
    const page=await browser.newPage({viewport:{width:1180,height:780},reducedMotion:'reduce'});
    const errors=[]; page.on('pageerror',error=>errors.push(error.message));
    await page.addInitScript(()=>{if(!localStorage.getItem('ydlite.downloadPreferences')) localStorage.setItem('ydlite.downloadPreferences',JSON.stringify({defaultDir:'C:/downloads',defaultQuality:'p720',cookiesFilePath:'C:/cookies.txt',clipboardSuggestions:false}));});
    await page.route(/\/src\/main-app\.ts(?:\?[^?]*)?$/,route=>route.request().url().includes('queue-real') ? route.continue() : route.fulfill({contentType:'application/javascript',body:`
      import { mockIPC, mockWindows } from '/node_modules/@tauri-apps/api/mocks.js';
      import { emit } from '/node_modules/@tauri-apps/api/event.js';
      mockWindows('main'); let finish, fail; window.requests=[]; window.parseRequests=[]; window.cancelCount=0; window.parseFailures=true;
      window.finishTask=async(status='finished')=>{
        const request=window.requests.at(-1); const resolve=finish,reject=fail; finish=fail=undefined;
        const filePath=status==='finished'?request.dir+'/'+request.url.split('/').at(-1)+'.mp4':null;
        await emit('download://progress',{requestId:request.requestId,status,filePath});
        if(status==='error') reject({message:'Connection failed',detail:'HTTP error 502'}); else resolve({cancelled:status==='cancelled',filePath});
      };
      mockIPC((cmd,args)=>{
        if(cmd==='check_dependencies')return {ytdlp_ok:true,ffmpeg_ok:true,ytdlp_path:'yt-dlp',ffmpeg_path:'ffmpeg',ytdlp_update_available:false};
        if(cmd==='parse_video'){
          window.parseRequests.push(args.request);
          const number=args.request.url.split('/').at(-1);
          if(window.parseFailures && ['4','15'].includes(number)) throw {message:'Access denied',detail:'HTTP Error 403'};
          return {title:'Clip '+number,resolvedUrl:args.request.url,originalUrl:args.request.url,site:'example',parseStrategy:'default',cookieSource:args.request.options.cookieSource,formats:[]};
        }
        if(cmd==='start_download') {window.requests.push(args.request); return new Promise((resolve,reject)=>{finish=resolve;fail=reject});}
        if(cmd==='cancel_download'){window.cancelCount++;return;}
        if(cmd==='plugin:dialog|open')return 'C:/new-downloads';
        return false;
      },{shouldMockEvents:true});
      await import('/src/main-app.ts?queue-real');
    `}));
    await page.goto(process.env.FRONTEND_TEST_URL || 'http://127.0.0.1:1423');
    const singleTab=page.getByRole('tab',{name:'Single link',exact:true});
    const multipleTab=page.getByRole('tab',{name:'Multiple links',exact:true});
    const singleInput=page.getByRole('textbox',{name:'Video or audio URL'});
    const batchInput=page.getByRole('textbox',{name:'Video, audio or playlist links'});
    assert.equal(await singleTab.getAttribute('aria-selected'),'true');
    assert.equal(await singleTab.getAttribute('tabindex'),'0'); assert.equal(await multipleTab.getAttribute('tabindex'),'-1');
    assert.equal(await singleInput.isVisible(),true); assert.equal(await batchInput.isVisible(),false);
    assert.equal(await page.getByRole('tabpanel').count(),1);
    await singleInput.fill('https://example.com/preview');
    await page.getByRole('button',{name:'Analyze',exact:true}).click();
    await page.getByRole('heading',{name:'Clip preview',exact:true}).waitFor();
    await page.getByRole('combobox',{name:'Format',exact:true}).click();
    await page.getByRole('option',{name:'Audio only · MP3',exact:true}).click();
    fs.mkdirSync('artifacts/download-tabs',{recursive:true});
    await page.screenshot({path:'artifacts/download-tabs/single.png'});
    await singleTab.focus(); await page.keyboard.press('ArrowRight');
    assert.equal(await multipleTab.getAttribute('aria-selected'),'true');
    assert.equal(await multipleTab.evaluate(el=>document.activeElement===el),true);
    assert.equal(await page.getByRole('tabpanel').count(),1); assert.equal(await singleInput.isVisible(),false);
    await batchInput.fill('https://example.com/draft');
    await multipleTab.focus(); await page.keyboard.press('Home');
    assert.equal(await singleTab.getAttribute('aria-selected'),'true');
    assert.equal(await singleInput.inputValue(),'https://example.com/preview');
    assert.ok((await page.getByRole('combobox',{name:'Format',exact:true}).innerText()).includes('MP3'));
    assert.equal(await page.getByRole('heading',{name:'Clip preview',exact:true}).isVisible(),true,'retain parsed single-link results across tabs');
    assert.equal(await page.locator('#batch-links').evaluate(el=>{el.focus();return document.activeElement===el}),false,'hidden panel controls must not receive focus');
    await singleTab.focus(); await page.keyboard.press('End'); assert.equal(await multipleTab.getAttribute('aria-selected'),'true');
    assert.equal(await batchInput.inputValue(),'https://example.com/draft','retain unsubmitted batch input across tabs');
    await multipleTab.focus(); await page.keyboard.press('ArrowRight'); assert.equal(await singleTab.getAttribute('aria-selected'),'true');
    await page.keyboard.press('ArrowLeft'); assert.equal(await multipleTab.getAttribute('aria-selected'),'true');
    await page.screenshot({path:'artifacts/download-tabs/multiple.png'});
    await page.locator('.download-page').evaluate(el=>{
      const data=new DataTransfer(); data.setData('text/uri-list','# dropped links\nhttps://example.com/drop-1\nhttps://example.com/drop-2');
      el.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));
    });
    assert.equal(await batchInput.inputValue(),'https://example.com/draft\nhttps://example.com/drop-1\nhttps://example.com/drop-2','drop appends to the active batch input');
    await singleTab.click(); assert.equal(await singleInput.inputValue(),'https://example.com/preview','batch drop must not replace the hidden single-link input');
    await multipleTab.click();
    const text=Array.from({length:20},(_,n)=>'https://example.com/'+n).join('\n')+'\nhttps://example.com/0#again\nwrong-link';
    await page.getByRole('textbox',{name:'Video, audio or playlist links'}).fill(text);
    await page.getByRole('button',{name:'Import and analyze',exact:true}).click();
    await page.getByText('20 added · 1 duplicates skipped · 1 invalid lines skipped',{exact:true}).waitFor();
    await page.waitForFunction(()=>document.querySelectorAll('[data-status="queued"]').length===18 && document.querySelectorAll('[data-status="failed"]').length===2);
    assert.equal(await page.getByRole('textbox',{name:'Video, audio or playlist links'}).inputValue(),'wrong-link','invalid lines remain editable');
    const saved=await page.evaluate(()=>JSON.parse(localStorage.getItem('ydlite.downloadQueue.v1')).jobs);
    assert.equal(saved.length,20); assert.ok(saved.every(job=>job.mode==='p720'&&job.dir==='C:/downloads'));
    for(const width of [1180,900,800,720]) {
      await page.setViewportSize({width,height:780});
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'window overflow at '+width);
      assert.equal(await page.locator('.saved-queue').evaluate(el=>el.scrollWidth>el.clientWidth+1),false,'queue overflow at '+width);
    }
    fs.mkdirSync('artifacts/download-queue',{recursive:true});
    await page.screenshot({path:'artifacts/download-queue/queue-720.png'});
    await page.screenshot({path:'artifacts/download-tabs/multiple-720.png'});
    await page.setViewportSize({width:1180,height:780});
    await page.getByRole('button',{name:'Continue 18 tasks',exact:true}).click();
    await page.waitForFunction(()=>window.requests.length===1);
    const second=page.locator('[data-job-id="'+saved[1].id+'"]');
    await second.getByRole('button',{name:'Cancel Clip 1',exact:true}).click();
    await page.locator('[data-job-id="'+saved[0].id+'"]').getByRole('button',{name:'Cancel Clip 0',exact:true}).click();
    assert.equal(await page.evaluate(()=>window.cancelCount),1);
    await singleTab.click();
    assert.equal(await page.locator('[data-status="cancelling"]').count(),1,'queue remains visible during single-link input');
    assert.equal(await page.evaluate(()=>window.requests.length),1,'tab switching must not start or cancel another task');
    await multipleTab.click();
    await page.evaluate(()=>window.finishTask('cancelled'));
    await page.waitForFunction(()=>window.requests.length===2);
    assert.equal(await page.evaluate(()=>window.requests[1].url),'https://example.com/2','single cancellation must continue next eligible task');
    await page.getByRole('button',{name:'Pause after current',exact:true}).click();
    await page.evaluate(()=>window.finishTask());
    await page.getByRole('button',{name:/^Continue \d+ tasks$/}).waitFor();
    assert.equal(await page.evaluate(()=>window.requests.length),2);
    await page.reload(); await singleTab.waitFor();
    assert.equal(await page.evaluate(()=>window.requests.length),0,'restart must not auto-download');
    assert.equal(await page.locator('[data-status="completed"]').count(),1);
    assert.equal(await page.locator('[data-status="cancelled"]').count(),2);
    const failed=page.locator('[data-job-id="'+saved[4].id+'"]');
    await failed.locator('summary').click();
    await failed.getByRole('button',{name:'Edit folder and format',exact:true}).click();
    await failed.getByRole('combobox',{name:'Task format'}).click();
    await page.getByRole('option',{name:'Audio only · MP3',exact:true}).click();
    await failed.getByRole('button',{name:'Choose folder',exact:true}).click();
    await failed.getByRole('button',{name:'Save task',exact:true}).click();
    await page.evaluate(()=>window.parseFailures=false);
    await failed.getByRole('button',{name:'Retry https://example.com/4',exact:true}).click();
    await page.waitForFunction(()=>window.requests.length===1);
    assert.equal(await page.evaluate(()=>window.requests[0].url),'https://example.com/4');
    assert.equal(await page.evaluate(()=>window.requests[0].mode),'mp3'); assert.equal(await page.evaluate(()=>window.requests[0].dir),'C:/new-downloads');
    await page.evaluate(()=>window.finishTask()); await failed.locator('.queue-state.completed').waitFor();
    assert.equal(await page.evaluate(()=>window.requests.length),1,'one retry must not download other waiting or failed tasks');
    await page.getByRole('combobox',{name:'Filter download queue'}).click();
    await page.getByRole('option',{name:'Failed (1)',exact:true}).click(); assert.equal(await page.locator('.saved-queue-item').count(),1);
    await page.getByRole('combobox',{name:'Filter download queue'}).click(); await page.getByRole('option',{name:'All tasks (20)',exact:true}).click();
    await page.getByRole('button',{name:/^Continue \d+ tasks$/}).click(); await page.waitForFunction(()=>window.requests.length===2);
    await page.reload(); await page.getByText('Some tasks were interrupted.',{exact:false}).waitFor();
    assert.equal(await page.evaluate(()=>window.requests.length),0); assert.equal(await page.locator('[data-status="interrupted"]').count(),1);
    await page.screenshot({path:'artifacts/download-queue/queue-recovered.png'});
    assert.equal(await page.evaluate(()=>JSON.parse(localStorage.getItem('ydlite.history')).length),2);
    assert.deepEqual(errors,[]);
    console.log('PASS: accessible tab switching with retained drafts/results, batch keyboard/input, 20 links with independent failures, format/folder persistence, responsive queue, per-item and waiting cancellation, pause/reload, scoped retry/edit, filters, interrupted recovery and history; no runtime errors.');
  } finally {await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1});
