import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

test('CI runs cheap contracts and units before preparation and integration or browsers', () => {
  const result = spawnSync('task', ['--dry', '--force', '--verbose', 'ci'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const output = result.stdout + result.stderr;
  const unit = output.indexOf('cargo test --locked --lib');
  const prepare = output.indexOf('loco-build.mjs');
  assert.ok(unit >= 0, 'Library tests must be an explicit first stage');
  assert.ok(prepare > unit, 'Compile integration targets only after units');
  assert.ok(output.indexOf('tracked_paths.mjs') < unit);
  assert.ok(output.indexOf('relocation.test.mjs') < unit);
  assert.ok(output.indexOf('loco-integration.mjs') > prepare);
  assert.ok(output.indexOf('loco-browser.mjs') > prepare);
});

test('a real Task invocation stops before preparation or acceptance when units fail', () => {
  const directory = mkdtempSync(join(tmpdir(), 'loco-stage-failure-'));
  const log = join(directory, 'calls');
  const stub = `#!${process.execPath}
const fs = require('node:fs');
fs.appendFileSync(process.env.LOCO_STAGE_LOG, process.argv.slice(1).join(' ') + '\\n');
if (process.argv.includes('--lib')) process.exit(42);
`;
  try {
    for (const name of ['node', 'cargo']) writeFileSync(join(directory, name), stub, { mode: 0o755 });
    const result = spawnSync('task', ['--force', 'test'], { encoding: 'utf8', env: { ...process.env, PATH: `${directory}:${process.env.PATH}`, LOCO_STAGE_LOG: log } });
    assert.notEqual(result.status, 0);
    const calls = readFileSync(log, 'utf8');
    assert.match(calls, /cargo test --locked --lib/);
    assert.doesNotMatch(calls, /loco-build|loco-integration|loco-browser/);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
