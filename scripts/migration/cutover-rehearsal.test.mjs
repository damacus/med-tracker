import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { rehearseCutover, safeCutoverEnvironment } from './cutover-rehearsal.mjs';

const owned = 'postgres://medtracker:medtracker_password@127.0.0.1:54321/medtracker_loco';
const project = 'mtloco-http-00000000-0000-4000-8000-000000000000';
const image = `sha256:${'a'.repeat(64)}`;
const root = fileURLToPath(new URL('../../', import.meta.url));

async function fixture(callback) {
  const calls = [];
  const directory = await mkdtemp(join(tmpdir(), 'medtracker-cutover-test-'));
  const dependencies = {
    environment: { ...process.env, RAILS_ROLLBACK_IMAGE: image },
    withDatabase: async run => run(owned, async () => calls.push('provision'), { project }),
    directory: async () => directory,
    runTask: async (name, variables) => {
      calls.push(name);
      assert.equal(variables.CARE_DATABASE_URL, owned.replace('/medtracker_loco', '/medtracker_reference'));
      assert.equal(variables.FOUNDATION_PROJECT, project);
      if (name === 'browser-care:migrate') assert.deepEqual({ MIGRATION_DATABASE_URL: variables.MIGRATION_DATABASE_URL, LOCO_ENV: variables.LOCO_ENV }, { MIGRATION_DATABASE_URL: owned.replace('/medtracker_loco', '/medtracker_reference'), LOCO_ENV: 'test' });
      if (name === 'release:cutover:loco-journey') assert.deepEqual({ CUTOVER_DATABASE_URL: variables.CUTOVER_DATABASE_URL, MEDTRACKER_OWNED_DATABASE: variables.MEDTRACKER_OWNED_DATABASE }, { CUTOVER_DATABASE_URL: owned.replace('/medtracker_loco', '/medtracker_reference'), MEDTRACKER_OWNED_DATABASE: '1' });
      if (name === 'rails:rollback:journey') assert.equal(variables.RAILS_ROLLBACK_IMAGE, image);
      return '';
    },
    runCommand: async (name, args) => {
      calls.push(name);
      if (args.includes('--version')) return `${name} (PostgreSQL) 18.6`;
      if (name === 'psql' && args.includes('SHOW server_version_num')) return '180006';
      if (name === 'pg_dump' && args.includes('--format=custom')) await writeFile(args.at(-1), 'saved-rails-state');
      if (name === 'pg_dump') {
        const key = args.find(arg => arg.startsWith('--restrict-key='))?.split('=')[1] ?? Math.random().toString(36);
        return `\\restrict ${key}\nsaved-state-identity\n\\unrestrict ${key}`;
      }
      if (name === 'pg_dumpall') {
        const key = args.find(arg => arg.startsWith('--restrict-key='))?.split('=')[1] ?? Math.random().toString(36);
        return `\\restrict ${key}\nsaved-cluster-roles\n\\unrestrict ${key}`;
      }
      if (name === 'psql' && args.some(arg => arg.includes('SELECT count(*)'))) return '0';
      return '';
    },
  };
  try { await callback(dependencies, calls); } finally { await rm(directory, { recursive: true, force: true }); }
}

test('rollback uses an independent pre-Loco dump, stops writers and checks restored Rails state', async () => {
  await fixture(async (dependencies, calls) => {
    const evidence = await rehearseCutover(dependencies);
    assert.deepEqual(calls, [
      'psql', 'pg_dump', 'pg_dumpall', 'pg_restore', 'psql',
      'provision', 'browser-care:seed', 'psql', 'psql', 'pg_dump', 'pg_dumpall', 'pg_dump', 'browser-care:migrate',
      'release:cutover:loco-journey', 'psql', 'psql', 'psql', 'pg_restore', 'pg_dump', 'pg_dumpall', 'rails:rollback:journey',
    ]);
    assert.equal(evidence.before, evidence.restored);
    assert.equal(evidence.beforeRoles, evidence.restoredRoles);
    assert.match(evidence.snapshotSha256, /^[a-f0-9]{64}$/);
    assert.equal(calls.includes('rails:rollback:journey'), true);
  });
});

