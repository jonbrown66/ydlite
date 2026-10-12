const fs = require('node:fs');
const vm = require('node:vm');
const ts = require('typescript');
const vue = require('vue');
const pinia = require('pinia');
const { randomUUID } = require('node:crypto');

exports.tick = () => new Promise(resolve => setImmediate(resolve));
exports.setupQueue = function setupQueue(initial = {}, settings = {}) {
  const data = new Map(Object.entries(initial));
  const calls = [], parses = [], cancels = [], finishes = [], listeners = [];
  let storageFails = false, controlParsing = false, controlCancel = false;
  const storage = { getItem: key => data.get(key) ?? null, setItem(key, value) { if (storageFails) throw new Error('Quota exceeded'); data.set(key, value); } };
  const api = {
    startDownload(request) { return new Promise((resolve, reject) => calls.push({ request, resolve, reject })); },
    parseVideo(request) {
      if (controlParsing) return new Promise((resolve, reject) => parses.push({request, resolve, reject}));
      parses.push({request}); return Promise.resolve({title:`Video ${request.url.split('/').at(-1)}`,resolvedUrl:request.url,cookieSource:request.options.cookieSource});
    },
    cancelDownload() { if (controlCancel) return new Promise((resolve,reject)=>cancels.push({resolve,reject})); cancels.push({}); return Promise.resolve(); },
    onDownloadProgress: async handler => { listeners.push(handler); return () => listeners.splice(listeners.indexOf(handler),1); },
    onToolInstallProgress: async () => () => {},
  };
  const activity = new Proxy({}, { get: (_, name) => name === 'finishDownload' ? value => finishes.push(value) : () => {} });
  const cache = new Map();
  const context = vm.createContext({exports:{},console, URL, crypto:{randomUUID}, localStorage:storage, window:{setTimeout,clearTimeout}, require:load});
  pinia.setActivePinia(pinia.createPinia());
  function load(name) {
    if (name === 'vue') return {...vue,onMounted(){},onBeforeUnmount(){},onActivated(){}};
    if (name === 'pinia') return pinia;
    if (name.includes('api/tauri')) return api;
    if (name.endsWith('/activity') && !settings.realActivity) return {useActivityStore:()=>activity};
    if (!name.startsWith('@/') && !name.startsWith('./')) return {};
    const file = name.startsWith('@/') ? `src/${name.slice(2)}.ts` : `src/stores/${name.slice(2)}.ts`;
    if (cache.has(file)) return cache.get(file);
    const module = {exports:{}}; cache.set(file,module.exports);
    const code = ts.transpileModule(fs.readFileSync(file,'utf8'), {compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
    vm.runInContext(`(function(exports,require){${code}\n})`,context)(module.exports,load);
    return module.exports;
  }
  const queue = load('@/stores/downloadQueue').useDownloadQueueStore();
  const history = load('@/stores/tasks').useTasksStore();
  return {queue, history, calls, parses, cancels, finishes, data, context, api, load,
    emit:event=>listeners.forEach(handler=>handler(event)),
    failStorage:value=>storageFails=value, controlParsing:value=>controlParsing=value, controlCancel:value=>controlCancel=value,
    page(extra) {
      const source = fs.readFileSync('src/pages/DownloadPage.vue','utf8').split('<script setup lang="ts">')[1].split('</script>')[0];
      const code = ts.transpileModule(source+extra, {compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
      vm.runInContext(code,context); return context.test;
    },
  };
};
