import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

test('coverage advisory lock recovers after SIGKILL and keeps live owners exclusive', { timeout: 10000 }, async () => {
  const { acquireCoverageLock } = await import('./coverage-lock.mjs');
  const directory = await mkdtemp(join(tmpdir(), 'coverage-lock-'));
  const lockPath = join(directory, 'run.flock');
  await writeFile(lockPath, 'leftover lock file');
  const holder = spawn(process.execPath, ['--input-type=module', '-e', `
    const { acquireCoverageLock } = await import(${JSON.stringify(new URL('./coverage-lock.mjs', import.meta.url).href)});
    await acquireCoverageLock(process.argv[1]);
    process.stdout.write('ready');
    setInterval(() => {}, 1000);
  `, lockPath], { stdio: ['ignore', 'pipe', 'pipe'] });
  const closed = once(holder, 'close');
  try {
    await Promise.race([once(holder.stdout, 'data'), closed.then(() => { throw new Error('Lock holder exited before acquiring ownership'); })]);
    await assert.rejects(acquireCoverageLock(lockPath), /already in use/);
    holder.kill('SIGKILL');
    const [, signal] = await closed;
    assert.equal(signal, 'SIGKILL');
    let release;
    for (let attempt = 0; attempt < 50; attempt++) {
      try {
        release = await acquireCoverageLock(lockPath);
        break;
      } catch (error) {
        if (!/already in use/.test(error.message)) throw error;
        await new Promise(resolve => setTimeout(resolve, 20));
      }
    }
    assert.equal(typeof release, 'function', 'A killed runner must release its advisory lock');
    await assert.rejects(acquireCoverageLock(lockPath), /already in use/);
    await release();
    const releaseAgain = await acquireCoverageLock(lockPath);
    await releaseAgain();
  } finally {
    if (holder.exitCode === null && holder.signalCode === null) holder.kill('SIGKILL');
    await closed;
    await rm(directory, { recursive: true, force: true });
  }
});
