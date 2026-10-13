import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const ambientUrl = 'postgres://unowned.invalid:1/unowned';

async function withProcessFixture(exitCode, callback, taskName = 'slice:test', assignments = ['TARGET=persistence'], provisionExit = 0, extraEnvironment = {}) {
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
  state.owned_storage_used = process.env.S3_ENDPOINT === 'http://127.0.0.1:54322' && process.env.S3_BUCKET === 'scanner-fixture' && process.env.AWS_ACCESS_KEY_ID === 'storage-smoke-access' && process.env.MEDTRACKER_OWNED_STORAGE === '1';
  state.owned_storage_endpoint_used = process.env.ACTIVE_STORAGE_S3_ENDPOINT === 'http://127.0.0.1:54322';
  state.child_identity_key_synthetic = process.env.MEDTRACKER_SESSION_KEY === Buffer.alloc(64, 7).toString('base64');
  state.child_verification_key_synthetic = process.env.RAILS_SECRET_KEY_BASE === 'synthetic-slice-rails-verification-secret';
  state.child_compose_environment_removed = ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES'].every(name => process.env[name] === undefined);
  state.child_docker_environment_removed = ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH'].every(name => process.env[name] === undefined);
  if (process.env.DATABASE_URL === ${JSON.stringify(ambientUrl)}) state.foreign_resources_touched += 1;
  state.calls.push({ command, args, coverage_target_dir: process.env.CARGO_TARGET_DIR, coverage_build_dir: process.env.CARGO_BUILD_BUILD_DIR, coverage_profile_override: process.env.LLVM_PROFILE_FILE, llvm_cov_target_override: process.env.CARGO_LLVM_COV_TARGET_DIR, llvm_cov_build_override: process.env.CARGO_LLVM_COV_BUILD_DIR, llvm_cov_setup_override: process.env.CARGO_LLVM_COV_SETUP, llvm_cov_deprecated_flags_override: process.env.CARGO_LLVM_COV_FLAGS, llvm_profdata_deprecated_flags_override: process.env.CARGO_LLVM_PROFDATA_FLAGS, llvm_cov_tool_override: process.env.LLVM_COV, llvm_profdata_tool_override: process.env.LLVM_PROFDATA, llvm_cov_flags_override: process.env.LLVM_COV_FLAGS, llvm_profdata_flags_override: process.env.LLVM_PROFDATA_FLAGS, llvm_profile_name_override: process.env.LLVM_PROFILE_FILE_NAME });
  fs.writeFileSync(statePath, JSON.stringify(state));
  if (command === 'cargo' && args.includes('--version')) {
    if (process.env.SLICE_FIXTURE_MISSING_COVERAGE === '1') process.exit(101);
    process.stdout.write('cargo-llvm-cov ' + (process.env.SLICE_FIXTURE_COV_VERSION || '0.9.1') + '\\n');
  }
  if (command === 'cargo' && args.includes('--no-report') && args.includes('--no-clean')) {
    process.stderr.write('error: --no-report may not be used together with --no-clean\\n');
    process.exit(2);
  }
  if (command === 'rustup' && args.includes('--installed')) process.stdout.write(process.env.SLICE_FIXTURE_MISSING_LLVM_TOOLS === '1' ? 'rustfmt-aarch64-apple-darwin\\n' : 'llvm-tools-aarch64-apple-darwin\\n');
  if (command === 'cargo' && args.includes('report') && args.includes('--output-path')) {
    const reportPath = args[args.indexOf('--output-path') + 1];
    fs.mkdirSync(path.dirname(reportPath), { recursive: true });
    fs.writeFileSync(reportPath, 'TN:\\nSF:src/lib.rs\\nDA:1,1\\nend_of_record\\n');
  }
  process.exit(command === 'cargo' && (!process.env.SLICE_FIXTURE_FAIL_MATCH || args.join(' ').includes(process.env.SLICE_FIXTURE_FAIL_MATCH)) ? Number(process.env.SLICE_FIXTURE_EXIT) : 0);
}
`;
  for (const command of ['docker', 'cargo', 'curl', 'rustup']) await writeFile(join(directory, command), executable, { mode: 0o755 });
  try {
    const taskAssignments = typeof assignments === 'function' ? assignments(directory) : assignments;
    const result = spawnSync('task', ['--taskfile', join(root, 'Taskfile.yml'), taskName, ...taskAssignments], {
      cwd: root,
      env: { ...process.env, PATH: `${directory}:${process.env.PATH}`, DATABASE_URL: ambientUrl, MEDTRACKER_SESSION_KEY: 'ambient-identity-fixture-key-must-not-be-used', COMPOSE_FILE: '/unowned/compose.yaml', COMPOSE_PROJECT_NAME: 'unrelated', DOCKER_HOST: 'tcp://unowned.invalid:2376', DOCKER_CONTEXT: 'unrelated', DOCKER_CONFIG: '/unowned/docker-config', DOCKER_TLS_VERIFY: '1', DOCKER_CERT_PATH: '/unowned/certs', SLICE_FILTER: 'unrelated_filter', SLICE_FIXTURE_STATE: statePath, SLICE_FIXTURE_EXIT: String(exitCode), SLICE_FIXTURE_PROVISION_EXIT: String(provisionExit), CARGO_TARGET_DIR: '/unowned/target', CARGO_BUILD_BUILD_DIR: '/unowned/intermediates', LLVM_PROFILE_FILE: '/unowned/profile.profraw', CARGO_LLVM_COV_TARGET_DIR: '/unowned/llvm-cov-target', CARGO_LLVM_COV_BUILD_DIR: '/unowned/llvm-cov-build', CARGO_LLVM_COV_SETUP: 'skip', CARGO_LLVM_COV_FLAGS: '--unowned', CARGO_LLVM_PROFDATA_FLAGS: '--unowned', LLVM_COV: '/unowned/llvm-cov', LLVM_PROFDATA: '/unowned/llvm-profdata', LLVM_COV_FLAGS: '--unowned', LLVM_PROFDATA_FLAGS: '--unowned', LLVM_PROFILE_FILE_NAME: 'unowned.profraw', ...extraEnvironment },
      encoding: 'utf8',
      timeout: 15000,
    });
    assert.equal(result.error, undefined, result.error?.message);
    await callback(result, JSON.parse(await readFile(statePath, 'utf8')), directory);
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
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'exec', 'down']);
  });
});

test('scanner import tests provision owned object storage and clean it on failure', async () => {
  await withProcessFixture(42, (result,state) => {
    assert.notEqual(result.status,0);
    assert.equal(state.owned_storage_used,true,`Import tests require real isolated storage, without silent skipping: ${JSON.stringify(state.calls)} ${result.stderr}`);
    assert.deepEqual(state.resources,{unrelated:['existing-volume']});
  },'slice:test',['TARGET=care_api','FILTER=nhs_dmd']);
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
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'exec', 'down'], 'Failure cleanup did not run for the owned fixture');
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

test('provisioning creates only an empty database without importing application SQL', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    const calls = state.calls.filter(call => call.operation === 'exec');
    assert.equal(calls.length, 1);
    for (const call of calls) {
      assert.ok(call.args.includes('psql'));
      assert.ok(call.args.includes('-X'));
      assert.ok(call.args.includes('ON_ERROR_STOP=1'));
    }
    assert.equal(calls[0].args[calls[0].args.indexOf('-c') + 1], 'CREATE DATABASE medtracker_reference');
  });
});


test('owned application tests use a synthetic identity key instead of the inherited key', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0);
    assert.equal(state.child_identity_key_synthetic, true);
  });
});

test('API v1 contract task selects the native request binary and focused filter', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(state.calls.find(call => call.command === 'cargo').args,
      ['test', '--locked', '--test', 'requests', 'api_v1::baseline::anonymous_access', '--', '--nocapture']);
    assert.equal(state.owned_endpoint_used, true);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1-inventory', 'FILTER=api_v1::baseline::anonymous_access']);
});

test('API v1 completion task selects the deliberately failing acceptance gate', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(state.calls.find(call => call.command === 'cargo').args,
      ['test', '--locked', '--test', 'requests', 'api_v1', '--', '--include-ignored']);
  }, 'slice:contracts', ['GROUP=api-v1']);
});

test('API v1 inventory task selects all request and fixture cases', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(state.calls.find(call => call.command === 'cargo').args,
      ['test', '--locked', '--test', 'requests', 'api_v1', '--', '--nocapture']);
  }, 'slice:contracts', ['GROUP=api-v1-inventory']);
});

test('API v1 completion cannot narrow coverage with a filter', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /cannot select one test/);
    assert.equal(state.calls.length, 0);
  }, 'slice:contracts', ['GROUP=api-v1', 'FILTER=api_v1::baseline::anonymous_access']);
});

test('contract task rejects unknown groups before provisioning', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Unknown contract group/);
    assert.equal(state.calls.length, 0);
  }, 'slice:contracts', ['GROUP=unknown']);
});

test('care API and complete runs use owned storage and clean it after test failures', async () => {
  for (const [taskName, assignments] of [['slice:test', ['TARGET=care_api']], ['slice:test-all', []]]) {
    await withProcessFixture(42, (result, state) => {
      assert.notEqual(result.status, 0);
      assert.equal(state.owned_endpoint_used, true);
      assert.equal(state.owned_storage_endpoint_used, true);
      const calls = state.calls.filter(call => call.command === 'docker');
      assert.deepEqual(calls.map(call => call.operation), ['up', 'port', 'up', 'port', 'exec', 'down']);
      assert.ok(calls.every(call => call.project === calls[0].project));
      assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    }, taskName, assignments);
  }
});

test('coverage report collects root units, API inventory and care API through one owned fixture', async () => {
  await withProcessFixture(0, async (result, state, directory) => {
    assert.equal(result.status, 0, result.stderr);
    const coverageCalls = state.calls.filter(call => call.command === 'cargo' && call.args[0] === 'llvm-cov' && !call.args.includes('--version'));
    assert.deepEqual(coverageCalls.map(call => call.args.slice(1, 3)), [
      ['clean', '--workspace'],
      ['test', '--no-report'],
      ['test', '--no-report'],
      ['test', '--no-report'],
      ['report', '--lcov'],
    ]);
    assert.ok(coverageCalls[1].args.includes('--lib'));
    assert.ok(coverageCalls[2].args.includes('api_v1'));
    assert.ok(coverageCalls[2].args.includes('requests'));
    assert.ok(coverageCalls[3].args.includes('care_api'));
    assert.ok(coverageCalls[3].args.includes('--test-threads=2'));
    assert.ok(coverageCalls.every(call => call.coverage_target_dir?.startsWith(join(root, 'target/coverage/'))));
    assert.ok(coverageCalls.every(call => call.coverage_build_dir?.startsWith(join(root, 'target/coverage/'))));
    assert.ok(coverageCalls.every(call => call.coverage_profile_override === undefined));
    for (const call of state.calls.filter(call => call.command === 'cargo' && call.args[0] === 'llvm-cov')) {
      for (const name of ['llvm_cov_target_override', 'llvm_cov_build_override', 'llvm_cov_setup_override', 'llvm_cov_deprecated_flags_override', 'llvm_profdata_deprecated_flags_override', 'llvm_cov_tool_override', 'llvm_profdata_tool_override', 'llvm_cov_flags_override', 'llvm_profdata_flags_override', 'llvm_profile_name_override']) {
        assert.equal(call[name], undefined, `${name} escaped into cargo-llvm-cov`);
      }
    }
    assert.equal(state.owned_endpoint_used, true);
    assert.equal(state.owned_storage_used, true);
    assert.equal(state.child_identity_key_synthetic, true);
    assert.equal(state.child_verification_key_synthetic, true);
    assert.equal(state.child_docker_environment_removed, true);
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'up', 'port', 'exec', 'down']);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    const report = join(directory, 'coverage');
    const reportPath = coverageCalls[4].args[coverageCalls[4].args.indexOf('--output-path') + 1];
    assert.ok(reportPath.startsWith(join(report, '')));
    assert.notEqual(reportPath, join(report, 'lcov.info'));
    const completedDirectory = join(reportPath, '..');
    assert.match(await readFile(reportPath, 'utf8'), /SF:src\/lib.rs/);
    const manifest = JSON.parse(await readFile(join(completedDirectory, 'manifest.json'), 'utf8'));
    assert.deepEqual(manifest.selectors, ['lib', 'requests:api_v1', 'care_api']);
    assert.equal(manifest.status, 'complete');
    assert.match(manifest.source_state.sha256, /^[a-f0-9]{64}$/);
    assert.equal(typeof manifest.source_state.dirty, 'boolean');
  }, 'test:coverage', directory => [`OUTPUT_DIR=${join(directory, 'coverage')}`], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never' });
});

test('failed coverage selection stops before report and cleans its owned resources', async () => {
  await withProcessFixture(42, async (result, state, directory) => {
    assert.notEqual(result.status, 0);
    const coverageCalls = state.calls.filter(call => call.command === 'cargo' && call.args[0] === 'llvm-cov' && !call.args.includes('--version'));
    assert.equal(coverageCalls.some(call => call.args.includes('report')), false);
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'up', 'port', 'exec', 'down']);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    assert.deepEqual(await readdir(join(directory, 'coverage')), []);
  }, 'test:coverage', directory => [`OUTPUT_DIR=${join(directory, 'coverage')}`], 0, { SLICE_FIXTURE_FAIL_MATCH: '--test care_api' });
});

test('separate coverage invocations never reuse instrumented Cargo directories', async () => {
  const targets = [];
  for (let index = 0; index < 2; index += 1) {
    await withProcessFixture(0, (result, state) => {
      assert.equal(result.status, 0, result.stderr);
      const coverageCalls = state.calls.filter(call => call.command === 'cargo' && call.args[0] === 'llvm-cov' && call.args.includes('test'));
      assert.equal(new Set(coverageCalls.map(call => call.coverage_target_dir)).size, 1);
      assert.equal(new Set(coverageCalls.map(call => call.coverage_build_dir)).size, 1);
      targets.push(coverageCalls[0].coverage_target_dir);
    }, 'test:coverage', [], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never' });
  }
  assert.notEqual(targets[0], targets[1]);
});

test('coverage prerequisite failure exits before owned fixture provisioning', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /cargo-llvm-cov 0\.9\.1/);
    assert.equal(state.calls.some(call => call.command === 'docker'), false);
  }, 'test:coverage', [], 0, { SLICE_FIXTURE_MISSING_COVERAGE: '1' });
});

test('coverage prerequisite rejects a near-prefix tool version before provisioning', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /cargo-llvm-cov 0\.9\.1/);
    assert.equal(state.calls.some(call => call.command === 'docker'), false);
  }, 'test:coverage', [], 0, { SLICE_FIXTURE_COV_VERSION: '0.9.10' });
});

test('coverage rejects a flag-shaped output directory before provisioning', async () => {
  const invalidName = `--coverage-invalid-${randomUUID()}`;
  try {
    await withProcessFixture(0, (result, state) => {
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, /output directory/i);
      assert.equal(state.calls.some(call => call.command === 'docker'), false);
    }, 'test:coverage', [`OUTPUT_DIR=${invalidName}`], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never' });
  } finally {
    await rm(join(root, invalidName), { recursive: true, force: true });
  }
});

test('coverage rejects surrounding output-directory whitespace before provisioning', async () => {
  const invalidName = ` coverage-invalid-${randomUUID()} `;
  try {
    await withProcessFixture(0, (result, state) => {
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, /output directory/i);
      assert.equal(state.calls.some(call => call.command === 'docker'), false);
    }, 'test:coverage', [`OUTPUT_DIR=${invalidName}`], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never' });
  } finally {
    await rm(join(root, invalidName), { recursive: true, force: true });
  }
});

test('coverage prerequisite rejects missing LLVM tools before provisioning', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /llvm-tools-preview/);
    assert.equal(state.calls.some(call => call.command === 'docker'), false);
  }, 'test:coverage', [], 0, { SLICE_FIXTURE_MISSING_LLVM_TOOLS: '1' });
});
