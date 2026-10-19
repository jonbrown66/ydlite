const task=await taskSpace(3),page=task.page('p1');
await page.fill('loc=css:input[aria-label="Video or audio URL"]','https://example.com/qa');
await page.click('loc=role:button[name="Analyze"]');
await page.waitForSelector('loc=css:.video-details');
console.log(await page.snapshot());
console.log({screenshot:await page.screenshot({path:'E:/demo2/ydlite/artifacts/p0-p1/single.png'})});
