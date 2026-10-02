import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const script = fileURLToPath(new URL('../../../bin/lighthouse_score_gate.js', import.meta.url));

function runGate(t, scores, { malformed = false, audits = {} } = {}) {
  const directory = mkdtempSync(join(tmpdir(), 'ci-lighthouse-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const prefix = join(directory, 'report');
  scores.forEach((score, index) => {
    if (score === undefined) return;
    const report = { categories: {
      performance: { score }, accessibility: { score: 0.95 }, 'best-practices': { score: 0.95 }
    }, audits };
    writeFileSync(`${prefix}-${index + 1}.report.json`, malformed && index === 1 ? '{' : JSON.stringify(report));
  });
  const result = spawnSync(process.execPath, [script, '--report-path', prefix, '--runs', '3',
    '--perf-threshold', '60', '--a11y-threshold', '90', '--bp-threshold', '90'], { encoding: 'utf8' });
  return { ...result, output: result.stdout + result.stderr };
}

test('Lighthouse uses the median despite one low outlier', t => {
  const result = runGate(t, [0.45, 0.77, 0.72]);
  assert.equal(result.status, 0, result.output);
  assert.match(result.output, /Performance:\s+72%/);
  assert.match(result.output, /All scores meet thresholds/);
});

test('Lighthouse reports failing audits below the threshold', t => {
  const result = runGate(t, [0.55, 0.58, 0.77], {
    audits: { 'first-contentful-paint': { score: 0.5, title: 'First Contentful Paint' } }
  });
  assert.notEqual(result.status, 0);
  assert.match(result.output, /Performance score below threshold/);
  assert.match(result.output, /\[50%\] First Contentful Paint/);
});

test('Lighthouse floors boundary scores consistently', t => {
  const result = runGate(t, [0.45, 0.599, 0.77]);
  assert.notEqual(result.status, 0);
  assert.match(result.output, /Performance:\s+59%/);
  assert.doesNotMatch(result.output, /All scores meet thresholds/);
});

test('Lighthouse rejects a missing report', t => {
  const result = runGate(t, [0.72, undefined, 0.77]);
  assert.notEqual(result.status, 0);
  assert.match(result.output, /Missing Lighthouse JSON report/);
});

test('Lighthouse rejects malformed JSON', t => {
  const result = runGate(t, [0.72, 0.75, 0.77], { malformed: true });
  assert.notEqual(result.status, 0);
  assert.match(result.output, /Malformed Lighthouse JSON report/);
});
