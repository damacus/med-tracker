import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const ambientUrl = 'postgres://unowned.invalid:1/unowned';

async function withProcessFixture(exitCode, callback, taskName = 'slice:test', assignments = ['TARGET=persistence'], provisionExit = 0) {
  const directory = await mkdtemp(join(tmpdir(), 'medtracker-slice-runner-'));
  const statePath = join(directory, 'state.json');
  await writeFile(statePath, JSON.stringify({ resources: { unrelated: ['existing-volume'] }, calls: [], foreign_resources_touched: 0 }));
  const executable = `#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const statePath = process.env.SLICE_FIXTURE_STATE;
const state = JSON.parse(fs.readFileSync(statePath, 'utf8'));
const command = path.basename(process.argv[1]);
const args = process.argv.slice(2);
if (command === 'docker') {
  const project = args[args.indexOf('-p') + 1];
  if (!/^mtloco-http-[a-f0-9-]+$/.test(project)) throw new Error('Unowned project selected');
  const operation = ['up', 'port', 'down', 'exec'].find(value => args.includes(value));
  state.calls.push({ command, project, operation, args, docker_environment_removed: ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH'].every(name => process.env[name] === undefined) });
  if (operation === 'up') state.resources[project] = ['owned-volume'];
  if (operation === 'down') delete state.resources[project];
  fs.writeFileSync(statePath, JSON.stringify(state));
  if (operation === 'port') process.stdout.write(args.includes('rustfs') ? '127.0.0.1:54322\\n' : '127.0.0.1:54321\\n');
  if (operation === 'exec') process.exit(Number(process.env.SLICE_FIXTURE_PROVISION_EXIT));
} else {
  state.owned_endpoint_used = process.env.DATABASE_URL === 'postgres://medtracker:medtracker_password@127.0.0.1:54321/medtracker_loco';
  state.owned_storage_endpoint_used = process.env.ACTIVE_STORAGE_S3_ENDPOINT === 'http://127.0.0.1:54322';
  state.child_identity_key_synthetic = process.env.MEDTRACKER_SESSION_KEY === Buffer.alloc(64, 7).toString('base64');
  state.child_verification_key_synthetic = process.env.RAILS_SECRET_KEY_BASE === 'synthetic-slice-rails-verification-secret';
  state.child_compose_environment_removed = ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES'].every(name => process.env[name] === undefined);
  state.child_docker_environment_removed = ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH'].every(name => process.env[name] === undefined);
  if (process.env.DATABASE_URL === ${JSON.stringify(ambientUrl)}) state.foreign_resources_touched += 1;
  state.calls.push({ command, args });
  fs.writeFileSync(statePath, JSON.stringify(state));
  process.exit(Number(process.env.SLICE_FIXTURE_EXIT));
}
`;
  for (const command of ['docker', 'cargo']) await writeFile(join(directory, command), executable, { mode: 0o755 });
  try {
    const result = spawnSync('task', ['--taskfile', join(root, 'Taskfile.yml'), taskName, ...assignments], {
      cwd: root,
      env: { ...process.env, PATH: `${directory}:${process.env.PATH}`, DATABASE_URL: ambientUrl, MEDTRACKER_SESSION_KEY: 'ambient-identity-fixture-key-must-not-be-used', COMPOSE_FILE: '/unowned/compose.yaml', COMPOSE_PROJECT_NAME: 'unrelated', DOCKER_HOST: 'tcp://unowned.invalid:2376', DOCKER_CONTEXT: 'unrelated', DOCKER_CONFIG: '/unowned/docker-config', DOCKER_TLS_VERIFY: '1', DOCKER_CERT_PATH: '/unowned/certs', SLICE_FILTER: 'unrelated_filter', SLICE_FIXTURE_STATE: statePath, SLICE_FIXTURE_EXIT: String(exitCode), SLICE_FIXTURE_PROVISION_EXIT: String(provisionExit) },
      encoding: 'utf8',
      timeout: 15000,
    });
    assert.equal(result.error, undefined, result.error?.message);
    await callback(result, JSON.parse(await readFile(statePath, 'utf8')));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

test('slice_runner_ignores_ambient_database', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.equal(state.owned_endpoint_used, true, 'The slice child used the ambient database instead of an owned endpoint');
    assert.equal(state.child_verification_key_synthetic, true, 'Signup verification requires a synthetic Rails secret in the slice child');
    assert.equal(state.foreign_resources_touched, 0);
    assert.equal(state.child_compose_environment_removed, true);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'exec', 'exec', 'down']);
  });
});

