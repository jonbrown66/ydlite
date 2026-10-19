from pathlib import Path
p=Path('index.html');s=p.read_bytes().decode('utf-8').replace('\r\r\n','\n').replace('\r\n','\n');p.write_text(s,encoding='utf-8')
p=Path('src/pages/SettingsPage.vue');s=p.read_text(encoding='utf-8').replace('draft.value = { ...preferences.settings }','draft.value = cloneSettings()');p.write_text(s,encoding='utf-8')
