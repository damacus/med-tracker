import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { withOwnedDatabase } from './foundation-database.mjs';
import { stopOwnedProcess } from './process-cleanup.mjs';

async function runCargo(args, databaseUrl, inheritedEnvironment, signal) {
  signal.throwIfAborted();
  const environment = { ...inheritedEnvironment, DATABASE_URL: databaseUrl, MEDTRACKER_OWNED_DATABASE: '1' };
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
    await withOwnedDatabase(async (url, provision) => {
      if (all || ['persistence', 'tenant_access', 'care_doses'].includes(target)) await provision();
      await runCargo(args, url, inheritedEnvironment, controller.signal);
    }, undefined, inheritedEnvironment);
  } finally {
    process.off('SIGTERM', onTerm);
    process.off('SIGINT', onInt);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    if (process.argv.slice(2).some(argument => !['--all', '--capture-catalog'].includes(argument))) throw new Error('Only --all and --capture-catalog are supported; TARGET and FILTER are Task variables');
    const all = process.argv.includes('--all');
    const captureCatalog = process.argv.includes('--capture-catalog');
    if (all && captureCatalog) throw new Error('Catalog capture requires its dedicated test');
    await runSlice({ all, captureCatalog, target: captureCatalog ? 'persistence' : process.env.SLICE_TARGET, filter: captureCatalog ? 'capture_persistence_catalog' : all ? '' : process.env.SLICE_FILTER });
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
