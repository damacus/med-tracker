import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const checker = fileURLToPath(new URL('../tracked_paths.mjs', import.meta.url));

function checkFile(name) {
  const cwd = mkdtempSync(join(tmpdir(), 'medtracker-ci-paths-'));
  try {
    execFileSync('git', ['init', '--quiet'], { cwd });
    writeFileSync(join(cwd, name), 'synthetic input');
    execFileSync('git', ['add', '--', name], { cwd });
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
