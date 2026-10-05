import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { stopOwnedProcess } from './process-cleanup.mjs';

async function runFoundationTask(args, options) {
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const taskfile = fileURLToPath(new URL('../../Taskfile.yml', import.meta.url));
  const child = spawn('task', ['--taskfile', taskfile, ...args], { cwd: root, env: options.env, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  let errors = '';
  child.stdout.on('data', chunk => { output += chunk; });
  child.stderr.on('data', chunk => { errors += chunk; });
  let timer;
  let failed = false;
  try {
    await new Promise((resolve, reject) => {
      timer = setTimeout(() => reject(new Error(`${args[0]} exceeded ${options.timeout}ms: ${errors}`)), options.timeout);
      child.on('error', reject);
      child.on('close', code => code === 0 ? resolve() : reject(new Error(`${args[0]} exited ${code}: ${errors}`)));
    });
    return output;
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    clearTimeout(timer);
    try { await stopOwnedProcess(child); } catch (error) {
      if (!failed) throw error;
      process.stderr.write(`Owned database Task cleanup failed: ${error.message}\n`);
    }
  }
}

export async function withOwnedDatabase(callback, runTask = runFoundationTask, inheritedEnvironment = process.env) {
  const project = `FOUNDATION_PROJECT=mtloco-http-${randomUUID()}`;
  const environment = { ...inheritedEnvironment };
  for (const variable of ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'DATABASE_URL']) delete environment[variable];
  const run = (name, timeout) => runTask([name, project], { env: environment, timeout });
  let failed = false;
  try {
    await run('foundation:db-up', 60000);
    const endpoint = String(await run('foundation:db-port', 10000)).trim();
    const match = /^127\.0\.0\.1:(\d+)$/.exec(endpoint);
    if (!match || Number(match[1]) < 1 || Number(match[1]) > 65535) throw new Error(`Invalid owned PostgreSQL endpoint: ${endpoint}`);
    return await callback(`postgres://medtracker:medtracker_password@127.0.0.1:${match[1]}/medtracker_loco`);
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    try { await run('foundation:db-down', 20000); } catch (error) {
      if (!failed) throw error;
      process.stderr.write(`Owned database cleanup failed: ${error.message}\n`);
    }
  }
}
