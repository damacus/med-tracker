import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { withOwnedDatabase } from './foundation-database.mjs';
import { stopOwnedProcess } from './process-cleanup.mjs';

async function runCargo(args, databaseUrl, inheritedEnvironment, signal) {
  signal.throwIfAborted();
  const environment = { ...inheritedEnvironment, DATABASE_URL: databaseUrl, MEDTRACKER_OWNED_DATABASE: '1', MEDTRACKER_SESSION_KEY: Buffer.alloc(64, 7).toString('base64'), RAILS_SECRET_KEY_BASE: 'synthetic-slice-rails-verification-secret' };
  for (const name of ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH']) delete environment[name];
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const child = spawn('cargo', args, { cwd: root, env: environment, detached: true, stdio: 'inherit' });
  let timer;
  let interrupt;
  let failed = false;
  try {
    await new Promise((resolve, reject) => {
      interrupt = () => reject(signal.reason);
      signal.addEventListener('abort', interrupt, { once: true });
      timer = setTimeout(() => reject(new Error('Slice Cargo execution exceeded 30 minutes')), 1800000);
      child.once('error', reject);
      child.once('close', code => code === 0 ? resolve() : reject(new Error(`Slice Cargo execution exited ${code}`)));
    });
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    clearTimeout(timer);
    signal.removeEventListener('abort', interrupt);
    try { await stopOwnedProcess(child); } catch (error) {
      if (!failed) throw error;
      process.stderr.write(`Slice child cleanup failed: ${error.message}\n`);
    }
  }
}

export async function runSlice({ all = false, target, filter = '', captureCatalog = false }, inheritedEnvironment = process.env) {
  if (!all && !/^[A-Za-z0-9][A-Za-z0-9_-]*$/.test(target ?? '')) throw new Error('TARGET must name a root Rust test binary');
  if (filter && !/^[A-Za-z0-9_:]+$/.test(filter)) throw new Error('FILTER must name a Rust test');
  const args = ['test', '--locked'];
  if (!all) args.push('--test', target);
  if (filter) args.push(filter);
  if (captureCatalog) args.push('--', '--ignored');
  const controller = new AbortController();
  const interrupted = signal => controller.abort(new Error(`Slice execution interrupted by ${signal}`));
  const onTerm = () => interrupted('SIGTERM');
  const onInt = () => interrupted('SIGINT');
  process.on('SIGTERM', onTerm);
  process.on('SIGINT', onInt);
  try {
    await withOwnedDatabase(async (url, provision, {storage}) => {
      if (all || ['persistence', 'tenant_access', 'care_doses', 'care_medications', 'care_api', 'identity_compatibility', 'identity_resource', 'oauth_server', 'queue_runtime'].includes(target)) await provision();
      const needsStorage = all || (target === 'care_api' && (!filter || filter.includes('nhs_dmd')));
      const storageEnvironment = needsStorage ? await storage() : {};
      await runCargo(args, url, {...inheritedEnvironment,...storageEnvironment}, controller.signal);
    }, undefined, inheritedEnvironment);
  } finally {
    process.off('SIGTERM', onTerm);
    process.off('SIGINT', onInt);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const arguments_ = process.argv.slice(2);
    if (arguments_.some((argument, index) => !['--all', '--capture-catalog', '--target', '--filter'].includes(argument) && !['--target', '--filter'].includes(arguments_[index - 1]))) throw new Error('Unsupported slice runner argument');
    const all = process.argv.includes('--all');
    const captureCatalog = process.argv.includes('--capture-catalog');
    const target = arguments_[arguments_.indexOf('--target') + 1];
    const filter = arguments_[arguments_.indexOf('--filter') + 1];
    if ((arguments_.includes('--target') && target === undefined) || (arguments_.includes('--filter') && filter === undefined)) throw new Error('Missing slice runner argument value');
    if (all && captureCatalog) throw new Error('Catalog capture requires its dedicated test');
    await runSlice({ all, captureCatalog, target: captureCatalog ? 'persistence' : target, filter: captureCatalog ? 'capture_persistence_catalog' : all ? '' : filter });
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
