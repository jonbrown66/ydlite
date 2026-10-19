import json, subprocess
cases={'youtube':'https://www.youtube.com/watch?v=jNQXAC9IVRw','bilibili':'https://www.bilibili.com/video/BV1JE411F741','public-media':'https://media.w3.org/2010/05/sintel/trailer.mp4'}
results=[]
for site,url in cases.items():
 try:
  run=subprocess.run(['yt-dlp','--ignore-config','--no-playlist','--socket-timeout','12','--retries','0','-J',url],capture_output=True,timeout=45,creationflags=subprocess.CREATE_NO_WINDOW)
  if run.returncode==0:
   info=json.loads(run.stdout);results.append({'site':site,'url':url,'parsed':True,'formats':len(info.get('formats',[])),'title':info.get('title')})
  else:
   error=run.stderr.decode('utf-8',errors='replace').lower()
   cause='sign-in/bot challenge' if any(s in error for s in ['sign in','not a bot','cookies']) else 'blocked/access' if any(s in error for s in ['403','412']) else 'network' if any(s in error for s in ['connection','timed out','ssl']) else 'unavailable/other'
   results.append({'site':site,'url':url,'parsed':False,'cause':cause})
 except subprocess.TimeoutExpired:results.append({'site':site,'url':url,'parsed':False,'cause':'timeout'})
 print(json.dumps(results[-1],ensure_ascii=True),flush=True)
open('E:/demo2/ydlite/artifacts/p0-p1/public-sites.json','w',encoding='utf-8').write(json.dumps(results,ensure_ascii=False,indent=2))
