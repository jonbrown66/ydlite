// Run against `npm run dev -- --mode tauri` with PLAYWRIGHT_MODULE pointing to Playwright.
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const artifactDir = process.env.FRONTEND_ARTIFACT_DIR || 'test-output/frontend';

(async () => {
  const browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'chrome' });
  try {
    const page = await browser.newPage({ viewport: { width: 1180, height: 780 }, reducedMotion: 'reduce' });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => localStorage.setItem('ydlite.downloadPreferences', JSON.stringify({
      defaultDir: 'C:/test-downloads', cookiesFilePath: '', defaultQuality: 'best', clipboardSuggestions: false,
      fileOptions: { naming: 'title', conflict: 'skip', quality: 'compatible', audioFormatId: null },
    })));
    await page.route(/\/src\/main-app\.ts(?:\?[^?]*)?$/, route => route.request().url().includes('regression-real') ? route.continue() : route.fulfill({ contentType: 'application/javascript', body: `
      import { mockIPC, mockWindows } from '/node_modules/@tauri-apps/api/mocks.js';
      import { emit } from '/node_modules/@tauri-apps/api/event.js';
      let finishDownload, requestId;
      window.testEmit = async (name, payload) => { await emit(name, {requestId, ...payload}); if (['finished','cancelled','paused','error'].includes(payload.status)) { finishDownload?.({cancelled:payload.status==='cancelled',paused:payload.status==='paused',filePath:payload.filePath || null}); finishDownload = undefined; } };
      window.destroyCount = 0;
      window.testMinimizeCount = 0;
      window.testMaximized = false;
      window.pauseCount = 0;
      mockWindows('main');
      mockIPC((cmd, args) => {
        if (cmd === 'plugin:window|minimize') { window.testMinimizeCount++; return; }
        if (cmd === 'plugin:window|maximize') { window.testMaximized = true; return; }
        if (cmd === 'plugin:window|unmaximize') { window.testMaximized = false; return; }
        if (cmd === 'plugin:window|is_maximized') return window.testMaximized;
        if (cmd === 'plugin:dialog|open') return 'C:/test-downloads/folder';
        if (cmd === 'pause_download') { window.pauseCount++; return; }
        if (cmd === 'cancel_download') return;
        if (cmd === 'start_download') { requestId = args.request.requestId; return new Promise(resolve => { finishDownload = resolve; }); }
        if (cmd === 'plugin:window|close') return emit('tauri://close-requested');
        if (cmd === 'plugin:window|destroy') { window.destroyCount++; return; }
        if (cmd === 'check_dependencies') return { ytdlp_ok: true, ffmpeg_ok: true, ytdlp_path: 'yt-dlp', ffmpeg_path: 'ffmpeg', ytdlp_update_available: false };
        if (cmd === 'check_ytdlp_update') return { ytdlp_latest_version: null, ytdlp_update_available: false };
        if (cmd === 'parse_video') return { title: 'Sample video · A long title for layout verification', uploader: 'Example creator', duration: 124, originalUrl: args.request.url, resolvedUrl: args.request.url, site: 'example', parseStrategy: 'default', isPlaylist: false, formats: [{formatId:'audio',ext:'m4a',vcodec:'none',acodec:'aac',filesize:4194304},{formatId: '720', ext: 'mp4', height: 720, vcodec: 'h264', acodec: 'none',filesize:8388608}] };
        if (cmd === 'get_tools_directory') return 'C:/tools';
        return false;
      }, { shouldMockEvents: true });
      await import('/src/main-app.ts?regression-real');
    ` }));
    await page.goto(process.env.FRONTEND_TEST_URL || 'http://localhost:1420');
    const close = page.getByRole('button', { name: 'Close window', exact: true });
    await close.waitFor();

    // Window chrome keeps a single focus indicator and its native hover colours.
    await page.getByRole('button', { name: 'Minimize window', exact: true }).click();
    assert.equal(await page.evaluate(() => window.testMinimizeCount), 1);
    await page.getByRole('button', { name: 'Maximize window', exact: true }).click();
    await page.getByRole('button', { name: 'Restore window', exact: true }).waitFor();
    await page.locator('.titlebar-drag-area').dblclick();
    await page.getByRole('button', { name: 'Maximize window', exact: true }).waitFor();
    await close.hover();
    await page.waitForFunction(() => getComputedStyle(document.querySelector('.close-window')).backgroundColor === 'rgb(196, 43, 28)');
    // Keyboard focus, not a programmatic focus call, is what shows :focus-visible.
    await page.locator('body').click({ position: { x: 600, y: 700 } });
    for (let i = 0; i < 6; i++) await page.keyboard.press('Tab');
    assert.equal(await close.evaluate(el => document.activeElement === el && el.matches(':focus-visible')), true, 'Tab reaches the window controls');
    assert.equal(await close.evaluate(el => getComputedStyle(el).outlineWidth), '2px');
    assert.equal(await close.evaluate(el => getComputedStyle(el).boxShadow), 'none', 'Button has one outline without an extra focus ring');

    const assertGroupFocus = async () => assert.equal(await page.getByRole('textbox', { name: 'Video link' }).evaluate(el => {
      const style = getComputedStyle(el);
      const group = getComputedStyle(el.closest('.input-row'));
      return style.outlineWidth === '0px' && group.outlineStyle === 'solid' && group.outlineWidth === '2px' && group.outlineColor !== 'rgba(0, 0, 0, 0)';
    }), true, 'URL input group has exactly one visible outer focus indicator');

    fs.mkdirSync(artifactDir, { recursive: true });
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    const url = page.getByRole('textbox', { name: 'Video link' });
    await url.focus();
    await assertGroupFocus();
    await page.emulateMedia({ forcedColors: 'active' });
    await assertGroupFocus();
    await page.emulateMedia({ forcedColors: 'none' });
    await page.screenshot({ path: artifactDir + '/download-empty.png' });

    // Analyze, choose a format, then download.
    await url.fill('https://example.com/video');
    await page.getByRole('button', { name: 'Go', exact: true }).click();
    await page.getByRole('heading', { name: /Sample video/ }).waitFor();
    await page.locator('.advanced > summary').click();
    const combo = page.getByRole('combobox', { name: 'Format' });
    assert.ok((await combo.innerText()).includes('Best'));
    await page.screenshot({ path: artifactDir + '/download.png' });
    await page.getByRole('button', { name: 'Download', exact: true }).click();
    const progress = page.getByRole('progressbar', { name: 'Download progress', exact: true });
    await page.waitForFunction(() => document.querySelector('[aria-label="Download progress"]')?.getAttribute('aria-valuenow') === '0');

    await page.evaluate(() => window.testEmit('download://progress', { status: 'downloading', percent: 40, speed: '4.0 MiB/s', metrics: { downloadedBytes: 4194304, totalBytes: 10485760, totalIsEstimate: false, speedBytes: 4194304 } }));
    await page.waitForFunction(() => document.querySelector('[aria-label="Download progress"]')?.getAttribute('aria-valuenow') === '40');
    const progressText = await page.locator('.progress-size').innerText();
    assert.ok(progressText.includes('4.0 MiB / 10.0 MiB'), 'size text stays visible');
    assert.ok(progressText.includes('4.0 MiB/s'), 'speed comes from the downloader metrics');
    assert.match(await page.locator('.progress-bar').getAttribute('class'), /is-running/);
    assert.equal(await page.locator('.progress-bar .fill').getAttribute('style'), 'width: 40%;', 'the bar reflects the reported percentage');
    await page.screenshot({ path: artifactDir + '/download-progress.png' });

    // Pause stops the transfer and keeps the progress visible for resuming.
    await page.getByRole('button', { name: 'Pause', exact: true }).click();
    assert.equal(await page.evaluate(() => window.pauseCount), 1);
    await page.evaluate(() => window.testEmit('download://progress', { status: 'paused' }));
    await page.getByRole('button', { name: 'Start', exact: true }).waitFor();
    assert.equal(await page.locator('.progress-status').innerText(), 'Paused');
    assert.match(await page.locator('.progress-bar').getAttribute('class'), /is-paused/, 'the bar switches to the paused treatment');
    assert.equal(await page.locator('.progress-bar .fill').getAttribute('style'), 'width: 40%;', 'a pause keeps the reached position');
    await page.screenshot({ path: artifactDir + '/download-paused.png' });

    await page.getByRole('button', { name: 'Start', exact: true }).click();
    await progress.waitFor();
    await page.evaluate(() => window.testEmit('download://progress', { status: 'finished', filePath: 'C:/test-downloads/first.mp4' }));
    await page.getByRole('button', { name: 'Folder', exact: true }).waitFor();
    assert.equal(await page.locator('.progress-status').innerText(), 'Done');
    await page.locator('.download-actions').screenshot({ path: artifactDir + '/download-completed-actions.png' });

    // Cancel ends the attempt without recording history.
    await page.getByRole('button', { name: 'Download', exact: true }).click();
    await page.evaluate(() => window.testEmit('download://progress', { status: 'downloading', percent: 10 }));
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.evaluate(() => window.testEmit('download://progress', { status: 'cancelled' }));
    await page.getByRole('button', { name: 'Download', exact: true }).waitFor();

    // Errors stay short, with details behind a disclosure.
    await page.evaluate(() => window.testEmit('download://progress', { status: 'error' }));
    await page.getByRole('button', { name: 'Download', exact: true }).click();
    await page.evaluate(() => window.testEmit('download://progress', { status: 'finished', filePath: 'C:/test-downloads/second.mp4' }));
    await page.getByRole('button', { name: 'Folder', exact: true }).waitFor();
    assert.deepEqual(await page.evaluate(() => JSON.parse(localStorage.getItem('ydlite.history')).map(item => item.filePath).sort()), ['C:/test-downloads/first.mp4', 'C:/test-downloads/second.mp4']);

    // History: search, clear, and the action menu's keyboard focus.
    await page.getByRole('link', { name: 'History', exact: true }).click();
    await page.getByRole('searchbox', { name: 'Search history' }).fill('no-match');
    await page.getByText('No matches.', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Clear', exact: true }).click();
    assert.equal(await page.locator('.task-row').count(), 2);
    await page.getByRole('button', { name: 'More actions', exact: true }).first().click();
    await page.keyboard.press('ArrowDown');
    const menuFocusVisible = () => page.evaluate(() => {
      const item = document.activeElement;
      const style = getComputedStyle(item);
      return item.getAttribute('role') === 'menuitem' && style.outlineStyle === 'solid' && style.outlineWidth === '2px';
    });
    assert.equal(await menuFocusVisible(), true, 'menu keyboard focus has a visible outline');
    await page.emulateMedia({ forcedColors: 'active' });
    assert.equal(await menuFocusVisible(), true, 'menu keyboard focus remains visible in high contrast');
    await page.emulateMedia({ forcedColors: 'none' });
    await page.keyboard.press('Escape');
    await page.screenshot({ path: artifactDir + '/tasks.png' });

    // Settings: unsaved changes are guarded and saving persists.
    await page.getByRole('link', { name: 'Settings', exact: true }).click();
    await page.locator('.settings-actions').waitFor();
    assert.equal(await page.getByRole('button', { name: 'Save', exact: true }).isDisabled(), true);
    await page.getByRole('textbox', { name: 'Save folder' }).fill('C:/changed');
    await page.getByRole('link', { name: 'Download', exact: true }).click();
    const dialog = page.getByRole('dialog');
    await dialog.waitFor();
    assert.equal(await page.locator('#app').evaluate(el => el.inert), true);
    await page.getByRole('button', { name: 'Discard', exact: true }).click();
    await dialog.waitFor({ state: 'hidden' });
    await page.getByRole('link', { name: 'Settings', exact: true }).click();
    assert.equal(await page.getByRole('textbox', { name: 'Save folder' }).inputValue(), 'C:/test-downloads');
    await page.getByRole('combobox', { name: 'Default format' }).click();
    await page.getByRole('option', { name: 'Audio · MP3', exact: true }).click();
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('ydlite.downloadPreferences')).defaultQuality), 'mp3');
    await page.screenshot({ path: artifactDir + '/settings.png' });

    // Closing while a download runs asks first.
    await page.getByRole('link', { name: 'Download', exact: true }).click();
    await page.getByRole('textbox', { name: 'Video link' }).fill('https://example.com/closing');
    await page.getByRole('button', { name: 'Go', exact: true }).click();
    await page.getByRole('heading', { name: /Sample video/ }).waitFor();
    await page.getByRole('button', { name: 'Download', exact: true }).click();
    await close.click();
    await dialog.waitFor();
    assert.equal(await page.evaluate(() => window.destroyCount), 0);
    await page.keyboard.press('Escape');
    await dialog.waitFor({ state: 'hidden' });

    for (const width of [1180, 900, 800, 720]) {
      await page.setViewportSize({ width, height: 560 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, 'horizontal overflow at ' + width);
      assert.equal(await page.evaluate(() => document.documentElement.scrollHeight <= innerHeight), true, 'body overflow at ' + width);
    }
    await page.getByRole('link', { name: 'Settings', exact: true }).click();
    await page.locator('.settings-actions').waitFor();
    await page.screenshot({ path: artifactDir + '/settings-small.png' });

    assert.deepEqual(errors, []);
    console.log('PASS: window chrome, focus visibility, analyze, download, pause/resume, cancel, history, settings guard, close confirmation, responsive layouts; no runtime errors.');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
