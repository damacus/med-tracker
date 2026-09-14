import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const script = fileURLToPath(new URL('../browser_shards.mjs', import.meta.url));
const examples = ['a', 'b', 'c', 'd'].map(name => ({
  id: `./spec/${name}_spec.rb[1:1]`, file_path: `./spec/${name}_spec.rb`
}));
const timings = Object.fromEntries(examples.map((example, index) => [example.file_path, 8 - index]));

function runShard(t, shard, durations = timings, selected = examples) {
  const directory = mkdtempSync(join(tmpdir(), 'ci-shards-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const input = join(directory, 'examples.json');
  const measured = join(directory, 'timings.json');
  writeFileSync(input, JSON.stringify({ examples: selected }));
  writeFileSync(measured, JSON.stringify(durations));
  return spawnSync(process.execPath, [script, '--input', input, '--timings', measured,
    '--shards', '2', '--shard', String(shard)], { encoding: 'utf8' });
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
  test(`browser shards reject ${duration ?? 'missing'} timing data`, t => {
    const result = runShard(t, 1, { ...timings, [examples[0].file_path]: duration });
    assert.notEqual(result.status, 0);
    assert.equal(result.stdout, '');
    assert.match(result.stderr, /Missing measured timing/);
  });
}

test('browser shards reject duplicate examples', t => {
  const result = runShard(t, 1, timings, [examples[0], examples[0]]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Duplicate or missing browser example id/);
});
