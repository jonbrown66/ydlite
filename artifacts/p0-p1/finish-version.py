from pathlib import Path
p=Path('index.html');raw=p.read_bytes();print(raw[260:355]);
s=raw.decode('gb18030');print([line.encode('unicode_escape').decode() for line in s.splitlines() if '<title>' in line]);
p.write_text(s.replace('0.1.0','0.2.0'),encoding='utf-8')
p=Path('package-lock.json');import json;data=json.loads(p.read_text(encoding='utf-8'));assert data['version']=='0.1.0';data['version']='0.2.0';data['packages']['']['version']='0.2.0';p.write_text(json.dumps(data,indent=2)+'\n',encoding='utf-8')
