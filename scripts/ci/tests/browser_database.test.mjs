import assert from 'node:assert/strict';
import test from 'node:test';
import { browserDatabasePool } from '../browser-database.mjs';

test('browser runtimes clone one prepared template and always drop private databases', async () => {
  const calls = [];
  let provisions = 0;
  const pool = await browserDatabasePool('postgres://medtracker:medtracker_password@127.0.0.1:54321/medtracker_loco', async () => { provisions++; }, { project: 'mtloco-http-00000000-0000-4000-8000-000000000000', timings: [] }, async (name, variables) => { calls.push({ name, variables }); });
  const databases = [];
  for (let index = 0; index < 2; index++) await assert.rejects(pool(async (url, provision, ownership) => {
    await provision();
    databases.push(new URL(url).pathname);
    assert.equal(ownership.templatePrepared, true);
    assert.equal(ownership.runtimeRole, `${ownership.databaseName}_runtime`);
    throw new Error('test failed');
  }), /test failed/);
  assert.equal(provisions, 1);
  assert.equal(new Set(databases).size, 2);
  assert.equal(calls.filter(call => call.name === 'browser-care:migrate').length, 0);
  assert.equal(calls.filter(call => call.name === 'browser-care:seed').length, 1);
  assert.equal(calls.filter(call => call.name === 'browser-care:clone').length, 2);
  assert.equal(calls.filter(call => call.name === 'browser-care:drop').length, 2);
});

test('each browser runtime receives its own valid object-storage bucket', async () => {
  const buckets = [];
  const pool = await browserDatabasePool('postgres://medtracker:medtracker_password@127.0.0.1:54321/medtracker_loco', async () => {}, {
    project: 'mtloco-http-00000000-0000-4000-8000-000000000000', templatePrepared: true,
    avatarStorage: async () => ({ ACTIVE_STORAGE_S3_ENDPOINT: 'http://127.0.0.1:9000' })
  }, async (name, variables) => {
    if (name === 'browser-care:storage-bucket') buckets.push(variables.find(value => value.startsWith('STORAGE_BUCKET=')).split('=')[1]);
  });
  for (let index = 0; index < 2; index++) await pool(async (_url, _provision, ownership) => {
    const storage = await ownership.avatarStorage();
    assert.match(storage.ACTIVE_STORAGE_S3_BUCKET, /^avatar-browser-[a-f0-9]{32}$/);
    assert.deepEqual(await ownership.avatarStorage(), storage);
  });
  assert.equal(buckets.length, 2);
  assert.equal(new Set(buckets).size, 2);
});
