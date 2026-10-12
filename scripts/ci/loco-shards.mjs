export function parseShard(value = '1/1') {
  if (!/^[1-9]\d*\/[1-9]\d*$/.test(value)) throw new Error('SHARD must be index/total');
  const [index, total] = value.split('/').map(Number);
  if (index > total || total > 64) throw new Error('Invalid shard range');
  return { index: index - 1, total };
}

export function partition(items, total, timings = {}) {
  if (new Set(items).size !== items.length) throw new Error('Duplicate test identity');
  const shards = Array.from({ length: total }, () => []);
  const weights = Array(total).fill(0);
  const weight = key => Number.isFinite(timings[key]) && timings[key] > 0 ? timings[key] : 1;
  for (const key of [...items].sort((a, b) => weight(b) - weight(a) || a.localeCompare(b))) {
    const index = weights.indexOf(Math.min(...weights));
    shards[index].push(key);
    weights[index] += weight(key);
  }
  return shards;
}

export function browserSelection(files, shard, timings) {
  const { index, total } = parseShard(shard);
  const selected = partition(['desktop', 'mobile'].flatMap(project => files.map(file => `${project}/${file}`)), total, timings)[index];
  return Object.fromEntries(['desktop', 'mobile'].map(project => [project, selected.filter(key => key.startsWith(`${project}/`)).map(key => key.slice(project.length + 1))]));
}
