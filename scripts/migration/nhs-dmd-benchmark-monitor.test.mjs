import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { BenchmarkMonitor, isQuiet, quietWindow } from './nhs-dmd-benchmark-monitor.mjs';

const sample = { idlePercent: 90, load: [3, 3, 3], cpuCount: 12, memoryPressure: 1, swapouts: 100, competingProcesses: [] };

test('quiet gate rejects competing work, high load, CPU use and memory pressure', () => {
  assert.equal(isQuiet(sample), true);
  for (const change of [{ idlePercent: 79 }, { load: [13, 3, 3] }, { memoryPressure: 2 }, { competingProcesses: ['rustc'] }]) {
    assert.equal(isQuiet({ ...sample, ...change }), false);
  }
});

test('a sampling failure does not replace the original import failure', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'dmd-monitor-test-'));
  try {
    const monitor = new BenchmarkMonitor(directory);
    monitor.sample = async () => { throw new Error('sampling failed'); };
    const original = new Error('import failed');
    await assert.rejects(monitor.measure('loco', async () => { throw original; }), error => error === original);
    await assert.rejects(monitor.measure('loco', async () => {}), /sampling failed/);
  } finally {
    await rm(directory, { recursive: true });
  }
});

test('quiet window requires a full minute without pressure or fresh swap-outs', () => {
  const samples = [0, 10000, 20000, 30000, 40000, 50000, 60000].map(timeMs => ({ ...sample, timeMs }));
  assert.equal(quietWindow(samples.slice(1)), false);
  assert.equal(quietWindow(samples), true);
  assert.equal(quietWindow(samples.map((s, i) => ({ ...s, swapouts: 100 + i }))), false);
  assert.equal(quietWindow(samples.map((s, i) => ({ ...s, idlePercent: i === 3 ? 40 : 90 }))), false);
  assert.equal(quietWindow([samples[0], samples.at(-1)]), false);
  assert.equal(quietWindow(samples.map(s => ({ ...s, swapouts: undefined }))), false);
});
