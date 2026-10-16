import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import { stopOwnedProcess } from '../migration/process-cleanup.mjs';

export async function execute(command, args, { env = process.env, capture = false, maxOutputBytes = 64 * 1024 * 1024, timeoutMs = 30 * 60 * 1000 } = {}) {
  const started = performance.now();
  const child = spawn(command, args, { env, detached: true, stdio: ['ignore', capture ? 'pipe' : 'inherit', 'inherit'] });
  let output = '';
  let outputBytes = 0;
  const interrupt = () => stopOwnedProcess(child).catch(error => process.stderr.write(`${error.message}\n`));
  process.on('SIGINT', interrupt);
  process.on('SIGTERM', interrupt);
  let timer;
  try {
    await new Promise((resolve, reject) => {
      child.stdout?.on('data', chunk => {
        outputBytes += chunk.length;
        if (outputBytes > maxOutputBytes) reject(new Error('Captured runner output exceeded limit'));
        else output += chunk;
      });
      timer = setTimeout(() => reject(new Error(`${command} exceeded runner deadline`)), timeoutMs);
      child.once('error', reject);
      child.once('close', code => code === 0 ? resolve() : reject(new Error(`${command} exited ${code}`)));
    });
    return output;
  } finally {
    clearTimeout(timer);
    process.off('SIGINT', interrupt);
    process.off('SIGTERM', interrupt);
    await stopOwnedProcess(child);
    await mkdir('test-results/timings', { recursive: true });
    await writeFile(`test-results/timings/${randomUUID()}.json`, JSON.stringify({ command, args, durationMs: performance.now() - started }));
  }
}
