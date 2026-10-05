import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { execFileSync, spawnSync } from 'node:child_process';
import test from 'node:test';

test('preservation check rejects byte drift without changing comments or modes', { timeout: 120000 }, () => {
  const fixture = mkdtempSync(join(tmpdir(), 'medtracker-preservation-'));
  try {
    for (const path of ['app', 'scripts/migration', 'rails/app', 'docs/plans/loco-migration-20261005']) mkdirSync(join(fixture, path), { recursive: true });
    const original = 'class Example\nend\n';
    writeFileSync(join(fixture, 'app/example.rb'), original);
    const environment = { ...process.env, GIT_AUTHOR_NAME: 'Preservation fixture', GIT_AUTHOR_EMAIL: 'fixture@example.test', GIT_COMMITTER_NAME: 'Preservation fixture', GIT_COMMITTER_EMAIL: 'fixture@example.test' };
    delete environment.GIT_DIR;
    delete environment.GIT_WORK_TREE;
    const git = (...args) => execFileSync('git', args, { cwd: fixture, env: environment, stdio: 'pipe' });
    git('init', '--quiet');
    git('add', 'app/example.rb');
    git('-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'Fixture baseline');
    cpSync('scripts/migration/preservation.mjs', join(fixture, 'scripts/migration/preservation.mjs'));
    const target = join(fixture, 'rails/app/example.rb');
    renameSync(join(fixture, 'app/example.rb'), target);
    const check = (...args) => spawnSync(process.execPath, ['scripts/migration/preservation.mjs', ...args], {
      cwd: fixture, env: environment, encoding: 'utf8', timeout: 10000,
    });
    const written = check('--write');
    assert.equal(written.status, 0, written.stderr);
    const before = check();
    assert.equal(before.status, 0, before.stderr);
    writeFileSync(target, readFileSync(target, 'utf8').replace('class Example', 'class DriftExample'));
    const after = check();
    assert.notEqual(after.status, 0, 'Byte drift without comment or mode changes was accepted');
    assert.match(after.stderr, /Preservation input changed/);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});
