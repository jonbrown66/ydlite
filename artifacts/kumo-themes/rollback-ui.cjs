const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {execFileSync} = require('node:child_process');
const root = process.cwd();
const hash = value => crypto.createHash('sha256').update(value).digest('hex').toUpperCase();
const normalize = value => value.toString('utf8').replace(/\r\n/g, '\n');
const originals = JSON.parse(fs.readFileSync('artifacts/kumo-themes/before/hashes.json', 'utf8'));
const files = ['src/pages/DownloadPage.vue', 'src/components/DownloadQueue.vue', 'src/components/DownloadOptionsFields.vue', 'src/components/ui/ActionMenu.vue', 'scripts/queue-browser-regression.cjs'];
const prepared = files.map(file => {
  const original = fs.readFileSync(path.join('artifacts/kumo-themes/before', file));
  const recorded = originals.find(record => record.path === file);
  if (recorded && hash(original) !== recorded.sha256) throw Error(`Original backup changed: ${file}`);
  if (normalize(original) !== normalize(execFileSync('git', ['show', `HEAD:${file}`]))) throw Error(`Backup differs from initial tracked baseline: ${file}`);
  return {file, original, current: fs.readFileSync(file)};
});
const storeFile = 'src/stores/downloadQueue.ts';
const storeCurrent = fs.readFileSync(storeFile);
let storeRestored = normalize(storeCurrent);
for (const addition of [
  "import { appendSpeedSample, type SpeedSample } from '@/lib/downloadSpeed'\n",
  "  const speedSamples = shallowRef<SpeedSample[]>([])\n",
  "  const speedSampleUrl = ref('')\n",
  "    if (event.status === 'downloading') speedSamples.value = appendSpeedSample(speedSamples.value, event.speed, Date.now())\n",
  "        speedSamples.value = []\n",
  "        speedSampleUrl.value = job.url\n",
  "            speedSampleUrl.value = job.url\n",
]) {
  if (!storeRestored.includes(addition)) throw Error(`Cannot identify speed sampler addition: ${addition}`);
  storeRestored = storeRestored.replace(addition, '');
}
storeRestored = storeRestored.replace('progress, speedSamples, speedSampleUrl, logs,', 'progress, logs,');
const storeOriginal = execFileSync('git', ['show', `HEAD:${storeFile}`]);
if (storeRestored !== normalize(storeOriginal)) throw Error('Store contains other changes; preserve them before rollback.');
prepared.push({file: storeFile, original: storeOriginal, current: storeCurrent});
for (const entry of prepared) {
  const backup = path.join(root, 'artifacts/kumo-themes/rollback-before', entry.file);
  fs.mkdirSync(path.dirname(backup), {recursive:true});
  fs.writeFileSync(backup, entry.current);
  if (hash(fs.readFileSync(backup)) !== hash(entry.current)) throw Error(`Rollback snapshot mismatch: ${entry.file}`);
}
for (const entry of prepared) {
  if (hash(fs.readFileSync(entry.file)) !== hash(entry.current)) throw Error(`Concurrent change detected: ${entry.file}`);
  fs.writeFileSync(entry.file, entry.original);
  if (hash(fs.readFileSync(entry.file)) !== hash(entry.original)) throw Error(`Restore mismatch: ${entry.file}`);
}
console.log(`Restored ${prepared.length} files to the verified original content; rollback snapshots retained.`);
