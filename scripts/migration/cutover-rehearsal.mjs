import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { withOwnedDatabase } from './foundation-database.mjs';
import { stopOwnedProcess } from './process-cleanup.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));

export function safeCutoverEnvironment(environment) {
  const safeEnvironment = { ...environment };
  for (const name of Object.keys(safeEnvironment)) {
    if (/^PG/.test(name) || ['DATABASE_URL', 'COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH'].includes(name)) delete safeEnvironment[name];
  }
  return safeEnvironment;
}

async function execute(command, args, environment = process.env) {
  const safeEnvironment = safeCutoverEnvironment(environment);
  const child = spawn(command, args, { cwd: root, env: safeEnvironment, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  let errors = '';
  child.stdout.on('data', chunk => { output += chunk; });
  child.stderr.on('data', chunk => { errors += chunk; });
  let failed = false;
  let timer;
  try {
    await new Promise((resolve, reject) => {
      timer = setTimeout(() => reject(new Error(`${command} exceeded 10 minutes: ${output}\n${errors}`)), 600000);
      child.once('error', reject);
      child.once('close', code => code === 0 ? resolve() : reject(new Error(`${command} exited ${code}: ${output}\n${errors}`)));
    });
    return output.trim();
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    clearTimeout(timer);
    try { await stopOwnedProcess(child); } catch (error) {
      if (!failed) throw error;
      process.stderr.write(`Cutover child cleanup failed: ${error.message}\n`);
    }
  }
}

function validateOwned(databaseUrl, ownership) {
  const url = new URL(databaseUrl);
  assert.equal(url.protocol, 'postgres:', 'Rehearsal requires owned PostgreSQL');
  assert.equal(url.hostname, '127.0.0.1', 'Rehearsal requires a loopback endpoint');
  assert.equal(url.pathname, '/medtracker_loco', 'Rehearsal requires the owned foundation database');
  assert.equal(url.username, 'medtracker', 'Rehearsal requires synthetic database credentials');
  assert.equal(url.password, 'medtracker_password', 'Rehearsal requires synthetic database credentials');
  assert.match(url.port, /^\d+$/, 'Rehearsal requires an explicit owned database port');
  assert.equal(url.search, '', 'Rehearsal rejects PostgreSQL connection overrides');
  assert.equal(url.hash, '', 'Rehearsal rejects PostgreSQL connection overrides');
  assert.match(ownership.project, /^mtloco-http-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/, 'Rehearsal requires the owned Compose project');
  const reference = new URL(url);
  reference.pathname = '/medtracker_reference';
  return { maintenance: url.href, reference: reference.href };
}

async function verifyPostgres18(database, maintenance) {
  for (const command of ['psql', 'pg_dump', 'pg_dumpall', 'pg_restore']) {
    const version = await database(command, ['--version']);
    if (!/\(PostgreSQL\) 18(?:\.|$)/.test(version)) throw new Error(`PostgreSQL 18 client required for ${command}: ${version}`);
  }
  const serverVersion = await database('psql', [maintenance, '-X', '-v', 'ON_ERROR_STOP=1', '-Atqc', 'SHOW server_version_num']);
  if (!/^18\d{4}$/.test(String(serverVersion).trim())) throw new Error(`PostgreSQL 18 server required: ${serverVersion}`);
}

export async function rehearseCutover({ withDatabase = withOwnedDatabase, runTask, runCommand = execute, directory = () => mkdtemp(join(tmpdir(), 'medtracker-cutover-')), environment = process.env } = {}) {
  const rollbackImage = environment.RAILS_ROLLBACK_IMAGE;
  if (!/^sha256:[a-f0-9]{64}$/.test(rollbackImage ?? '')) throw new Error('Rehearsal requires an immutable Rails rollback image ID');
  const task = runTask ?? ((name, variables) => execute('task', [name, ...Object.entries(variables).map(([key, value]) => `${key}=${value}`)]));
  return withDatabase(async (databaseUrl, provision, ownership) => {
    const { maintenance, reference } = validateOwned(databaseUrl, ownership);
    const variables = { CARE_DATABASE_URL: reference, FOUNDATION_PROJECT: ownership.project };
    const work = await directory();
    const snapshot = join(work, 'pre-loco.dump');
    const database = async (name, args) => runCommand(name, args);
    const assertExclusive = async () => {
      const count = await database('psql', [maintenance, '-X', '-v', 'ON_ERROR_STOP=1', '-Atqc', "SELECT count(*) FROM pg_stat_activity WHERE datname='medtracker_reference' AND pid<>pg_backend_pid() AND backend_type='client backend'"]);
      if (String(count).trim() !== '0') throw new Error(`Cutover requires zero active Rails/Loco database clients; observed ${count}`);
    };
    const state = () => database('pg_dump', ['--dbname', reference, '--format=plain', '--restrict-key=MedTrackerRollbackComparison1']);
    const roles = () => database('pg_dumpall', ['--dbname', maintenance, '--globals-only', '--restrict-key=MedTrackerRollbackComparison1']);
    try {
      await verifyPostgres18(database, maintenance);
      await provision();
      await task('browser-care:seed', variables);
      await database('psql', [reference, '-X', '-v', 'ON_ERROR_STOP=1', '-c', "INSERT INTO public.versions (item_type,item_id,event,household_id,created_at) VALUES ('Account',71001,'cutover_fixture/preserved',72001,now())"]);
      await assertExclusive();
      const before = createHash('sha256').update(await state()).digest('hex');
      const beforeRoles = createHash('sha256').update(await roles()).digest('hex');
      await database('pg_dump', ['--dbname', reference, '--format=custom', '--file', snapshot]);
      const saved = await readFile(snapshot);
      if (saved.length === 0) throw new Error('Pre-Loco snapshot is empty');
      const snapshotSha256 = createHash('sha256').update(saved).digest('hex');
      await task('browser-care:migrate', { ...variables, MIGRATION_DATABASE_URL: reference, LOCO_ENV: 'test' });
      await task('release:cutover:loco-journey', { ...variables, CUTOVER_DATABASE_URL: reference, MEDTRACKER_OWNED_DATABASE: '1' });
      await assertExclusive();
      await database('psql', [maintenance, '-X', '-v', 'ON_ERROR_STOP=1', '-c', 'DROP DATABASE medtracker_reference']);
      await database('psql', [maintenance, '-X', '-v', 'ON_ERROR_STOP=1', '-c', 'CREATE DATABASE medtracker_reference']);
      await database('pg_restore', ['--dbname', reference, '--exit-on-error', '--single-transaction', snapshot]);
      const restored = createHash('sha256').update(await state()).digest('hex');
      if (before !== restored) throw new Error('Restored Rails state differs from the saved pre-Loco dataset');
      const restoredRoles = createHash('sha256').update(await roles()).digest('hex');
      if (beforeRoles !== restoredRoles) throw new Error('PostgreSQL roles or global grants differ from the saved pre-Loco state');
      await task('rails:rollback:journey', { ...variables, RAILS_ROLLBACK_IMAGE: rollbackImage });
      return { before, restored, beforeRoles, restoredRoles, snapshotSha256 };
    } finally {
      await rm(work, { recursive: true, force: true });
    }
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const evidence = await rehearseCutover();
    process.stdout.write(`Saved-state rollback rehearsal passed: snapshot sha256 ${evidence.snapshotSha256}; restored dataset sha256 ${evidence.restored}\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
