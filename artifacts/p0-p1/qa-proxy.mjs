import http from 'node:http';
import fs from 'node:fs';
const mock = `
import { mockIPC, mockWindows } from '/node_modules/@tauri-apps/api/mocks.js';
import { emit } from '/node_modules/@tauri-apps/api/event.js';
mockWindows('main'); window.qaRequests=[]; window.qaParses=[]; window.qaErrors=[];
window.addEventListener('error',e=>window.qaErrors.push(e.message));
localStorage.setItem('ydlite.defaultDir','C:/qa-downloads');
let activeRequest,finish;
window.qaFinish = async (status='finished',skipped=false) => { const request=activeRequest; const done=finish; const filePath='C:/qa-downloads/中文标题.mp4'; finish=null; await emit('download://progress',{requestId:request.requestId,status,filePath}); done({cancelled:status==='cancelled',filePath,skipped}); };
mockIPC((cmd,args) => {
 if(cmd==='check_dependencies')return {ytdlp_ok:true,ffmpeg_ok:true,ytdlp_update_available:false,ytdlp_path:'yt-dlp',ffmpeg_path:'ffmpeg'};
 if(cmd==='check_ytdlp_update')return {ytdlp_update_available:false};
 if(cmd==='plugin:window|is_maximized')return false;
 if(cmd==='plugin:dialog|open')return 'C:/qa-other-folder';
 if(cmd==='start_download'){activeRequest=args.request;window.qaRequests.push(args.request);return new Promise(resolve=>finish=resolve);}
 if(cmd==='cancel_download')return window.qaFinish('cancelled');
 if(cmd==='parse_video'){window.qaParses.push(args.request);return {title:'中文标题 · Format and file options',uploader:'Example creator',duration:120,originalUrl:args.request.url,resolvedUrl:args.request.url,site:'example',cookieSource:{type:'none'},formats:[{formatId:'avc',ext:'mp4',height:1080,fps:30,vcodec:'avc1',acodec:'none',filesize:10485760},{formatId:'vp9',ext:'webm',height:1080,fps:60,vcodec:'vp9',acodec:'none',filesize:12582912},{formatId:'en',ext:'m4a',vcodec:'none',acodec:'aac',language:'en',bitrateKbps:128},{formatId:'zh',ext:'m4a',vcodec:'none',acodec:'aac',language:'zh',bitrateKbps:128}]};}
 return null;
},{shouldMockEvents:true});
await import('/src/main-app.ts?qa-real');
`;
const server=http.createServer((req,res)=>{
 if(req.url.startsWith('/src/main-app.ts')&&!req.url.includes('qa-real')){res.writeHead(200,{'content-type':'application/javascript'});res.end(mock);return;}
 if(['/fixture.mp4','/video.mp4','/en.m4a','/zh.m4a'].includes(req.url)){res.writeHead(200,{'content-type':req.url.endsWith('.m4a')?'audio/mp4':'video/mp4'});fs.createReadStream('E:/demo2/ydlite/artifacts/p0-p1'+req.url).pipe(res);return;}
 const proxy=http.request({hostname:'127.0.0.1',port:1432,path:req.url,method:req.method,headers:{...req.headers,host:'127.0.0.1:1432'}}, upstream=>{res.writeHead(upstream.statusCode,upstream.headers);upstream.pipe(res);});
 proxy.on('error',()=>{res.writeHead(502);res.end('QA upstream unavailable');});req.pipe(proxy);
});server.listen(1433,'127.0.0.1',()=>console.log('QA proxy listening on 1433'));
