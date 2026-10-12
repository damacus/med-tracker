import assert from 'node:assert/strict';
import test from 'node:test';
import { partition, parseShard, browserSelection } from '../loco-shards.mjs';

test('shards cover every test exactly once and balance measured durations', () => {
  const items = ['slow', 'medium', 'a', 'b', 'c', 'd'];
  const timings = { slow: 10, medium: 8, a: 2, b: 2, c: 2, d: 2 };
  const shards = partition(items, 2, timings);
  assert.deepEqual(shards.flat().sort(), [...items].sort());
  assert.equal(new Set(shards.flat()).size, items.length);
  assert.deepEqual(shards.map(shard => shard.reduce((sum, key) => sum + timings[key], 0)), [14, 12]);
  assert.deepEqual(partition([...items].reverse(), 2, timings), shards);
});

test('invalid shard selections and duplicate tests fail closed', () => {
  for (const value of ['0/2', '3/2', '1/0', 'one', '1/2;echo']) assert.throws(() => parseShard(value));
  assert.deepEqual(parseShard('2/4'), { index: 1, total: 4 });
  assert.throws(() => partition(['a', 'a'], 2));
});

test('browser sharding keeps each viewport and complete serial file exactly once', () => {
  const files = ['a.spec.mjs', 'b.spec.mjs', 'c.spec.mjs'];
  const shards = Array.from({ length: 3 }, (_, index) => browserSelection(files, `${index + 1}/3`, {}));
  const identities = shards.flatMap(shard => Object.entries(shard).flatMap(([project, matches]) => matches.map(file => `${project}/${file}`)));
  assert.equal(new Set(identities).size, 6);
  assert.equal(identities.length, 6);
  for (const file of files) for (const project of ['desktop', 'mobile']) assert.ok(identities.includes(`${project}/${file}`));
});
