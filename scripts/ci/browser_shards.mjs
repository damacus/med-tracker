import { readFileSync } from 'node:fs';

const options = parseOptions(process.argv.slice(2));
const report = JSON.parse(readFileSync(options.input, 'utf8'));
const examples = report.examples || [];
const seen = new Set();
const shards = Array.from({ length: options.shards }, () => ({ examples: [], duration: 0 }));

for (const example of examples) {
  const id = example.id;
  if (!id || seen.has(id)) throw new Error(`Duplicate or missing browser example id: ${id || 'missing'}`);
  seen.add(id);
}

const measuredDurations = examples.map(example => {
  const duration = options.timings[example.id] ?? options.timings[example.file_path];
  if (!Number.isFinite(Number(duration)) || Number(duration) <= 0) {
    throw new Error(`Missing measured timing for browser example: ${example.id || 'missing'}`);
  }
  return Number(duration);
});

examples
  .map((example, index) => ({ example, index, duration: measuredDurations[index] }))
  .sort((left, right) => right.duration - left.duration || left.index - right.index)
  .forEach(item => {
    const shard = shards.reduce((best, candidate) => candidate.duration < best.duration ? candidate : best);
    shard.examples.push(item.example.id);
    shard.duration += item.duration;
  });

if (options.shard < 1 || options.shard > options.shards) throw new Error('Shard must be within the requested range');
const selected = shards[options.shard - 1];
console.error(JSON.stringify({ shard: options.shard, shards: options.shards, examples: selected.examples.length, duration: selected.duration }));
console.log(selected.examples.join('\n'));

function parseOptions(args) {
  const options = {};
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    const value = args[index + 1];
    if (key === '--input') options.input = value;
    if (key === '--shard') options.shard = Number(value);
    if (key === '--shards') options.shards = Number(value);
    if (key === '--timings') options.timings = JSON.parse(readFileSync(value, 'utf8'));
  }
  if (
    !options.input || !options.shard || !options.shards || !options.timings ||
    typeof options.timings !== 'object' || Array.isArray(options.timings)
  ) {
    throw new Error('Input, shard, shards, and measured timings are required');
  }
  return options;
}
