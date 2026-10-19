const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const ts = require('typescript');
const vue = require('vue');
function load(file, initial = {}) {
  const data = new Map(Object.entries(initial));
  const storage = {getItem:key=>data.get(key) ?? null,setItem:(key,value)=>data.set(key,value)};
  const context = {exports:{},localStorage:storage,require(name) {
    if (name.startsWith('@/lib/')) return load('src/lib/' + name.slice(6) + '.ts').module;
    if (name === 'vue') return vue;
    if (name === 'pinia') return {defineStore:(_,setup)=>setup};
    return {};
  }};
  const code = ts.transpileModule(fs.readFileSync(file,'utf8'), {compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
  vm.runInNewContext(code,context);
  return {module:context.exports,data,storage};
}
const record = n => ({id:String(n),title:`Video ${n}`,url:`https://example.com/${n}`,filePath:`C:/test/${n}.mp4`,completedAt:'2026-10-11T00:00:00Z'});
{
  const {module,data} = load('src/stores/tasks.ts', {'ydlite.history':JSON.stringify([...Array.from({length:110},(_,n)=>record(n)),null,{id:'invalid',title:'Bad',filePath:'C:/bad'}])});
  const store = module.useTasksStore();
  assert.equal(store.downloads.value.length,100);
  store.addDownload(record(110));
  assert.equal(store.downloads.value.length,100);
  store.addDownload({...record(110),title:'Updated'});
  assert.equal(store.downloads.value.filter(item=>item.filePath==='C:/test/110.mp4').length,1);
  store.removeDownload('110');
  store.addDownload(record(111));
  assert.equal(store.downloads.value[0].id,'111');
  assert.equal(JSON.parse(data.get('ydlite.history'))[0].id,'111');
  store.clearDownloads(); assert.equal(store.total.value,0);
}
for (const corrupt of ['bad json','null','{}','[null,{"id":"bad"}]']) {
  assert.equal(load('src/stores/tasks.ts',{'ydlite.history':corrupt}).module.readDownloadHistory().length,0);
}
{
  const {module,data} = load('src/stores/preferences.ts', {'ydlite.defaultDir':'C:/existing','ydlite.cookiesFilePath':'C:/cookies.txt','legacy.preferences.backup':'preserve'});
  const store = module.usePreferencesStore();
  assert.equal(store.settings.value.defaultDir,'C:/existing');
  assert.equal(store.settings.value.cookiesFilePath,'C:/cookies.txt');
  store.update({defaultQuality:'mp3',defaultDir:' C:/new '});
  assert.equal(store.settings.value.defaultDir,'C:/new');
  assert.equal(JSON.parse(data.get('ydlite.downloadPreferences')).defaultQuality,'mp3');
  assert.equal(data.get('legacy.preferences.backup'),'preserve');
  assert.equal(load('src/stores/preferences.ts',Object.fromEntries(data)).module.usePreferencesStore().settings.value.defaultQuality,'mp3');
}
console.log('PASS: history capacity, validation, deduplication, deletion and persistence; legacy download preferences and retained legacy data.');
