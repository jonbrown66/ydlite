const task = await taskSpace(3);
const page = task.page('p1');
await page.goto('http://127.0.0.1:1433');
console.log(await page.snapshot());
