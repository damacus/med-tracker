import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

test('hosted relocation checks run after their tools are installed', () => {
  const workflow = readFileSync('.github/workflows/ci.yml', 'utf8').split('  loco_foundation:')[1].split('  loco_lint:')[0];
  const install = workflow.indexOf('sudo apt-get install --yes fish uuid-runtime ripgrep');
  assert.ok(install >= 0, 'Install every relocation and runner prerequisite');
  assert.ok(install < workflow.indexOf('run: task ci:paths foundation:test-relocation fmt'));
});

test('legacy household shards wait for Loco browsers without skipping legacy-only changes', () => {
  const workflow = readFileSync('.github/workflows/ci.yml', 'utf8');
  const job = workflow.match(/\n  rust_household_browser:\n([\s\S]*?)(?=\n  [a-z_]+:\n)/)?.[1];
  const dependencies = job.match(/needs: \[([^\]]+)\]/)?.[1].split(',').map(value => value.trim());
  assert.ok(dependencies?.includes('changes'));
  assert.ok(dependencies.includes('loco_browser'), 'Reserve runners for the Loco chain before the legacy matrix');
  const condition = job.match(/if: \$\{\{ (.+) \}\}/)?.[1];
  const selected = new Function('needs', 'cancelled', `return (${condition});`);
  for (const result of ['success', 'skipped', 'failure', 'cancelled']) {
    const needs = { changes: { outputs: { rust_port: 'true', loco: 'false' } }, loco_browser: { result } };
    assert.equal(selected(needs, () => false), ['success', 'skipped'].includes(result), result);
    needs.changes.outputs.loco = 'true';
    assert.equal(selected(needs, () => false), result === 'success', 'Do not run legacy browsers after Loco unit failure');
    assert.equal(selected(needs, () => true), false, 'Do not start work after workflow cancellation');
    needs.changes.outputs.rust_port = 'false';
    assert.equal(selected(needs, () => false), false, 'Unselected legacy suites stay skipped');
  }
});

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
