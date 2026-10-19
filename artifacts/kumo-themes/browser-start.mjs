const task = await taskSpace('YDLite Kumo theme verification');
console.log({ spaceId: task.spaceId });
const page = task.page('p1');
await page.goto('http://127.0.0.1:1433/artifacts/kumo-themes/preview.html');
await page.waitForSelector('.app-frame', { state: 'visible' });
console.log(await page.snapshot());
console.log(await page.screenshot({ path: 'E:/demo2/ydlite/artifacts/kumo-themes/light-empty.png' }));
