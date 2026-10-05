import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import test from 'node:test';
import { stopOwnedProcess } from './process-cleanup.mjs';

test('owned child ignoring TERM is killed within a bounded deadline', { timeout: 5000 }, async () => {
  const child = spawn(process.execPath, ['-e', "process.on('SIGTERM', () => {}); process.stdout.write('ready'); setInterval(() => {}, 1000)"], { detached: true, stdio: ['ignore', 'pipe', 'ignore'] });
  await once(child.stdout, 'data');
  try {
    const started = Date.now();
    await stopOwnedProcess(child, { termMs: 100, killMs: 1000 });
    assert.equal(child.signalCode, 'SIGKILL');
    assert.ok(Date.now() - started < 2000);
    assert.throws(() => process.kill(child.pid, 0), { code: 'ESRCH' });
  } finally {
    try { process.kill(-child.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
  }
});

test('owned descendant is killed even when its parent exits on TERM', { timeout: 5000 }, async () => {
  const fixture = "const {spawn}=require('node:child_process'); const child=spawn(process.execPath,['-e',\"process.on('SIGTERM',()=>{}); process.stdout.write(String(process.pid)); setInterval(()=>{},1000)\"],{stdio:['ignore','pipe','ignore']}); child.stdout.on('data',data=>process.stdout.write(data)); setInterval(()=>{},1000)";
  const child = spawn(process.execPath, ['-e', fixture], { detached: true, stdio: ['ignore', 'pipe', 'ignore'] });
  const [data] = await once(child.stdout, 'data');
  const descendant = Number(data.toString());
  try {
    await stopOwnedProcess(child, { termMs: 100, killMs: 1000 });
    assert.throws(() => process.kill(descendant, 0), { code: 'ESRCH' });
  } finally {
    try { process.kill(-child.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
  }
});
