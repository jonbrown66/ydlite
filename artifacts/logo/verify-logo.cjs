const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');

(async () => {
  const browser = await chromium.launch({ headless: true, channel: 'chrome' });
  try {
    const page = await browser.newPage({ viewport: { width: 1180, height: 780 }, reducedMotion: 'reduce' });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto('http://127.0.0.1:1425/artifacts/logo/review.html');
    assert.equal(await page.locator('img').evaluateAll(images => images.every(image => image.complete && image.naturalWidth > 0)), true);
    await page.screenshot({ path: 'artifacts/logo/logo-review.png' });
    await page.goto('http://127.0.0.1:1426');
    await page.locator('.landing-nav .brand-mark img').waitFor();
    assert.equal(await page.locator('.landing-nav .brand-mark img').evaluate(image => image.complete && image.naturalWidth > 0), true);
    const favicon = await page.locator('link[rel="icon"]').getAttribute('href');
    assert.equal((await page.request.get(new URL(favicon, page.url()).href)).status(), 200);
    await page.locator('.landing-nav').screenshot({ path: 'artifacts/logo/website-nav.png' });
    const startup = await browser.newPage({ viewport: { width: 1180, height: 780 }, reducedMotion: 'reduce' });
    await startup.route('**/src/main-app.ts*', route => route.fulfill({ contentType: 'application/javascript', body: 'export {};' }));
    await startup.goto('http://127.0.0.1:1425');
    const startupLogo = startup.locator('.startup-mark img');
    await startupLogo.waitFor();
    assert.equal(await startupLogo.evaluate(image => image.complete && image.naturalWidth > 0), true);
    await startup.locator('.startup-titlebar').screenshot({ path: 'artifacts/logo/startup-nav.png' });
    assert.deepEqual(errors, []);
    const ico = fs.readFileSync('src-tauri/icons/icon.ico');
    assert.equal(ico.readUInt16LE(0), 0);
    assert.equal(ico.readUInt16LE(2), 1);
    const sizes = Array.from({ length: ico.readUInt16LE(4) }, (_, i) => ico[6 + i * 16] || 256);
    for (const size of [16, 32, 48, 256]) assert(sizes.includes(size), `missing Windows icon size ${size}`);
    assert.equal(fs.readFileSync('public/app-icon.svg', 'utf8'), fs.readFileSync('src/assets/brand-mark.svg', 'utf8'));
    console.log('PASS: small-size logo sheet, app/website image loading, favicon 200, multi-resolution Windows ICO, matching SVG source, no runtime errors');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
