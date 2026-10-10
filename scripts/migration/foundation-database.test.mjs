import assert from 'node:assert/strict';
import test from 'node:test';
import { withOwnedDatabase } from './foundation-database.mjs';

test('owned cleanup permits a bounded ninety-second Compose shutdown without extending other phases', async () => {
  const deadlines = new Map();
  let removed = false;
  await withOwnedDatabase(async () => {}, async (args, options) => {
    deadlines.set(args[0], options.timeout);
    if (args[0] === 'foundation:db-port') return '127.0.0.1:54321';
    if (args[0] === 'foundation:db-down') {
      if (options.timeout < 90000) throw new Error('Compose shutdown exceeded its deadline before resource removal');
      assert.ok(options.timeout <= 120000, 'Cleanup must remain bounded');
      removed = true;
    }
    return '';
  }, {});
  assert.equal(removed, true, 'The owned resource must be removed after a simulated 90-second shutdown');
  assert.equal(deadlines.get('foundation:db-up'), 60000);
  assert.equal(deadlines.get('foundation:db-port'), 10000);
  assert.equal(deadlines.get('foundation:db-down'), 120000);
});

test('HTTP fixtures use unique projects and explicit disposable loopback databases', async () => {
  const projects = [];
  for (let index = 0; index < 2; index += 1) {
    const calls = [];
    await withOwnedDatabase(async url => {
      assert.equal(url, 'postgres://medtracker:medtracker_password@127.0.0.1:54321/medtracker_loco');
    }, (args, options) => {
      calls.push(args);
      assert.equal(options.env.COMPOSE_FILE, undefined);
      assert.equal(options.env.COMPOSE_PROJECT_NAME, undefined);
      assert.equal(options.env.DATABASE_URL, undefined);
      return args[0] === 'foundation:db-port' ? '127.0.0.1:54321\n' : '';
    }, { COMPOSE_FILE: '/unowned/compose.yaml', COMPOSE_PROJECT_NAME: 'unowned-project', DATABASE_URL: 'postgres://unowned' });
    assert.deepEqual(calls.map(args => args[0]), ['foundation:db-up', 'foundation:db-port', 'foundation:db-down']);
    const project = calls[0][1];
    assert.match(project, /^FOUNDATION_PROJECT=mtloco-http-[a-f0-9-]+$/);
    assert.ok(calls.every(args => args[1] === project));
    projects.push(project);
  }
  assert.notEqual(projects[0], projects[1]);
});

test('failed database setup still cleans only its attempted project', async () => {
  const calls = [];
  await assert.rejects(withOwnedDatabase(async () => assert.fail('Application must not start'), args => {
    calls.push(args);
    if (args[0] === 'foundation:db-up') throw new Error('Fixture setup failed');
    return '';
  }), /Fixture setup failed/);
  assert.deepEqual(calls.map(args => args[0]), ['foundation:db-up', 'foundation:db-down']);
  assert.equal(calls[0][1], calls[1][1]);
});

test('unusable published endpoints are rejected and the owned database is cleaned', async () => {
  const calls = [];
  await assert.rejects(withOwnedDatabase(async () => assert.fail('Application must not start'), args => {
    calls.push(args);
    return args[0] === 'foundation:db-port' ? '0.0.0.0:54321' : '';
  }), /Invalid owned PostgreSQL endpoint/);
  assert.equal(calls.at(-1)[0], 'foundation:db-down');
});

test('application failure survives database cleanup failure', async () => {
  await assert.rejects(withOwnedDatabase(async () => { throw new Error('Original application failure'); }, args => {
    if (args[0] === 'foundation:db-down') throw new Error('Cleanup failure');
    return args[0] === 'foundation:db-port' ? '127.0.0.1:54321' : '';
  }), /Original application failure/);
});

test('storage fixtures share only their owned project and supply an isolated loopback endpoint', async () => {
  const calls = [];
  await withOwnedDatabase(async (_url, _provision, ownership) => {
    assert.equal(ownership.storageEnvironment?.ACTIVE_STORAGE_S3_ENDPOINT, 'http://127.0.0.1:54322');
    assert.equal(ownership.storageEnvironment?.ACTIVE_STORAGE_S3_BUCKET, 'medtracker-fixture');
    assert.equal(ownership.storageEnvironment?.ACTIVE_STORAGE_S3_ACCESS_KEY_ID, 'storage-smoke-access');
  }, args => {
    calls.push(args);
    if (args[0] === 'foundation:db-port') return '127.0.0.1:54321';
    if (args[0] === 'foundation:storage-port') return '127.0.0.1:54322';
    return '';
  }, { ACTIVE_STORAGE_S3_ENDPOINT: 'https://unowned.invalid' }, { storage: true });
  assert.deepEqual(calls.map(args => args[0]), ['foundation:db-up', 'foundation:db-port', 'foundation:storage-up', 'foundation:storage-port', 'foundation:db-down']);
  assert.ok(calls.every(args => args[1] === calls[0][1]));
});

test('storage fixture rejects non-loopback endpoints before starting the application', async () => {
  const calls = [];
  await assert.rejects(withOwnedDatabase(async () => assert.fail('Application must not start'), args => {
    calls.push(args);
    if (args[0] === 'foundation:db-port') return '127.0.0.1:54321';
    if (args[0] === 'foundation:storage-port') return '0.0.0.0:9000';
    return '';
  }, {}, { storage: true }), /Invalid owned storage endpoint/);
  assert.equal(calls.at(-1)[0], 'foundation:db-down');
});

test('failed storage setup cleans the attempted project without changing the original error', async () => {
  const calls = [];
  await assert.rejects(withOwnedDatabase(async () => assert.fail('Application must not start'), args => {
    calls.push(args);
    if (args[0] === 'foundation:db-port') return '127.0.0.1:54321';
    if (args[0] === 'foundation:storage-up') throw new Error('Owned storage unavailable');
    return '';
  }, {}, { storage: true }), /Owned storage unavailable/);
  assert.equal(calls.at(-1)[0], 'foundation:db-down');
  assert.ok(calls.every(args => args[1] === calls[0][1]));
});
