const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const ts = require('typescript');
const vue = require('vue');

function load(saved, systemDark = false, unavailable = false) {
  const data = new Map(saved === undefined ? [] : [['ydlite.appearance', saved]]);
  const storage = {
    getItem: key => { if (unavailable) throw Error('Storage unavailable'); return data.get(key) ?? null; },
    setItem: (key, value) => { if (unavailable) throw Error('Storage unavailable'); data.set(key, value); },
  };
  const listeners = new Map();
  const media = { matches: systemDark, addEventListener: (_, fn) => listeners.set('media', fn), removeEventListener: () => listeners.delete('media') };
  const root = { dataset: {}, style: {} };
  let themeColor;
  const context = {
    exports: {}, localStorage: storage,
    document: { documentElement: root, querySelector: () => ({ setAttribute: (_, value) => { themeColor = value; } }) },
    window: { matchMedia: () => media, addEventListener: (name, fn) => listeners.set(name, fn), removeEventListener: name => listeners.delete(name) },
    require: name => name === 'vue' ? vue : { defineStore: (_, setup) => setup },
  };
  const code = ts.transpileModule(fs.readFileSync('src/stores/appearance.ts', 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  vm.runInNewContext(code, context);
  const store = context.exports.useAppearanceStore();
  store.initialize();
  return { store, root, media, data, storage, listeners, getThemeColor: () => themeColor };
}

for (const saved of [undefined, 'system', 'invalid', '{}']) {
  const env = load(saved, true);
  assert.equal(env.root.dataset.theme, 'dark');
  assert.equal(env.store.preference.value, 'system');
  env.media.matches = false; env.listeners.get('media')();
  assert.equal(env.root.dataset.theme, 'light');
  env.store.choose('dark');
  assert.equal(env.data.get('ydlite.appearance'), 'dark');
  env.media.matches = false; env.listeners.get('media')();
  assert.equal(env.root.dataset.theme, 'dark', 'explicit choice ignores system changes');
  assert.equal(load(env.data.get('ydlite.appearance'), false).root.dataset.theme, 'dark', 'saved choice survives restart');
  env.listeners.get('storage')({ storageArea: env.storage, key: 'unrelated', newValue: 'light' });
  assert.equal(env.root.dataset.theme, 'dark');
  env.listeners.get('storage')({ storageArea: env.storage, key: 'ydlite.appearance', newValue: 'light' });
  assert.equal(env.root.dataset.theme, 'light', 'other windows stay in sync');
  assert.equal(env.root.style.colorScheme, 'light');
  env.listeners.get('storage')({ storageArea: env.storage, key: null, newValue: null });
  assert.equal(env.store.preference.value, 'system');
  env.store.stop(); assert.equal(env.listeners.size, 0, 'listeners are disposed');
}
assert.equal(load('light', true).root.dataset.theme, 'light');
assert.equal(load('dark', false).getThemeColor(), '#17191d');
const blocked = load(undefined, true, true);
blocked.store.choose('light');
assert.equal(blocked.root.dataset.theme, 'dark', 'failed persistence retains current appearance');
assert.ok(blocked.store.error.value.includes('Unable to save'));
console.log('PASS: system changes, explicit overrides, restart persistence, corrupt preferences, cross-window sync, storage failure and listener cleanup.');
