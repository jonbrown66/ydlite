import yt_dlp.extractor.bilibili as module
for cls in [module.BiliBiliIE]:
 for test in cls._TESTS[:12]:
  print(test.get('url'),test.get('info_dict',{}).get('duration'),test.get('info_dict',{}).get('title'))
