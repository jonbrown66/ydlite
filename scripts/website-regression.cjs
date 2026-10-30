const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const base = process.env.WEBSITE_TEST_URL || 'http://127.0.0.1:1427';
(async()=>{
 const browser = await chromium.launch({headless:true,channel:process.env.PLAYWRIGHT_CHANNEL || 'chrome'});
 try {
  const page = await browser.newPage({viewport:{width:1440,height:1000},reducedMotion:'reduce'});
  const errors = [];
  page.on('pageerror',e=>errors.push(e.message));
  page.on('response',r=>{if(r.status()>=400) errors.push(`${r.status()} ${r.url()}`);});
  await page.goto(base);
  await page.getByRole('heading',{level:1}).waitFor();
  assert.equal(await page.locator('meta[name="description"]').count(),1);
  assert.equal(await page.locator('meta[property="og:image"]').count(),1);
  assert.equal(await page.locator('html').getAttribute('lang'),'en');
  for (const width of [1440,1180,1024,900,768,760,600,390,320]) {
   await page.setViewportSize({width,height:900});
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true,'overflow at '+width);
  }
  for (const name of ['Download','History','Settings']) {
   await page.getByRole('tab',{name,exact:true}).click();
   await page.waitForFunction(()=>{const img=document.querySelector('.hero-shot.is-active');return img?.complete && img.naturalWidth>0;});
  }
  await page.getByRole('tab',{name:'Download',exact:true}).click();
  await page.getByRole('tab',{name:'Download',exact:true}).focus();
  await page.keyboard.press('ArrowRight');
  assert.equal(await page.getByRole('tab',{name:'History',exact:true}).getAttribute('aria-selected'),'true');
  assert.equal(await page.locator('.hero-shot.is-active').getAttribute('alt'),'YDLite download history with saved files and folder actions');
  await page.keyboard.press('Home');
  assert.equal(await page.getByRole('tab',{name:'Download',exact:true}).getAttribute('aria-selected'),'true');
  assert.equal(await page.locator('.stage-viewport').evaluate(el=>el.tagName),'DIV');
  assert.equal(await page.locator('.screenshot-dialog').count(),0);
  await page.setViewportSize({width:390,height:844});
  const menu = page.locator('.menu-toggle');
  assert.equal(await page.locator('.nav-download').isVisible(),false,'mobile header keeps its compact layout');
  await menu.focus(); await page.keyboard.press('Enter'); await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.activeElement?.getAttribute('href')),'#features','expanded mobile menu must be next in tab order');
  await page.keyboard.press('Escape');
  assert.equal(await menu.getAttribute('aria-expanded'),'false');
  assert.equal(await menu.evaluate(el=>el===document.activeElement),true);
  await page.getByRole('link',{name:'System requirements'}).click();
  assert.equal(await page.locator('.faq-item h3 button').first().getAttribute('aria-expanded'),'true');
  const firstFaq = page.locator('.faq-item h3 button').first();
  await firstFaq.focus(); await page.keyboard.press('Enter');
  assert.equal(await firstFaq.getAttribute('aria-expanded'),'false');
  await page.keyboard.press('Enter');
  assert.equal(await firstFaq.getAttribute('aria-expanded'),'true');
  assert.equal(await page.locator('body').innerText().then(text=>text.includes('·')),false,'no middle-dot separators in page copy');
  const fontSizes = await page.locator('.landing-page').evaluate(root=>[...new Set([...root.querySelectorAll('*')].filter(el=>el.getClientRects().length && el.textContent.trim()).map(el=>getComputedStyle(el).fontSize))]);
  assert.ok(fontSizes.length<=4,`too many font sizes: ${fontSizes.join(', ')}`);
  assert.ok(fontSizes.every(size=>parseFloat(size)>=12),'text must be at least 12px');
  for (const filename of ['YDLite_0.2.0_x64-setup.exe','YDLite_0.2.0_x64_en-US.msi']) {
   const response = await page.request.get(`${base}/downloads/${filename}`);
   assert.equal(response.status(),200);
   assert.deepEqual(await response.body(),fs.readFileSync(`public/downloads/${filename}`));
  }
  const artifactDir = process.env.WEBSITE_ARTIFACT_DIR;
  if (artifactDir) {
   fs.mkdirSync(artifactDir,{recursive:true});
   await page.getByRole('link',{name:'YDLite home',exact:true}).click();
   await page.screenshot({path:artifactDir + '/website-mobile.png',fullPage:true});
   await page.setViewportSize({width:1440,height:1000});
   await page.getByRole('link',{name:'YDLite home',exact:true}).click();
   await page.screenshot({path:artifactDir + '/website-desktop.png',fullPage:true});
  }
  const fallback = await browser.newPage({javaScriptEnabled:false});
  await fallback.goto(base);
  assert.equal(await fallback.locator('noscript a[download]').count(),2);
  assert.ok((await fallback.locator('body').innerText()).includes('Download the installer (EXE)'));
  assert.deepEqual(errors,[]);
  console.log('PASS: download-only website, 9 widths, loaded screenshots, keyboard tabs, non-interactive screenshots, mobile navigation tab order, FAQ, four text sizes, no middle-dot separators, installer bytes, metadata and no-JavaScript downloads.');
 } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
