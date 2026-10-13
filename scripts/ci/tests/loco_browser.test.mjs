import assert from 'node:assert/strict';
import test from 'node:test';
import { runBrowser } from '../loco-browser.mjs';

test('browser stage owns one service across workers and cleans it after failure', async () => {
  let starts = 0;
  let stops = 0;
  const calls = [];
  const owner = async (callback, _run, _env, options) => {
    starts++;
    assert.equal(options.storage, false);
    try {
      return await callback('postgres://medtracker:medtracker_password@127.0.0.1:54321/medtracker_loco', async () => {}, {
        project: 'mtloco-http-00000000-0000-4000-8000-000000000000', timings: [], storageEnvironment: {}
      });
    } finally { stops++; }
  };
  const run = async (command, args, options) => {
    calls.push({ command, args });
    if (args.includes('--list')) return JSON.stringify({ suites: [{ file: 'runtime-isolation.spec.mjs' }] });
    if (command === 'npx') {
      const session = JSON.parse(options.env.LOCO_BROWSER_DATABASE);
      assert.equal(session.project, 'mtloco-http-00000000-0000-4000-8000-000000000000');
      assert.equal(session.templatePrepared, true);
      throw new Error('browser failed');
    }
    return '';
  };
  await assert.rejects(runBrowser([], owner, run), /browser failed/);
  assert.equal(starts, 1);
  assert.equal(stops, 1);
  assert.equal(calls.filter(call => call.args[0] === 'browser-care:seed').length, 1);
  assert.equal(calls.filter(call => call.args[0] === 'browser-care:migrate').length, 0);
});

test('NHS import suite selects owned scanner storage before browser workers start', async () => {
  let storageCalls = 0;
  const owner = async (callback, _run, _env, options) => {
    assert.equal(options.storage, false);
    return callback('postgres://medtracker:medtracker_password@127.0.0.1:54321/medtracker_loco', async () => {}, {
      project: 'mtloco-http-00000000-0000-4000-8000-000000000000', timings: [], storageEnvironment: {},
      storage: async () => { storageCalls++; return { S3_ENDPOINT: 'http://127.0.0.1:9000' }; }
    });
  };
  const run = async (command, args, options) => {
    if (args.includes('--list')) return JSON.stringify({ suites: [{ file: 'nhs-dmd-import.spec.mjs' }] });
    if (command === 'npx') {
      const session = JSON.parse(options.env.LOCO_BROWSER_DATABASE);
      assert.equal(session.scannerEnvironment.S3_ENDPOINT, 'http://127.0.0.1:9000');
    }
    return '';
  };
  await runBrowser([], owner, run);
  assert.equal(storageCalls, 1);
});
