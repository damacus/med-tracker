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
  const timings = [];
  for (const variable of ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'DATABASE_URL', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH']) delete environment[variable];
  const run = async (name, timeout) => {
    const started = performance.now();
    try { return await runTask([name, project], { env: environment, timeout }); } finally { timings.push({ name, durationMs: performance.now() - started }); }
  };
  let failed = false;
  try {
    await run('foundation:db-up', 60000);
    const endpoint = String(await run('foundation:db-port', 10000)).trim();
    const match = /^127\.0\.0\.1:(\d+)$/.exec(endpoint);
    if (!match || Number(match[1]) < 1 || Number(match[1]) > 65535) throw new Error(`Invalid owned PostgreSQL endpoint: ${endpoint}`);
    const storage = async () => {
      await run('foundation:storage-up',60000);
      const endpoint = String(await run('foundation:storage-port',10000)).trim();
      const storagePort = /^127\.0\.0\.1:(\d+)$/.exec(endpoint);
      if (!storagePort || Number(storagePort[1])<1 || Number(storagePort[1])>65535) throw new Error('Invalid owned object storage endpoint');
      const url = `http://${endpoint}`;
      await runTask(['foundation:storage-init',`STORAGE_ENDPOINT=${url}`],{env:environment,timeout:10000});
      return {MEDTRACKER_OWNED_STORAGE:'1',S3_ENDPOINT:url,S3_BUCKET:'scanner-fixture',AWS_REGION:'us-east-1',AWS_ACCESS_KEY_ID:'scanner-fixture',AWS_SECRET_ACCESS_KEY:'synthetic-scanner-fixture-password'};
    };
    return await callback(`postgres://medtracker:medtracker_password@127.0.0.1:${match[1]}/medtracker_loco`, () => run('foundation:db-provision', 60000), { project: project.slice('FOUNDATION_PROJECT='.length), timings, storage });
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    try { await run('foundation:db-down', 120000); } catch (error) {
      if (!failed) throw error;
      process.stderr.write(`Owned database cleanup failed: ${error.message}\n`);
    }
  }
}
