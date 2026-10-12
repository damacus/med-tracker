import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

test('manual Loco benchmark selects the complete Loco chain without unrelated suites', () => {
  const result = spawnSync(process.execPath, ['scripts/ci/classify.mjs'], {
    env: { ...process.env, CI_EVENT: 'workflow_dispatch', CI_BASE: '', GITHUB_OUTPUT: '', GITHUB_STEP_SUMMARY: '' }, encoding: 'utf8'
  });
  assert.equal(result.status, 0, result.stderr);
  const { selected } = JSON.parse(result.stdout);
  assert.equal(selected.loco, true);
  assert.ok(Object.entries(selected).every(([name, value]) => name === 'loco' || value === false));
});
