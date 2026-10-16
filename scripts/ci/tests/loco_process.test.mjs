import assert from 'node:assert/strict';
import test from 'node:test';
import { execute } from '../loco-process.mjs';

test('captured runner output has a bounded memory budget', async () => {
  await assert.rejects(execute(process.execPath, ['-e', 'process.stdout.write("x".repeat(2048))'], { capture: true, maxOutputBytes: 256 }), /output.*limit/i);
});

test('a hung archived runner is stopped at its deadline', { timeout: 5000 }, async () => {
  await assert.rejects(execute(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { timeoutMs: 100 }), /deadline/i);
});
