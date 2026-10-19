import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { stopOwnedProcess } from './process-cleanup.mjs';

export async function execute(command, args, { env = process.env, capture = false } = {}) {
  const child = spawn(command, args, { env, detached: true, stdio: ['ignore', capture ? 'pipe' : 'inherit', 'inherit'] });
  let output = '';
  child.stdout?.on('data', chunk => { output += chunk; });
  const interrupt = () => stopOwnedProcess(child).catch(error => process.stderr.write(`${error.message}\n`));
  process.on('SIGINT', interrupt);
  process.on('SIGTERM', interrupt);
  try {
    await new Promise((resolve, reject) => {
      child.once('error', reject);
      child.once('close', code => code === 0 ? resolve() : reject(new Error(`${command} exited ${code}`)));
    });
    return output;
  } finally {
    process.off('SIGINT', interrupt);
    process.off('SIGTERM', interrupt);
    await stopOwnedProcess(child);
  }
}

export async function browserDatabasePool(url, provision, ownership, run = (name, variables) => execute('task', [name, ...variables], { capture: true })) {
  const template = new URL(url);
  if (template.hostname !== '127.0.0.1' || !/^mtloco-http-[a-f0-9-]{36}$/.test(ownership.project)) throw new Error('Browser pool requires owned loopback PostgreSQL');
  template.pathname = '/medtracker_reference';
  if (!ownership.templatePrepared) {
    await provision();
    await run('browser-care:seed', [`CARE_DATABASE_URL=${template.href}`]);
    await run('browser-care:freeze-template', [`CARE_DATABASE_URL=${url}`]);
  }
  return async callback => {
    const database = `browser_${randomUUID().replaceAll('-', '')}`;
    const variables = [`CARE_DATABASE_URL=${url}`, `CARE_DATABASE_NAME=${database}`];
    const privateUrl = new URL(url);
    privateUrl.pathname = `/${database}`;
    let failed = false;
    try {
      await run('browser-care:clone', variables);
      const storage = async (avatar = false) => {
        const environment = { ...(avatar ? await ownership.avatarStorage() : await ownership.storage()) };
        const endpoint = avatar ? environment.ACTIVE_STORAGE_S3_ENDPOINT : environment.S3_ENDPOINT;
        if (!endpoint) throw new Error('Browser storage was not selected for this suite');
        const bucket = `${avatar ? 'avatar' : 'scanner'}-${database.replaceAll('_', '-')}`;
        await run('browser-care:storage-bucket', [`STORAGE_ENDPOINT=${endpoint}`, `STORAGE_BUCKET=${bucket}`]);
        environment[avatar ? 'ACTIVE_STORAGE_S3_BUCKET' : 'S3_BUCKET'] = bucket;
        return environment;
      };
      let avatar;
      let scanner;
      return await callback(privateUrl.href, async () => {}, { ...ownership, databaseName: database, runtimeRole: `${database}_runtime`, templatePrepared: true,
        avatarStorage: () => avatar ??= storage(true), storage: () => scanner ??= storage(false) });
    } catch (error) {
      failed = true;
      throw error;
    } finally {
      try { await run('browser-care:drop', variables); } catch (error) {
        if (!failed) throw error;
        process.stderr.write(`Browser database cleanup failed: ${error.message}\n`);
      }
    }
  };
}

export async function sharedBrowserDatabase(descriptor) {
  const session = JSON.parse(descriptor);
  const url = new URL(session.url);
  if (!/^mtloco-http-[a-f0-9-]{36}$/.test(session.project)
      || url.hostname !== '127.0.0.1' || url.protocol !== 'postgres:'
      || url.username !== 'medtracker' || url.password !== 'medtracker_password'
      || url.pathname !== '/medtracker_loco' || !url.port || !session.templatePrepared) throw new Error('Invalid shared browser database ownership');
  const endpoint = (await execute('task', ['foundation:db-port', `FOUNDATION_PROJECT=${session.project}`], { capture: true })).trim();
  if (endpoint !== `${url.hostname}:${url.port}`) throw new Error('Shared database does not belong to this Compose project');
  return browserDatabasePool(session.url, async () => {}, { ...session, timings: [],
    avatarStorage: async () => session.storageEnvironment, storage: async () => session.scannerEnvironment });
}
