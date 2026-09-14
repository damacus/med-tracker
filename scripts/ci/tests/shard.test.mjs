import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const script = fileURLToPath(new URL('../shard.mjs', import.meta.url));
const examples = ['a', 'b', 'c', 'd'].map(name => ({
  id: `./spec/${name}_spec.rb[1:1]`, file_path: `./spec/${name}_spec.rb`
}));
const timings = Object.fromEntries(examples.map((example, index) => [example.file_path, 8 - index]));

function runShard(t, shard, durations = timings, selected = examples, kind = 'examples') {
  const directory = mkdtempSync(join(tmpdir(), 'ci-shards-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const input = join(directory, 'examples.json');
  const measured = join(directory, 'timings.json');
  writeFileSync(input, JSON.stringify({ examples: selected }));
  if (durations !== 'missing-file') writeFileSync(measured, JSON.stringify(durations));
  return spawnSync(process.execPath, [script, '--kind', kind, '--input', input,
    ...(durations === null ? [] : ['--timings', measured]),
    '--shards', '2', `--shard=${shard}`], { encoding: 'utf8' });
}

test('browser shards assign every example once and balance measured durations', t => {
  const first = runShard(t, 1);
  const second = runShard(t, 2);
  for (const result of [first, second]) {
    assert.equal(result.status, 0, result.stderr);
    assert.equal(JSON.parse(result.stderr).duration, 13);
  }
  assert.deepEqual(first.stdout.trim().split('\n'), [examples[0].id, examples[3].id]);
  assert.deepEqual(second.stdout.trim().split('\n'), [examples[1].id, examples[2].id]);
});

for (const duration of [undefined, 0, -1, 'invalid']) {
  test(`shards estimate ${duration ?? 'missing'} timing data without excluding examples`, t => {
    const result = runShard(t, 1, { ...timings, [examples[0].file_path]: duration });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(JSON.parse(result.stderr).estimated, 1);
  });
}

test('browser shards reject duplicate examples', t => {
  const result = runShard(t, 1, timings, [examples[0], examples[0]]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Duplicate or missing example id/);
});

test('file shards include a new untimed spec and keep each file intact', t => {
  const selected = [...examples, { ...examples[0], id: './spec/a_spec.rb[1:2]' },
    { id: './spec/new_spec.rb[1:1]', file_path: './spec/new_spec.rb' }];
  const results = [1, 2].map(shard => runShard(t, shard, timings, selected, 'files'));
  for (const result of results) assert.equal(result.status, 0, result.stderr);
  const assigned = results.flatMap(result => result.stdout.trim().split('\n'));
  assert.deepEqual(assigned.sort(), [...new Set(selected.map(example => example.file_path))].sort());
});

test('file shards work without any measured timings', t => {
  const results = [1, 2].map(shard => runShard(t, shard, {}, examples, 'files'));
  for (const result of results) assert.equal(result.status, 0, result.stderr);
  const assigned = results.flatMap(result => result.stdout.trim().split('\n'));
  assert.deepEqual(assigned.sort(), examples.map(example => example.file_path).sort());
});

for (const timingSource of [null, 'missing-file']) {
  test(`shards work with ${timingSource === null ? 'no timing option' : 'a missing timing file'}`, t => {
    const result = runShard(t, 1, timingSource);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(JSON.parse(result.stderr).estimated, examples.length);
  });
}

test('assignment is stable when manifest order changes', t => {
  assert.equal(runShard(t, 1, {}, examples, 'files').stdout,
    runShard(t, 1, {}, [...examples].reverse(), 'files').stdout);
});

test('empty manifests fail instead of accidentally running the complete suite', t => {
  const result = runShard(t, 1, {}, []);
  assert.notEqual(result.status, 0);
  assert.equal(result.stdout, '');
  assert.match(result.stderr, /No examples in manifest/);
});

for (const index of [0, -1, 3, 1.5]) {
  test(`shards reject invalid index ${index}`, t => {
    const result = runShard(t, index);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Shard must be an integer within/);
  });
}
