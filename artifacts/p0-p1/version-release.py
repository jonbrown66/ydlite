from pathlib import Path
import json
records=json.loads(Path('artifacts/p0-p1/before/hashes.json').read_text(encoding='utf-8-sig'))
import hashlib
for record in records:
 path=Path(record['Path'])
 if hashlib.sha256(path.read_bytes()).hexdigest().upper()!=record['Hash']:
  raise RuntimeError('Configuration changed after backup: '+str(path))
for file in ['package.json','src-tauri/Cargo.toml','src-tauri/tauri.conf.json','src/LandingPage.vue','README.md','index.html']:
 p=Path(file);s=p.read_text(encoding='utf-8');s=s.replace('0.1.0','0.2.0');p.write_text(s,encoding='utf-8')
p=Path('package-lock.json');data=json.loads(p.read_text(encoding='utf-8'));data['version']='0.2.0';data['packages']['']['version']='0.2.0';p.write_text(json.dumps(data,indent=2)+'\n',encoding='utf-8')
