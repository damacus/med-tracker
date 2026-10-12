import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const checker = fileURLToPath(new URL('../tracked_paths.mjs', import.meta.url));

function checkFile(name, { staged = true, ignored = false } = {}) {
  const cwd = mkdtempSync(join(tmpdir(), 'medtracker-ci-paths-'));
  try {
    execFileSync('git', ['init', '--quiet'], { cwd });
    if (ignored) writeFileSync(join(cwd, '.gitignore'), `${name}\n`);
    mkdirSync(dirname(join(cwd, name)), { recursive: true });
    writeFileSync(join(cwd, name), 'synthetic input');
    if (staged) execFileSync('git', ['add', '--', name], { cwd });
    return spawnSync(process.execPath, [checker], { cwd, encoding: 'utf8' });
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}

test('local CI accepts a staged root Rust build script before its first commit', () => {
  const result = checkFile('build.rs');
  assert.equal(result.status, 0, result.stderr);
});

test('local CI rejects an unmapped staged file and identifies its full name', () => {
  const result = checkFile('unmapped build input');
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unmapped paths; update scripts\/ci\/policy.json:\nunmapped build input/);
});

test('local CI accepts a new file in an owned frontend scripts directory', () => {
  const result = checkFile('scripts/frontend/new build helper.mjs', { staged: false });
  assert.equal(result.status, 0, result.stderr);
});

test('local CI rejects an unmapped untracked file and preserves spaces in its name', () => {
  const result = checkFile('scripts/unknown build helper.mjs', { staged: false });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unmapped paths; update scripts\/ci\/policy.json:\nscripts\/unknown build helper.mjs/);
});

test('local CI excludes ignored untracked output', () => {
  const result = checkFile('unknown generated output', { staged: false, ignored: true });
  assert.equal(result.status, 0, result.stderr);
});

test('local CI retains an unstaged tracked deletion until it is staged', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'medtracker-ci-paths-'));
  try {
    execFileSync('git', ['init', '--quiet'], { cwd });
    mkdirSync(join(cwd, 'scripts'), { recursive: true });
    writeFileSync(join(cwd, 'scripts/old-helper.mjs'), 'removed');
    execFileSync('git', ['add', '--', 'scripts/old-helper.mjs'], { cwd });
    unlinkSync(join(cwd, 'scripts/old-helper.mjs'));
    const unstaged = spawnSync(process.execPath, [checker], { cwd, encoding: 'utf8' });
    assert.notEqual(unstaged.status, 0);
    assert.match(unstaged.stderr, /scripts\/old-helper.mjs/);
    execFileSync('git', ['add', '-u'], { cwd });
    const staged = spawnSync(process.execPath, [checker], { cwd, encoding: 'utf8' });
    assert.equal(staged.status, 0, staged.stderr);
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
});