test('rollback cannot proceed if the Loco writer phase fails', async () => {
  await fixture(async (dependencies, calls) => {
    const original = dependencies.runTask;
    dependencies.runTask = async (name, variables) => {
      if (name === 'release:cutover:loco-journey') throw new Error('Loco journey missing');
      return original(name, variables);
    };
    await assert.rejects(rehearseCutover(dependencies), /Loco journey missing/);
    assert.equal(calls.filter(call => call === 'pg_restore').length, 1);
    assert.equal(calls.includes('rails:rollback:journey'), false);
  });
});

test('rollback rejects a restored dataset that differs from the saved state', async () => {
  await fixture(async (dependencies) => {
    const original = dependencies.runCommand;
    let probes = 0;
    dependencies.runCommand = async (name, args) => {
      if (name === 'pg_dump' && args.includes('--format=plain') && ++probes === 2) return 'changed-state';
      return original(name, args);
    };
    await assert.rejects(rehearseCutover(dependencies), /Restored Rails state differs/);
  });
});

test('rollback refuses an active writer before dropping its database', async () => {
  await fixture(async (dependencies, calls) => {
    const original = dependencies.runCommand;
    let checks = 0;
    dependencies.runCommand = async (name, args) => {
      if (name === 'psql' && args.some(arg => arg.includes('SELECT count(*)')) && ++checks === 2) return '1';
      return original(name, args);
    };
    await assert.rejects(rehearseCutover(dependencies), /zero active Rails\/Loco database clients/);
    assert.equal(calls.filter(call => call === 'pg_restore').length, 1);
    assert.equal(calls.some(call => call === 'rails:rollback:journey'), false);
  });
});

test('rollback rejects a database outside the owned loopback target', async () => {
  await fixture(async (dependencies, calls) => {
    dependencies.withDatabase = async run => run('postgres://medtracker:secret@db.example.test/medtracker_loco', async () => calls.push('provision'), { project });
    await assert.rejects(rehearseCutover(dependencies), /loopback endpoint/);
    assert.deepEqual(calls, []);
  });
});

test('client and server PostgreSQL 18 are verified before fixture provisioning', async () => {
  await fixture(async (dependencies, calls) => {
    await rehearseCutover(dependencies);
    assert.deepEqual(calls.slice(0, 5), ['psql', 'pg_dump', 'pg_dumpall', 'pg_restore', 'psql']);
    assert.equal(calls[5], 'provision');
  });
});

test('a PostgreSQL 17 client fails before fixture provisioning', async () => {
  await fixture(async (dependencies, calls) => {
    const original = dependencies.runCommand;
    dependencies.runCommand = (name, args) => name === 'pg_restore' && args.includes('--version')
      ? Promise.resolve('pg_restore (PostgreSQL) 17.7') : original(name, args);
    await assert.rejects(rehearseCutover(dependencies), /PostgreSQL 18 client required/);
    assert.equal(calls.includes('provision'), false);
  });
});

test('a PostgreSQL 17 server fails before fixture provisioning', async () => {
  await fixture(async (dependencies, calls) => {
    const original = dependencies.runCommand;
    dependencies.runCommand = (name, args) => name === 'psql' && args.includes('SHOW server_version_num')
      ? Promise.resolve('170007') : original(name, args);
    await assert.rejects(rehearseCutover(dependencies), /PostgreSQL 18 server required/);
    assert.equal(calls.includes('provision'), false);
  });
});

