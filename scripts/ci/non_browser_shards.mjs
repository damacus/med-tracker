import { readFileSync } from 'node:fs';

const options = parseOptions(process.argv.slice(2));
const report = JSON.parse(readFileSync(options.input, 'utf8'));
const examples = report.examples || [];
const files = new Set();

for (const example of examples) {
  if (!example.file_path) throw new Error(`Missing file path for non-browser example: ${example.id || 'missing'}`);
  files.add(example.file_path);
}

const measuredFiles = [...files].map(filePath => {
  const duration = options.timings[filePath];
  if (!Number.isFinite(Number(duration)) || Number(duration) <= 0) {
    throw new Error(`Missing measured timing for non-browser file: ${filePath}`);
  }
  return { filePath, duration: Number(duration) };
});
const shards = Array.from({ length: options.shards }, () => ({ files: [], duration: 0 }));

measuredFiles
  .sort((left, right) => right.duration - left.duration || left.filePath.localeCompare(right.filePath))
  .forEach(file => {
    const shard = shards.reduce((best, candidate) => candidate.duration < best.duration ? candidate : best);
    shard.files.push(file.filePath);
    shard.duration += file.duration;
  });

if (options.shard < 1 || options.shard > options.shards) throw new Error('Shard must be within the requested range');
const selected = shards[options.shard - 1];
console.error(JSON.stringify({
  shard: options.shard,
  shards: options.shards,
  files: selected.files.length,
  duration: selected.duration
}));
console.log(selected.files.join('\n'));

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
    !options.input || options.shard === undefined || options.shards === undefined || options.shards < 1 ||
    !Number.isInteger(options.shard) || !Number.isInteger(options.shards) || !options.timings ||
    typeof options.timings !== 'object' || Array.isArray(options.timings)
  ) {
    throw new Error('Input, shard, shards, and measured timings are required');
  }
  return options;
}
