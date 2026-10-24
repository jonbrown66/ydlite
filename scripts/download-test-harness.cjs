const fs = require('node:fs');
const vm = require('node:vm');
const ts = require('typescript');
const vue = require('vue');
const pinia = require('pinia');
const { randomUUID } = require('node:crypto');

exports.tick = () => new Promise(resolve => setImmediate(resolve));

// Loads the real Pinia download store and the real component script with a
// controllable IPC layer, so pause/resume timing can be exercised without a GUI.
exports.setupDownload = function setupDownload(settings = {}) {
  const data = new Map();
  const calls = [], pauses = [], cancels = [], listeners = [];
  let controlStart = true;
  const storage = {
    getItem: key => data.get(key) ?? null,
    setItem(key, value) { data.set(key, value); },
  };
  const api = {
    startDownload(request) {
      if (!controlStart) return Promise.resolve({ cancelled: false, paused: false, filePath: request.dir + '/' + request.url.split('/').at(-1) + '.mp4', skipped: false });
      return new Promise((resolve, reject) => calls.push({ request, resolve, reject }));
    },
    cancelDownload() { cancels.push({}); return Promise.resolve(); },
    pauseDownload() { pauses.push({}); return Promise.resolve(); },
    onDownloadProgress: async handler => { listeners.push(handler); return () => listeners.splice(listeners.indexOf(handler), 1); },
  };
  const cache = new Map();
  const context = vm.createContext({ exports: {}, console, URL, crypto: { randomUUID }, localStorage: storage, window: { setTimeout, clearTimeout }, require: load });
  pinia.setActivePinia(pinia.createPinia());
  function load(name) {
    if (name === 'vue') return { ...vue, onMounted() {}, onBeforeUnmount() {}, onActivated() {} };
    if (name === 'pinia') return pinia;
    if (name.includes('api/tauri')) return api;
    if (!name.startsWith('@/') && !name.startsWith('./')) return {};
    const file = name.startsWith('@/') ? `src/${name.slice(2)}.ts` : `src/stores/${name.slice(2)}.ts`;
    if (cache.has(file)) return cache.get(file);
    const module = { exports: {} };
    cache.set(file, module.exports);
    const code = ts.transpileModule(fs.readFileSync(file, 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
    vm.runInContext(`(function(exports,require){${code}\n})`, context)(module.exports, load);
    return module.exports;
  }
  const download = load('@/stores/download').useDownloadStore();
  const history = load('@/stores/tasks').useTasksStore();
  return {
    download, history, calls, pauses, cancels, data, context, load,
    emit: event => listeners.forEach(handler => handler(event)),
    controlStart: value => { controlStart = value; },
    request: (url, extra = {}) => ({ dir: settings.dir || 'C:/downloads', mode: 'best', formatId: null, cookieSource: { type: 'none' }, fileOptions: undefined, ...extra, url }),
  };
};
