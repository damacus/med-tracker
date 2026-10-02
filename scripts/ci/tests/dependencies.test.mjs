import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { verify } from '../../test_dependency_verification.js';

for (const scenario of ['current', 'stale', 'refresh failure', 'launch failure']) {
  test(`dependency verification handles ${scenario}`, async t => {
    const root = mkdtempSync(join(tmpdir(), 'ci-dependencies-'));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    const packageLockPath = join(root, 'package-lock.json');
    const markerPath = join(root, 'node_modules', 'marker');
    mkdirSync(join(root, 'node_modules'));
    writeFileSync(packageLockPath, 'lock contents');
    const hash = createHash('sha256').update('lock contents').digest('hex');
    writeFileSync(markerPath, scenario === 'stale' || scenario === 'refresh failure' ? 'stale' : hash);
    const calls = [];
    const options = {
      packageLockPath, markerPath,
      execFileSync(command, args) {
        assert.equal(command, 'npm');
        assert.deepEqual(args, ['ci', '--ignore-scripts']);
        calls.push('refresh');
        if (scenario === 'refresh failure') throw new Error('refresh failed');
      },
      async launch() {
        calls.push('launch');
        if (scenario === 'launch failure') throw new Error('launch failed');
        return { async close() { calls.push('close'); } };
      },
      runCapybara() { calls.push('capybara'); }
    };
    if (scenario === 'refresh failure') {
      await assert.rejects(verify(options), /refresh failed/);
      assert.deepEqual(calls, ['refresh']);
      assert.equal(readFileSync(markerPath, 'utf8'), 'stale');
    } else if (scenario === 'launch failure') {
      await assert.rejects(verify(options), /launch failed/);
      assert.deepEqual(calls, ['launch']);
    } else {
      await verify(options);
      assert.deepEqual(calls, [...(scenario === 'stale' ? ['refresh'] : []), 'launch', 'close', 'capybara']);
      assert.equal(readFileSync(markerPath, 'utf8').trim(), hash);
    }
  });
}
