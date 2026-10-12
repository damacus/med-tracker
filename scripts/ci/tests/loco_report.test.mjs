import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

test('timing summaries aggregate phases without exposing command credentials', () => {
  const directory = mkdtempSync(join(tmpdir(), 'loco-timings-'));
  try {
    mkdirSync(join(directory, 'test-results/timings'), { recursive: true });
    for (const id of ['one', 'two']) writeFileSync(join(directory, `test-results/timings/${id}.json`), JSON.stringify({ command: 'task', args: ['browser-care:clone', 'CARE_DATABASE_URL=private-credential'], durationMs: 1500 }));
    const result = spawnSync(process.execPath, [fileURLToPath(new URL('../loco-report.mjs', import.meta.url))], { cwd: directory, encoding: 'utf8', env: { ...process.env, GITHUB_STEP_SUMMARY: '' } });
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stdout, /private-credential/);
    assert.match(result.stdout, /browser-care:clone \| 2 \| 3.00/);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