test('only a generated project and synthetic database credentials are accepted', async () => {
  await fixture(async (dependencies, calls) => {
    dependencies.withDatabase = async run => run(owned, async () => calls.push('provision'), { project: 'mtloco-http-prod' });
    await assert.rejects(rehearseCutover(dependencies), /owned Compose project/);
    dependencies.withDatabase = async run => run('postgres://other:secret@127.0.0.1:54321/medtracker_loco', async () => calls.push('provision'), { project });
    await assert.rejects(rehearseCutover(dependencies), /synthetic database credentials/);
    dependencies.withDatabase = async run => run(`${owned}?host=db.example.test`, async () => calls.push('provision'), { project });
    await assert.rejects(rehearseCutover(dependencies), /connection overrides/);
    assert.deepEqual(calls, []);
  });
});

test('PostgreSQL client options are removed from inherited process environment', () => {
  const environment = safeCutoverEnvironment({ PATH: '/pg18/bin:/usr/bin', PGOPTIONS: '-c search_path=public', PGSSLMODE: 'require', PGSERVICE: 'other', PGCLIENTENCODING: 'LATIN1', DATABASE_URL: 'unowned', COMPOSE_PROJECT_NAME: 'other' });
  assert.equal(environment.PATH, '/pg18/bin:/usr/bin');
  for (const name of ['PGOPTIONS', 'PGSSLMODE', 'PGSERVICE', 'PGCLIENTENCODING', 'DATABASE_URL', 'COMPOSE_PROJECT_NAME']) assert.equal(environment[name], undefined);
});

test('missing or malformed Rails image identity rejects before owned database creation', async () => {
  await fixture(async (dependencies, calls) => {
    dependencies.environment = { ...process.env };
    delete dependencies.environment.RAILS_ROLLBACK_IMAGE;
    await assert.rejects(rehearseCutover(dependencies), /immutable Rails rollback image/);
    dependencies.environment.RAILS_ROLLBACK_IMAGE = 'med-tracker-web-test:latest';
    await assert.rejects(rehearseCutover(dependencies), /immutable Rails rollback image/);
    assert.deepEqual(calls, []);
  });
});

test('saved dump comparison is repeatable and ownership and privileges are restored', async () => {
  await fixture(async (dependencies) => {
    const commands = [];
    const original = dependencies.runCommand;
    dependencies.runCommand = async (name, args) => {
      commands.push({ name, args });
      return original(name, args);
    };
    await rehearseCutover(dependencies);
    const comparisons = commands.filter(call => call.name === 'pg_dump' && call.args.includes('--format=plain'));
    assert.equal(comparisons.length, 2);
    assert.equal(comparisons[0].args.find(arg => arg.startsWith('--restrict-key=')), comparisons[1].args.find(arg => arg.startsWith('--restrict-key=')));
    for (const call of comparisons) {
      assert.equal(call.args.includes('--no-owner'), false);
      assert.equal(call.args.includes('--no-privileges'), false);
    }
    const restore = commands.find(call => call.name === 'pg_restore' && call.args.includes('--dbname'));
    assert.equal(restore.args.includes('--no-owner'), false);
    assert.equal(restore.args.includes('--no-privileges'), false);
    const roleComparisons = commands.filter(call => call.name === 'pg_dumpall' && call.args.includes('--globals-only'));
    assert.equal(roleComparisons.length, 2);
    assert.equal(roleComparisons[0].args.find(arg => arg.startsWith('--restrict-key=')), roleComparisons[1].args.find(arg => arg.startsWith('--restrict-key=')));
  });
});

test('rollback rejects changed PostgreSQL role authority', async () => {
  await fixture(async (dependencies) => {
    const original = dependencies.runCommand;
    let probes = 0;
    dependencies.runCommand = async (name, args) => {
      if (name === 'pg_dumpall' && ++probes === 2) return 'changed-cluster-roles';
      return original(name, args);
    };
    await assert.rejects(rehearseCutover(dependencies), /roles or global grants differ/);
  });
});

test('restored-state Rails spec is outside normal Rails spec discovery', () => {
  assert.equal(existsSync(join(root, 'rails/spec/system/loco_rollback_restored_state_spec.rb')), false);
  assert.equal(existsSync(join(root, 'scripts/migration/rails_rollback_spec.rb')), true);
});
