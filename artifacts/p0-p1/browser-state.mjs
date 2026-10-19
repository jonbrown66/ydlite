const task=await taskSpace(3),page=task.page('p1');
console.log(await page.evaluate(()=>({body:document.body.innerText,errors:window.qaErrors,ready:!!window.__TAURI_INTERNALS__,scripts:[...document.scripts].map(s=>s.src)})));
console.log(await page.snapshot());