test('fixture and Cargo subprocesses remove ambient Docker endpoint and credential settings', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.equal(state.child_docker_environment_removed, true, 'Cargo inherited ambient Docker settings');
    for (const call of state.calls.filter(call => call.command === 'docker')) {
      assert.equal(call.docker_environment_removed, true, `${call.operation} inherited ambient Docker settings`);
    }
  });
});

test('every public foundation database task pins the root Compose file and disables automatic env files', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    for (const call of state.calls.filter(call => call.command === 'docker')) {
      assert.equal(call.args[call.args.indexOf('-f') + 1], join(root, 'compose.yaml'), `${call.operation} did not select the authoritative Compose file`);
      assert.equal(call.args[call.args.indexOf('--env-file') + 1], '/dev/null', `${call.operation} could load an ambient env file`);
    }
  });
});

test('the complete Cargo phase retains unit, integration and doc tests without an ambient filter', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(state.calls.find(call => call.command === 'cargo').args, ['test', '--locked']);
    assert.equal(state.owned_endpoint_used, true);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:test-all', []);
});

test('the persistence baseline preserves the pinned administrative artifact bytes', async () => {
  const baseline = await readFile(join(root, 'migration/baseline.sql'));
  assert.equal(createHash('sha256').update(baseline).digest('hex'), 'f5043a97e60cea912a2f36dec79ee99c2d19e5a950916d1508f76d3e0b87966b');
});

test('failed slice execution cleans its owned database and preserves unrelated resources', async () => {
  await withProcessFixture(42, (result, state) => {
    assert.notEqual(result.status, 0);
    assert.deepEqual(state.resources.unrelated, ['existing-volume']);
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'exec', 'exec', 'down'], 'Failure cleanup did not run for the owned fixture');
    assert.deepEqual(Object.keys(state.resources), ['unrelated']);
    assert.equal(state.foreign_resources_touched, 0);
  });
});

test('failed administrative provisioning never starts Cargo and cleans only its owned resources', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0);
    assert.equal(state.calls.some(call => call.command === 'cargo'), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    assert.deepEqual(state.calls.map(call => call.operation), ['up', 'port', 'exec', 'down']);
  }, 'slice:test', ['TARGET=persistence'], 42);
});

test('administrative restore uses psql without startup files, stops on error and restores atomically', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    const calls = state.calls.filter(call => call.operation === 'exec');
    assert.equal(calls.length, 2);
    for (const call of calls) {
      assert.ok(call.args.includes('psql'));
      assert.ok(call.args.includes('-X'));
      assert.ok(call.args.includes('ON_ERROR_STOP=1'));
    }
    assert.ok(calls[1].args.includes('--single-transaction'));
    assert.equal(calls[1].args[calls[1].args.indexOf('-d') + 1], 'medtracker_reference');
  });
});

test('catalog capture uses its explicit ignored test and does not inherit ambient filters', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(state.calls.find(call => call.command === 'cargo').args,
      ['test', '--locked', '--test', 'persistence', 'capture_persistence_catalog', '--', '--ignored']);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:catalog', []);
});


test('owned application tests use a synthetic identity key instead of the inherited key', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0);
    assert.equal(state.child_identity_key_synthetic, true);
  });
});

test('care API and complete runs use owned storage and clean it after test failures', async () => {
  for (const [taskName, assignments] of [['slice:test', ['TARGET=care_api']], ['slice:test-all', []]]) {
    await withProcessFixture(42, (result, state) => {
      assert.notEqual(result.status, 0);
      assert.equal(state.owned_endpoint_used, true);
      assert.equal(state.owned_storage_endpoint_used, true);
      const calls = state.calls.filter(call => call.command === 'docker');
      assert.deepEqual(calls.map(call => call.operation), ['up', 'port', 'up', 'port', 'exec', 'exec', 'down']);
      assert.ok(calls.every(call => call.project === calls[0].project));
      assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    }, taskName, assignments);
  }
});
