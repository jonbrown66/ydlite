import inspect,yt_dlp.extractor.bilibili as m
s=inspect.getsource(m.BiliBiliIE._real_extract)
for line in s.splitlines():
 if any(t in line for t in ["'id':",'video_id =','cid =','webpage_url']): print(line)
