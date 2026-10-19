import {spawn} from 'node:child_process';
import fs from 'node:fs';
const vite=spawn(process.execPath,['node_modules/vite/bin/vite.js','--mode','tauri','--host','127.0.0.1','--port','1432'],{cwd:'E:/demo2/ydlite',windowsHide:true,stdio:['ignore','pipe','pipe']});
vite.stdout.on('data',d=>fs.appendFileSync('E:/demo2/ydlite/artifacts/p0-p1/vite.log',d));
vite.stderr.on('data',d=>fs.appendFileSync('E:/demo2/ydlite/artifacts/p0-p1/vite-error.log',d));
await import('./qa-proxy.mjs');
console.log('QA supervisor active; vite PID '+vite.pid);
process.on('SIGINT',()=>{vite.kill();process.exit()});
