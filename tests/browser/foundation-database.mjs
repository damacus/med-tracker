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

export async function withOwnedDatabase(callback, runTask = runFoundationTask, inheritedEnvironment = process.env, { storage = false } = {}) {
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
    let storageEndpoint;
    const ensureStorage = async () => {
      if (storageEndpoint) return storageEndpoint;
      await run('foundation:storage-up', 180000);
      const endpoint = String(await run('foundation:storage-port', 10000)).trim();
      const storageMatch = /^127\.0\.0\.1:(\d+)$/.exec(endpoint);
      if (!storageMatch || Number(storageMatch[1]) < 1 || Number(storageMatch[1]) > 65535) throw new Error(`Invalid owned storage endpoint: ${endpoint}`);
      storageEndpoint = `http://${endpoint}`;
      return storageEndpoint;
    };
    let scannerEnvironment;
    const scannerStorage = async () => {
      if (scannerEnvironment) return scannerEnvironment;
      const endpoint = await ensureStorage();
      await runTask(['foundation:storage-init', `STORAGE_ENDPOINT=${endpoint}`], { env: environment, timeout: 10000 });
      scannerEnvironment = { MEDTRACKER_OWNED_STORAGE: '1', S3_ENDPOINT: endpoint, S3_BUCKET: 'scanner-fixture', AWS_REGION: 'us-east-1', AWS_ACCESS_KEY_ID: 'storage-smoke-access', AWS_SECRET_ACCESS_KEY: 'storage-smoke-secret' };
      return scannerEnvironment;
    };
    const avatarStorage = async () => {
      const endpoint = await ensureStorage();
      return {
        ACTIVE_STORAGE_SERVICE: 's3',
        ACTIVE_STORAGE_S3_ENDPOINT: endpoint,
        ACTIVE_STORAGE_S3_BUCKET: 'medtracker-fixture',
        ACTIVE_STORAGE_S3_REGION: 'us-east-1',
        ACTIVE_STORAGE_S3_ACCESS_KEY_ID: 'storage-smoke-access',
        ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY: 'storage-smoke-secret',
        ACTIVE_STORAGE_S3_FORCE_PATH_STYLE: 'true',
      };
    };
    const storageEnvironment = storage ? await avatarStorage() : {};
    return await callback(`postgres://medtracker:medtracker_password@127.0.0.1:${match[1]}/medtracker_loco`, () => run('foundation:db-provision', 60000), { project: project.slice('FOUNDATION_PROJECT='.length), timings, storageEnvironment, storage: scannerStorage, avatarStorage });
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
