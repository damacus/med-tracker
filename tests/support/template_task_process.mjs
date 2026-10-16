import { spawn } from 'node:child_process';
import { stopOwnedProcess } from '../../scripts/migration/process-cleanup.mjs';

export async function runOwnedTemplateTask(command, args, {
  cwd = process.cwd(), env = process.env, timeoutMs = 900000, spawnProcess = spawn,
} = {}) {
  const child = spawnProcess(command, args, { cwd, env, detached: true, stdio: 'inherit' });
  let timer;
  const signals = new Map();
  try {
    const code = await new Promise((resolve, reject) => {
      for (const signal of ['SIGINT', 'SIGTERM']) {
        const handler = () => reject(new Error(`template concurrency task interrupted by ${signal}`));
        signals.set(signal, handler);
        process.once(signal, handler);
      }
      timer = setTimeout(() => reject(new Error(`template concurrency task exceeded ${timeoutMs}ms`)), timeoutMs);
      child.once('error', reject);
      child.once('close', resolve);
    });
    if (code !== 0) throw new Error(`template concurrency task exited ${code}`);
  } finally {
    clearTimeout(timer);
    for (const [signal, handler] of signals) process.removeListener(signal, handler);
    await stopOwnedProcess(child);
  }
}
