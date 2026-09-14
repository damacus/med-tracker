import { readFileSync } from 'node:fs';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: Object.fromEntries(
  ['input', 'kind', 'timings', 'shard', 'shards'].map(name => [name, { type: 'string' }])
) });
const count = Number(values.shards);
const index = Number(values.shard);
if (!Number.isInteger(count) || count < 1 || !Number.isInteger(index) || index < 1 || index > count) {
  throw new Error('Shard must be an integer within the requested range');
}
if (!values.input || !['files', 'examples'].includes(values.kind)) {
  throw new Error('Input and kind (files or examples) are required');
}

let timings = {};
if (values.timings) {
  try {
    timings = JSON.parse(readFileSync(values.timings, 'utf8'));
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
}
if (!timings || typeof timings !== 'object' || Array.isArray(timings)) {
  throw new Error('Timings must be an object');
}
const validTiming = value => typeof value === 'number' && Number.isFinite(value) && value > 0;
const measured = Object.values(timings).filter(validTiming);
const fallback = measured.length ? measured.reduce((sum, value) => sum + value, 0) / measured.length : 1;
const report = JSON.parse(readFileSync(values.input, 'utf8'));
if (!Array.isArray(report.examples) || report.examples.length === 0) throw new Error('No examples in manifest');

const items = new Map();
const seen = new Set();
for (const example of report.examples) {
  if (typeof example.id !== 'string' || !example.id || seen.has(example.id)) {
    throw new Error('Duplicate or missing example id');
  }
  seen.add(example.id);
  if (typeof example.file_path !== 'string' || !example.file_path.startsWith('./spec/')) {
    throw new Error('Example must have a spec file path');
  }
  const key = values.kind === 'files' ? example.file_path : example.id;
  const duration = timings[key] ?? timings[example.file_path];
  items.set(key, { key, duration: validTiming(duration) ? duration : fallback, estimated: !validTiming(duration) });
}

const shards = Array.from({ length: count }, () => ({ items: [], duration: 0 }));
const ordered = [...items.values()].sort((a, b) => b.duration - a.duration || a.key.localeCompare(b.key));
for (const item of ordered) {
  const shard = shards.reduce((best, candidate) => candidate.duration < best.duration ? candidate : best);
  shard.items.push(item.key);
  shard.duration += item.duration;
}
const selected = shards[index - 1];
if (!selected.items.length) throw new Error('No examples selected for shard');
console.error(JSON.stringify({ shard: index, shards: count, items: selected.items.length,
  duration: selected.duration, estimated: ordered.filter(item => item.estimated).length }));
console.log(selected.items.join('\n'));
