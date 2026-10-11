import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { changedPaths, classify } from '../classify.mjs';

test('deleted scanner wrapper in a committed pull request diff remains classified', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'medtracker-ci-deletion-'));
  const original = process.cwd();
  try {
    execFileSync('git', ['init', '--quiet'], { cwd });
    mkdirSync(join(cwd, 'scripts'));
    writeFileSync(join(cwd, 'scripts/build-scanner.mjs'), 'old wrapper');
    execFileSync('git', ['add', '.'], { cwd });
    execFileSync('git', ['-c', 'user.name=CI Fixture', '-c', 'user.email=fixture@example.test', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'Add wrapper'], { cwd });
    const base = execFileSync('git', ['rev-parse', 'HEAD'], { cwd, encoding: 'utf8' }).trim();
    execFileSync('git', ['rm', '--quiet', 'scripts/build-scanner.mjs'], { cwd });
    execFileSync('git', ['-c', 'user.name=CI Fixture', '-c', 'user.email=fixture@example.test', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'Remove wrapper'], { cwd });
    process.chdir(cwd);
    const paths = changedPaths(base);
    assert.deepEqual(paths, ['scripts/build-scanner.mjs']);
    assert.equal(classify(paths).selected.loco, true);
  } finally {
    process.chdir(original);
    rmSync(cwd, { recursive: true, force: true });
  }
});
