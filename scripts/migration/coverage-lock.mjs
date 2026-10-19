import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export async function acquireCoverageLock(path, { onLost = () => {} } = {}) {
  const child = spawn('python3', ['-I', '-u', fileURLToPath(new URL('./coverage-lock.py', import.meta.url)), path], { stdio: ['pipe', 'pipe', 'pipe'] });
  let stderr = '';
  let releasing = false;
  let acquired = false;
  child.stderr.on('data', chunk => { stderr += chunk; });
  child.stdin.on('error', error => { if (acquired && !releasing) onLost(error); });
  const closed = new Promise(resolve => child.once('close', (code, signal) => {
    if (acquired && !releasing) onLost(new Error(`Coverage lock owner exited unexpectedly (${code ?? signal})`));
    resolve({ code, signal });
  }));
  try {
    await new Promise((resolve, reject) => {
      child.once('error', reject);
      child.stdout.once('data', chunk => {
        if (chunk.toString().trim() !== 'locked') reject(new Error('Invalid coverage lock handshake'));
        else { acquired = true; resolve(); }
      });
      closed.then(({ code, signal }) => reject(new Error(stderr.trim() || `Coverage lock failed (${code ?? signal}); Python 3 with fcntl is required`)));
    });
  } catch (error) {
    child.stdin.end();
    await closed;
    throw error;
  }
  return async () => {
    releasing = true;
    child.stdin.end();
    const { code, signal } = await closed;
    if (code !== 0) throw new Error(stderr.trim() || `Coverage lock release failed (${code ?? signal})`);
  };
}
