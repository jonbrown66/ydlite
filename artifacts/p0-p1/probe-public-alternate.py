import json,subprocess,re
urls={'blender-public-media':'https://download.blender.org/peach/trailer/trailer_400p.ogg','bilibili-alternate':'https://www.bilibili.com/video/BV1xx411c7mD'}
out=[]
for site,url in urls.items():
 try:
  run=subprocess.run(['yt-dlp','--ignore-config','--no-playlist','--no-warnings','--socket-timeout','12','--retries','0','-J',url],capture_output=True,timeout=40,creationflags=subprocess.CREATE_NO_WINDOW)
  error=run.stderr.decode('utf-8',errors='replace')
  details=[line for line in error.splitlines() if line.startswith('ERROR:')]
  summary=re.sub(r'https?://\S+','<url>', ' '.join(details))[:220]
  info=json.loads(run.stdout) if run.returncode==0 else {}
  result={'site':site,'url':url,'parsed':run.returncode==0,'title':info.get('title'),'formats':len(info.get('formats',[])),'errorSummary':summary}
 except subprocess.TimeoutExpired:result={'site':site,'url':url,'parsed':False,'errorSummary':'timeout'}
 out.append(result);print(json.dumps(result,ensure_ascii=True),flush=True)
open('E:/demo2/ydlite/artifacts/p0-p1/public-sites-alternate.json','w',encoding='utf-8').write(json.dumps(out,ensure_ascii=False,indent=2))
