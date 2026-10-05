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

test('canonical preservation is independent of global and local Git ignore configuration', { timeout: 120000 }, () => {
  const fixture = mkdtempSync(join(tmpdir(), 'medtracker-preservation-ignore-'));
  try {
    for (const path of ['scripts/migration', 'rails', 'docs/plans/loco-migration-20261005']) mkdirSync(join(fixture, path), { recursive: true });
    writeFileSync(join(fixture, '.ruby-version'), '4.0.6\n');
    const ignored = join(fixture, 'ignored-global-config');
    const clean = join(fixture, 'clean-global-config');
    const patterns = join(fixture, 'global-ignore');
    writeFileSync(patterns, '.ruby-version\n');
    writeFileSync(ignored, `[core]\nexcludesFile = ${patterns}\n`);
    writeFileSync(clean, '');
    const environment = { ...process.env, GIT_CONFIG_GLOBAL: clean, GIT_CONFIG_NOSYSTEM: '1', GIT_AUTHOR_NAME: 'Preservation fixture', GIT_AUTHOR_EMAIL: 'fixture@example.test', GIT_COMMITTER_NAME: 'Preservation fixture', GIT_COMMITTER_EMAIL: 'fixture@example.test' };
    delete environment.GIT_DIR;
    delete environment.GIT_WORK_TREE;
    const git = (...args) => execFileSync('git', args, { cwd: fixture, env: environment, stdio: 'pipe' });
    git('init', '--quiet');
    git('add', '--force', '.ruby-version');
    git('-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'Fixture baseline');
    cpSync('scripts/migration/preservation.mjs', join(fixture, 'scripts/migration/preservation.mjs'));
    renameSync(join(fixture, '.ruby-version'), join(fixture, 'rails/.ruby-version'));
    const check = (config, ...args) => spawnSync(process.execPath, ['scripts/migration/preservation.mjs', ...args], {
      cwd: fixture, env: { ...environment, GIT_CONFIG_GLOBAL: config }, encoding: 'utf8', timeout: 10000,
    });
    const written = check(ignored, '--write');
    assert.equal(written.status, 0, written.stderr);
    const ledger = JSON.parse(readFileSync(join(fixture, 'docs/plans/loco-migration-20261005/preservation.json'), 'utf8'));
    assert.equal(ledger.entries[0].force_stage_tracked_relocation, true);
    const withoutGlobalIgnore = check(clean);
    assert.equal(withoutGlobalIgnore.status, 0, withoutGlobalIgnore.stderr);
    writeFileSync(join(fixture, '.git/info/exclude'), '.ruby-version\n');
    const withLocalIgnore = check(clean);
    assert.equal(withLocalIgnore.status, 0, withLocalIgnore.stderr);
    writeFileSync(join(fixture, '.git/info/exclude'), '');
    writeFileSync(join(fixture, 'rails/.ruby-version'), '4.0.7\n');
    const drift = check(ignored);
    assert.notEqual(drift.status, 0, 'Ignoring environment-specific staging advice must not accept changed bytes');
    assert.match(drift.stderr, /Preservation input changed/);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});
