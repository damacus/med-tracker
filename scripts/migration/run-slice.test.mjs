import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { once } from 'node:events';
import { mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { sourceIdentity } from './run-slice.mjs';
import { acquireCoverageLock } from './coverage-lock.mjs';
import { parseNextestJUnit } from './nextest-proof.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const ambientUrl = 'postgres://unowned.invalid:1/unowned';

test('ordinary Rust entry points load without the optional proof parser', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'medtracker-missing-parser-'));
  try {
    const loader = join(directory, 'block-saxes.mjs');
    await writeFile(loader, `export async function resolve(specifier, context, nextResolve) {
  if (specifier === 'saxes') throw new Error('isolated checkout has no saxes');
  return nextResolve(specifier, context);
}\n`);
    const environment = { NODE_OPTIONS: `--experimental-loader=${loader}` };
    const imported = spawnSync(process.execPath, ['--input-type=module', '-e', 'await import("./scripts/migration/run-slice.mjs")'], { cwd: root, env: { ...process.env, ...environment }, encoding: 'utf8' });
    assert.equal(imported.status, 0, imported.stderr);
    for (const [taskName, assignments] of [['slice:test', ['TARGET=persistence']], ['slice:test-all', []]]) {
      await withProcessFixture(0, (result, state) => {
        assert.equal(result.status, 0, result.stderr);
        assert.ok(state.calls.some(call => call.command === 'cargo'));
      }, taskName, assignments, 0, environment);
    }
    const proof = spawnSync(process.execPath, ['--input-type=module', '-e', 'const { runSlice } = await import("./scripts/migration/run-slice.mjs"); await runSlice({ target: "persistence", executionProof: true })'], { cwd: root, env: { ...process.env, ...environment }, encoding: 'utf8' });
    assert.notEqual(proof.status, 0);
    assert.match(proof.stderr, /isolated checkout has no saxes/);
    for (const taskName of ['slice:contracts', 'slice:result-proof']) {
      const planned = spawnSync('task', ['--force', '--dry', '--verbose', taskName, ...(taskName === 'slice:contracts' ? ['GROUP=api-v1'] : [])], { cwd: root, encoding: 'utf8' });
      assert.equal(planned.status, 0, planned.stderr);
      const commands = `${planned.stdout}${planned.stderr}`;
      assert.ok(commands.includes('npm ci') && commands.indexOf('npm ci') < commands.indexOf('node ', commands.indexOf('npm ci')), `${taskName} must install before loading proof: ${commands}`);
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('coverage Task installs its JUnit parser before invoking the owned runner', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'medtracker-coverage-missing-parser-'));
  try {
    const loader = join(directory, 'block-saxes.mjs');
    await writeFile(loader, `export async function resolve(specifier, context, nextResolve) {
  if (specifier === 'saxes') throw new Error('isolated checkout has no saxes');
  return nextResolve(specifier, context);
}\n`);
    const missing = spawnSync(process.execPath, ['--input-type=module', '-e', 'await import("./scripts/migration/nextest-proof.mjs")'], { cwd: root, env: { ...process.env, NODE_OPTIONS: `--experimental-loader=${loader}` }, encoding: 'utf8' });
    assert.notEqual(missing.status, 0);
    assert.match(missing.stderr, /isolated checkout has no saxes/);
    const planned = spawnSync('task', ['--force', '--dry', '--verbose', 'test:coverage'], { cwd: root, encoding: 'utf8' });
    assert.equal(planned.status, 0, planned.stderr);
    const commands = `${planned.stdout}${planned.stderr}`;
    const install = commands.indexOf('npm ci');
    const runner = commands.indexOf('node scripts/migration/run-slice.mjs --coverage');
    assert.ok(install >= 0 && runner > install, `Coverage parser must install before fixture provisioning: ${commands}`);
    const focused = spawnSync('task', ['--force', '--dry', '--verbose', 'test:coverage-prerequisite'], { cwd: root, encoding: 'utf8' });
    assert.equal(focused.status, 0, focused.stderr);
    const focusedCommands = `${focused.stdout}${focused.stderr}`;
    const testCommand = focusedCommands.indexOf('node --test --test-name-pattern=');
    assert.ok(focusedCommands.indexOf('npm ci') >= 0 && testCommand > focusedCommands.indexOf('npm ci'), `Focused prerequisite test must install before importing parser: ${focusedCommands}`);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

async function withProcessFixture(exitCode, callback, taskName = 'slice:test', assignments = ['TARGET=persistence'], provisionExit = 0, extraEnvironment = {}, { closeOutputBeforeCargo = false, injectReadableError = false } = {}) {
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
  state.owned_avatar_storage_used = state.owned_storage_endpoint_used && process.env.ACTIVE_STORAGE_SERVICE === 's3' && process.env.ACTIVE_STORAGE_S3_BUCKET === 'medtracker-fixture' && process.env.ACTIVE_STORAGE_S3_REGION === 'us-east-1' && process.env.ACTIVE_STORAGE_S3_ACCESS_KEY_ID === 'storage-smoke-access' && process.env.ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY === 'storage-smoke-secret' && process.env.ACTIVE_STORAGE_S3_FORCE_PATH_STYLE === 'true';
  state.child_identity_key_synthetic = process.env.MEDTRACKER_SESSION_KEY === Buffer.alloc(64, 7).toString('base64');
  state.child_verification_key_synthetic = process.env.RAILS_SECRET_KEY_BASE === 'synthetic-slice-rails-verification-secret';
  state.child_compose_environment_removed = ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES'].every(name => process.env[name] === undefined);
  state.child_docker_environment_removed = ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH'].every(name => process.env[name] === undefined);
  if (process.env.DATABASE_URL === ${JSON.stringify(ambientUrl)}) state.foreign_resources_touched += 1;
  state.calls.push({ command, args, rust_test_nocapture: process.env.RUST_TEST_NOCAPTURE, coverage_target_dir: process.env.CARGO_TARGET_DIR, coverage_build_dir: process.env.CARGO_BUILD_BUILD_DIR, coverage_profile_override: process.env.LLVM_PROFILE_FILE, llvm_cov_target_override: process.env.CARGO_LLVM_COV_TARGET_DIR, llvm_cov_build_override: process.env.CARGO_LLVM_COV_BUILD_DIR, llvm_cov_setup_override: process.env.CARGO_LLVM_COV_SETUP, llvm_cov_deprecated_flags_override: process.env.CARGO_LLVM_COV_FLAGS, llvm_profdata_deprecated_flags_override: process.env.CARGO_LLVM_PROFDATA_FLAGS, llvm_cov_tool_override: process.env.LLVM_COV, llvm_profdata_tool_override: process.env.LLVM_PROFDATA, llvm_cov_flags_override: process.env.LLVM_COV_FLAGS, llvm_profdata_flags_override: process.env.LLVM_PROFDATA_FLAGS, llvm_profile_name_override: process.env.LLVM_PROFILE_FILE_NAME });
  state.calls.at(-1).owned_endpoint_used = state.owned_endpoint_used;
  fs.writeFileSync(statePath, JSON.stringify(state));
  if (command === 'cargo' && args.includes('--version')) {
    if (process.env.SLICE_FIXTURE_MISSING_COVERAGE === '1') process.exit(101);
    process.stdout.write(args.includes('nextest') ? 'cargo-nextest ' + (process.env.SLICE_FIXTURE_NEXTEST_VERSION || '0.9.148') + '\\n' : 'cargo-llvm-cov ' + (process.env.SLICE_FIXTURE_COV_VERSION || '0.9.1') + '\\n');
  }
  if (command === 'cargo' && args[0] === 'nextest' && args[1] === 'list') {
    const binary = args.includes('--lib') ? 'med-tracker' : 'med-tracker::' + args[args.indexOf('--test') + 1];
    const names = binary === 'med-tracker' ? ['unit_case', 'held_case'] : binary.endsWith('::requests') ? ['api_v1::baseline::anonymous_access', 'other_case', 'held_case'] : ['care_case'];
    const testcases = Object.fromEntries(names.map(name => [name, { ignored: name === 'held_case' }]));
    process.stdout.write(JSON.stringify(process.env.SLICE_FIXTURE_COVERAGE_EMPTY_LIST === '1' ? { 'test-count': 0, 'rust-suites': {} } : { 'test-count': names.length, 'rust-suites': { [binary]: { 'binary-id': binary, status: 'listed', testcases } } }));
    process.exit(0);
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
    if (process.env.SLICE_FIXTURE_EXTRA_REPORT_PATH) fs.writeFileSync(process.env.SLICE_FIXTURE_EXTRA_REPORT_PATH, 'unrelated source change\\n');
  }
  const complete = () => {
    if (command === 'cargo' && args[0] === 'llvm-cov' && args[1] === 'nextest' && args.includes('--config-file')) {
      const binary = args.includes('--lib') ? 'med-tracker' : 'med-tracker::' + args[args.indexOf('--test') + 1];
      const names = binary === 'med-tracker' ? ['unit_case', 'held_case'] : binary.endsWith('::requests') ? ['api_v1::baseline::anonymous_access', 'held_case'] : ['care_case'];
      const mode = process.env.SLICE_FIXTURE_COVERAGE_PROOF_MODE;
      const reported = mode === 'wrong-name' && binary.endsWith('::care_api') ? ['wrong_case'] : names;
      const config = fs.readFileSync(args[args.indexOf('--config-file') + 1], 'utf8');
      const reportName = /path = "([^"]+)"/.exec(config)?.[1];
      const reportPath = path.join(process.cwd(), 'target', 'nextest', 'contract-proof', reportName);
      const testcase = reported.map(name => '<testcase name="' + name + '" classname="' + binary + '">' + (name === 'held_case' || mode === 'skipped' && binary.endsWith('::care_api') ? '<skipped/>' : '') + '</testcase>').join('');
      fs.mkdirSync(path.dirname(reportPath), { recursive: true });
      if (mode !== 'missing' || !binary.endsWith('::care_api')) fs.writeFileSync(reportPath, '<testsuites tests="' + reported.length + '" failures="0" errors="0"><testsuite name="' + binary + '" tests="' + reported.length + '" failures="0" errors="0">' + testcase + '</testsuite></testsuites>');
    }
    const transcript = args.includes('--run-ignored') && process.env.SLICE_FIXTURE_COMPLETION_TRANSCRIPT
      ? process.env.SLICE_FIXTURE_COMPLETION_TRANSCRIPT : args.includes('care_api') && process.env.SLICE_FIXTURE_CARE_TRANSCRIPT
        ? process.env.SLICE_FIXTURE_CARE_TRANSCRIPT : process.env.SLICE_FIXTURE_CARGO_TRANSCRIPT;
    if (command === 'cargo' && args.includes('--config-file') && transcript) {
      const config = fs.readFileSync(args[args.indexOf('--config-file') + 1], 'utf8');
      const reportName = /path = "([^"]+)"/.exec(config)?.[1];
      const reportPath = path.join(process.env.CARGO_TARGET_DIR, 'nextest', 'contract-proof', reportName);
      const summary = /test result: ok\\. (\\d+) passed; (\\d+) failed; (\\d+) ignored;/.exec(transcript);
      if (summary) {
        const binary = (args.includes('-p') ? args[args.indexOf('-p') + 1] : 'med-tracker') + '::' + args[args.indexOf('--test') + 1];
        const cases = [...transcript.matchAll(/^test ([A-Za-z0-9_:]+)(?: - should panic(?: - expected)?)? \\.\\.\\. (ok|ignored(?:, .+)?|FAILED)$/gm)];
        const testcase = cases.map(([, name, status]) => '<testcase name="' + name + '" classname="' + binary + '">' + (status.startsWith('ignored') ? '<skipped/>' : status === 'FAILED' ? '<failure/>' : '') + '</testcase>').join('');
        const tests = Number(summary[1]) + Number(summary[2]) + Number(summary[3]);
        const counts = ' tests="' + tests + '" failures="' + summary[2] + '" errors="0"';
        fs.mkdirSync(path.dirname(reportPath), { recursive: true });
        fs.writeFileSync(reportPath, '<testsuites' + counts + '><testsuite name="' + binary + '"' + counts + '>' + testcase + '</testsuite></testsuites>' + 'x'.repeat(Number(process.env.SLICE_FIXTURE_CARGO_OVERFLOW_BYTES || 0)));
      }
    }
    if (command === 'cargo' && transcript) process.stdout.write(transcript);
    if (command === 'cargo' && process.env.SLICE_FIXTURE_CARGO_OVERFLOW_BYTES) {
      process.stdout.write('x'.repeat(Number(process.env.SLICE_FIXTURE_CARGO_OVERFLOW_BYTES)));
    }
    process.exitCode = command === 'cargo' && (!process.env.SLICE_FIXTURE_FAIL_MATCH || args.join(' ').includes(process.env.SLICE_FIXTURE_FAIL_MATCH)) ? Number(process.env.SLICE_FIXTURE_EXIT) : 0;
  };
  if (command === 'cargo' && process.env.SLICE_FIXTURE_CARGO_GATE) {
    const gate = require('node:net').connect(process.env.SLICE_FIXTURE_CARGO_GATE);
    gate.once('data', () => { gate.end(); complete(); });
  } else complete();
}
`;
  for (const command of ['docker', 'cargo', 'curl', 'rustup']) await writeFile(join(directory, command), executable, { mode: 0o755 });
  let gateDirectory;
  let gateServer;
  let gateConnection;
  try {
    const taskAssignments = typeof assignments === 'function' ? assignments(directory) : assignments;
    let gateReady;
    let gatePath;
    if (closeOutputBeforeCargo || injectReadableError) {
      gateDirectory = await mkdtemp(join(tmpdir(), 'mt-gate-'));
      gatePath = join(gateDirectory, 's');
      gateReady = new Promise(resolve => {
        gateServer = createServer(connection => { gateConnection = connection; resolve(connection); });
      });
      gateServer.listen(gatePath);
      await once(gateServer, 'listening');
    }
    let preloadPath;
    if (injectReadableError) {
      preloadPath = join(directory, 'inject-readable-error.mjs');
      await writeFile(preloadPath, `import childProcess from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';
const original = childProcess.spawn;
childProcess.spawn = (...args) => {
  const child = original(...args);
  if (args[0] === 'cargo' && child.stdout) child.stdout.once('data', () => child.stdout.destroy(new Error('synthetic EIO')));
  return child;
};
syncBuiltinESMExports();
`);
    }
      const options = {
      cwd: root,
      env: { ...process.env, PATH: `${directory}:${process.env.PATH}`, DATABASE_URL: ambientUrl, MEDTRACKER_SESSION_KEY: 'ambient-identity-fixture-key-must-not-be-used', COMPOSE_FILE: '/unowned/compose.yaml', COMPOSE_PROJECT_NAME: 'unrelated', DOCKER_HOST: 'tcp://unowned.invalid:2376', DOCKER_CONTEXT: 'unrelated', DOCKER_CONFIG: '/unowned/docker-config', DOCKER_TLS_VERIFY: '1', DOCKER_CERT_PATH: '/unowned/certs', SLICE_FILTER: 'unrelated_filter', SLICE_FIXTURE_STATE: statePath, SLICE_FIXTURE_EXIT: String(exitCode), SLICE_FIXTURE_PROVISION_EXIT: String(provisionExit), CARGO_TARGET_DIR: taskName === 'slice:contracts' ? join(directory, 'target') : '/unowned/target', CARGO_BUILD_BUILD_DIR: '/unowned/intermediates', LLVM_PROFILE_FILE: '/unowned/profile.profraw', CARGO_LLVM_COV_TARGET_DIR: '/unowned/llvm-cov-target', CARGO_LLVM_COV_BUILD_DIR: '/unowned/llvm-cov-build', CARGO_LLVM_COV_SETUP: 'skip', CARGO_LLVM_COV_FLAGS: '--unowned', CARGO_LLVM_PROFDATA_FLAGS: '--unowned', LLVM_COV: '/unowned/llvm-cov', LLVM_PROFDATA: '/unowned/llvm-profdata', LLVM_COV_FLAGS: '--unowned', LLVM_PROFDATA_FLAGS: '--unowned', LLVM_PROFILE_FILE_NAME: 'unowned.profraw', ...(taskName === 'slice:contracts' && taskAssignments.includes('GROUP=api-v1') ? { SLICE_FIXTURE_CARE_TRANSCRIPT: requestTranscript(creditedCareTests, []) } : {}), ...extraEnvironment, ...(gatePath ? { SLICE_FIXTURE_CARGO_GATE: gatePath } : {}), ...(preloadPath ? { NODE_OPTIONS: `${process.env.NODE_OPTIONS ?? ''} --import=${preloadPath}`.trim() } : {}) },
      encoding: 'utf8',
      timeout: 15000,
      maxBuffer: 4 * 1024 * 1024,
    };
    let result;
    if (closeOutputBeforeCargo || injectReadableError) {
      const child = spawn('task', ['--taskfile', join(root, 'Taskfile.yml'), taskName, ...taskAssignments], { ...options, stdio: ['ignore', 'pipe', 'pipe'] });
      let stderr = '';
      child.stderr.on('data', chunk => { stderr += chunk; });
      if (injectReadableError) child.stdout.on('data', () => {});
      const closed = new Promise((resolve, reject) => {
        child.once('error', reject);
        child.once('close', status => resolve({ status, stderr }));
      });
      await Promise.race([gateReady, closed.then(() => { throw new Error(`Task exited before fake Cargo connected: ${stderr}`); })]);
      if (closeOutputBeforeCargo) {
        const stdoutClosed = once(child.stdout, 'close');
        child.stdout.destroy();
        await stdoutClosed;
      }
      gateConnection.write('go');
      result = await closed;
    } else result = spawnSync('task', ['--taskfile', join(root, 'Taskfile.yml'), taskName, ...taskAssignments], options);
    assert.equal(result.error, undefined, result.error?.message);
    await callback(result, JSON.parse(await readFile(statePath, 'utf8')), directory);
  } finally {
    gateConnection?.destroy();
    if (gateServer) await new Promise(resolve => gateServer.close(resolve));
    if (gateDirectory) await rm(gateDirectory, { recursive: true, force: true });
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

test('request acceptance provisions owned avatar storage and removes it on success or failure', async () => {
  for (const exitCode of [0, 42]) {
    await withProcessFixture(exitCode, (result, state) => {
      assert.equal(result.status === 0, exitCode === 0, result.stderr);
      assert.equal(state.owned_avatar_storage_used, true, `requests need owned avatar storage: ${JSON.stringify(state.calls)}`);
      const docker = state.calls.filter(call => call.command === 'docker');
      assert.ok(docker.some(call => call.operation === 'up' && call.args.includes('rustfs')));
      assert.ok(docker.some(call => call.operation === 'down' && call.args.includes('--profile') && call.args.includes('fixture-storage') && call.args.includes('--volumes')));
      assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
      assert.equal(state.foreign_resources_touched, 0);
    }, 'slice:contracts', ['GROUP=api-v1-inventory', 'FILTER=api_v1::avatar'], 0);
  }
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
    const calls = state.calls.filter(call => call.command === 'cargo');
    assert.deepEqual(calls.map(call => call.args), [
      ['nextest', 'run', '--locked', '--no-tests=fail', '--workspace', '--test-threads', '4'],
      ['test', '--locked', '--doc'],
    ]);
    assert.equal(calls[0].owned_endpoint_used, true);
    assert.equal(calls[1].owned_endpoint_used, false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:test-all', []);
});

test('the explicit diagnostic option runs every selected test after a failure', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(state.calls.find(call => call.command === 'cargo').args,
      ['nextest', 'run', '--locked', '--no-tests=fail', '-p', 'med-tracker', '--test', 'care_api', '--test-threads', '2', '--no-fail-fast', 'platform']);
  }, 'slice:test', ['TARGET=care_api', 'FILTER=platform', 'NO_FAIL_FAST=true']);
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
      ['nextest', 'run', '--locked', '--no-tests=fail', '-p', 'med-tracker', '--test', 'requests', '--no-capture', 'api_v1::baseline::anonymous_access']);
    assert.equal(state.owned_endpoint_used, true);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1-inventory', 'FILTER=api_v1::baseline::anonymous_access']);
});

const creditedRequestTests = [
  'api_v1::household_settings::settings_audit_failure_hides_read_and_rolls_back_both_write_methods',
  'api_v1::household_settings::settings_current_manage_permission_blocks_writes_without_domain_effects',
  'api_v1::household_settings::settings_methods_reject_invalid_revoked_and_wrong_namespace_credentials',
  'api_v1::household_settings::settings_methods_reject_stale_membership_credentials_before_changes',
  'api_v1::household_settings::settings_methods_share_fixed_general_limit_without_domain_effects',
  'api_v1::household_settings::settings_patch_and_put_distinguish_inaccessible_and_absent_households_without_writes',
  'api_v1::household_settings::settings_patch_and_put_save_allowed_fields_without_replacing_slug_or_omitted_fields',
  'api_v1::household_settings::settings_patch_and_put_validate_bodies_and_leave_domain_state_unchanged',
  'api_v1::household_settings::settings_patch_concurrent_keyed_requests_commit_once',
  'api_v1::household_settings::settings_patch_replays_same_key_and_rejects_changed_body',
  'api_v1::household_settings::settings_put_concurrent_keyed_requests_commit_once',
  'api_v1::household_settings::settings_put_replays_same_key_and_rejects_changed_body',
  'api_v1::household_settings::settings_reads_scope_three_credential_kinds_and_validate_responses',
  'api_v1::avatar::avatar_malformed_multipart_does_not_replace_current_image',
  'api_v1::avatar::avatar_methods_recheck_stale_permissions_and_membership',
  'api_v1::avatar::avatar_methods_reject_wrong_namespace_scope_and_revoked_credentials',
  'api_v1::avatar::avatar_methods_separate_own_foreign_absent_and_invalid_credentials',
  'api_v1::avatar::avatar_methods_share_fixed_general_limit_without_storage_effects',
  'api_v1::avatar::avatar_missing_owned_bucket_returns_declared_unavailable_without_attachment',
  'api_v1::avatar::avatar_upload_download_delete_use_real_owned_storage_and_private_wire',
  'api_v1::profile::patch_profile_replays_exact_key_and_rejects_changed_body',
  'api_v1::profile::profile_audit_failure_hides_read_and_rolls_back_writes',
  'api_v1::profile::profile_invalid_and_revoked_credentials_do_not_reveal_household',
  'api_v1::profile::profile_permissions_and_membership_recheck_before_mutation',
  'api_v1::profile::profile_rate_limit_rejects_without_mutation',
  'api_v1::profile::profile_reads_scope_real_credentials_and_serialize_private_response',
  'api_v1::profile::profile_rejects_smart_namespace_and_insufficient_mobile_scope',
  'api_v1::profile::profile_write_methods_document_existing_keyed_conflict',
  'api_v1::profile::profile_writes_save_allowed_fields_and_reject_security_fields_atomically',
  'api_v1::profile::put_profile_replays_exact_key_and_rejects_changed_body',
  'api_v1::people::people_methods_reject_invalid_and_wrong_namespace_credentials_before_writes',
  'api_v1::people::revoked_credentials_and_withdrawn_membership_block_every_people_method',
  'api_v1::people::all_four_people_methods_share_fixed_general_limit_without_write_effects',
  'api_v1::people::people_audit_insert_failure_rolls_back_every_route_without_private_data',
  'api_v1::people::people_collection_scopes_credentials_and_validates_queries',
  'api_v1::people::people_listing_hides_data_when_restricted_household_read_fails',
  'api_v1::people::people_listing_rechecks_account_closed_during_mobile_authentication',
  'api_v1::people::people_write_routes_document_existing_idempotency_conflict',
  'api_v1::people::people_write_methods_recheck_current_manage_permission',
  'api_v1::people::app_and_mobile_credentials_can_each_write_all_people_methods',
  'api_v1::people::patch_person_merges_fields_and_rejects_changed_key_replay',
  'api_v1::people::concurrent_patch_person_key_replays_one_write',
  'api_v1::people::put_person_merges_fields_and_rejects_changed_key_replay',
  'api_v1::people::concurrent_put_person_key_replays_one_write',
  'api_v1::account::account_collections_keep_household_scope_and_session_secrets_out',
  'api_v1::account::account_management_authentication_envelopes_match_each_route',
  'api_v1::account::account_revoke_and_logout_preserve_namespace_idempotence_and_audit_effects',
  'api_v1::account::account_routes_reject_invalid_revoked_and_stale_credentials_without_writes',
  'api_v1::account::account_routes_share_the_fixed_general_limit',
  'api_v1::account::concurrent_account_revocation_and_logout_commit_one_effect_each',
  'api_v1::account::integration_grant_cannot_manage_account_sessions_but_can_log_out',
  'api_v1::account::malformed_delete_session_authenticates_before_identifier_validation',
  'api_v1::account::mobile_sessions_hide_foreign_and_expired_login_targets_from_listing_and_revocation',
  'api_v1::capabilities::advertised_features_match_executed_root_routes',
  'api_v1::capabilities::profile_and_avatar_support_is_discoverable',
  'api_v1::capabilities::public_mobile_clients_include_name_and_exclude_secrets',
  'api_v1::capabilities::configured_mobile_lifetimes_include_disabled_maximum_age',
  'api_v1::capabilities::capability_requests_use_the_fixed_general_limit',
  'api_v1::capabilities::capability_database_lookup_failure_uses_existing_unavailable_envelope',
  'api_v1::baseline::anonymous_access',
  'api_v1::me_ui::active_credentials_distinguish_own_foreign_and_missing_households',
  'api_v1::me_ui::invalid_and_revoked_credentials_do_not_reveal_household_existence',
  'api_v1::me_ui::me_and_ui_reads_share_the_fixed_general_limit_without_rejection_effects',
  'api_v1::me_ui::mobile_grant_without_medtracker_scope_is_forbidden_on_me_and_ui',
  'api_v1::me_ui::smart_integration_grant_cannot_read_household_me_or_ui',
  'api_v1::me_ui::ui_capabilities_private_success_and_anonymous_denial_are_not_cached',
  'api_v1::me_ui::ui_capabilities_rechecks_membership_and_hides_data_on_audit_failure',
  'api_v1::rate_limit::fixed_export_window_rejects_eleventh_request_with_contract_headers',
  'api_v1::rate_limit::untrusted_forwarded_sources_share_the_socket_peer_bucket',
  'api_v1::rate_limit::configured_loopback_proxy_separates_forwarded_client_buckets',
  'api_v1::rate_limit::head_export_uses_the_get_export_bucket',
];

const creditedCareTests = [
  'administration::household_admin_settings_get_matches_documented_contract',
  'administration::household_administration_saved_rails_retries_preserve_effects',
  'administration::household_administration_strong_parameters_filter_protected_fields_and_reject_empty',
  'administration::household_settings_and_grant_audit_failure_roll_back_effects_and_keys',
  'administration::household_settings_concurrent_key_retries_apply_once_and_conflict_on_changed_body',
  'administration::household_settings_partial_update_replay_and_permission_order',
  'avatar::avatar_failed_reply_cleanup_preserves_a_committed_attachment',
  'avatar::avatar_failed_reply_cleanup_waits_for_pending_attachment_commit',
  'avatar::avatar_read_prevents_content_sniffing',
  'avatar::avatar_read_rejects_active_legacy_content_types_without_removing_attachment',
  'avatar::avatar_read_rejects_legacy_non_image_bytes_without_deleting_attachment',
  'avatar::avatar_retirement_enqueue_failure_preserves_current_attachment',
  'avatar::avatar_retirement_worker_preserves_active_key_and_retries_removed_key_idempotently',
  'avatar::concurrent_avatar_replace_and_remove_leave_one_consistent_attachment_after_retirement',
  'avatar::profile_avatar_api_audit_failure_rolls_back_metadata_and_cleans_new_object',
  'avatar::profile_avatar_api_rejects_bad_images_without_replacing_existing',
  'avatar::profile_avatar_api_replacement_and_removal_queue_retired_objects_atomically',
  'avatar::profile_avatar_api_storage_failure_keeps_existing_attachment',
  'avatar::profile_avatar_api_updates_s3_primary_mirror_without_purging_unmigrated_services',
  'avatar::profile_avatar_api_upload_read_replace_remove_and_recheck_access',
  'guardian_declarations::guardian_declarations_manual_create_requires_explicit_attestation',
  'guardian_declarations::guardian_declarations_manual_create_is_atomic_and_replay_safe',
  'profile::age_validation_ignores_existing_and_submitted_account_timezones',
  'profile::current_person_profile_api_saves_nonsecurity_fields_and_preserves_omitted_values',
  'profile::keyed_profile_update_replays_without_a_second_account_version',
  'profile::membership_change_during_read_reports_forbidden_not_missing_profile',
  'profile::overlapping_keyed_profile_and_notification_requests_replay_once_and_conflict',
  'profile::personal_key_profile_and_preferences_overlap_and_revoked_keys_cannot_replay',
  'profile::profile_and_notification_apis_reject_malformed_requests_without_writes',
  'profile::profile_and_notification_apis_require_the_account_linked_membership_person',
  'profile::profile_and_notification_apis_roll_back_mutations_when_request_audit_fails',
  'profile::profile_and_notification_keyed_replays_reject_revoked_oauth_credentials',
  'profile::profile_api_clears_stored_timezone_and_reads_legacy_gravatar_without_cache',
  'profile::profile_api_rejects_invalid_and_security_fields_atomically_and_rechecks_grants',
  'profile::profile_get_matches_documented_contract',
  'profile::profile_requires_a_date_of_birth_and_rejects_incomplete_records_without_writes',
  'profile::profile_write_does_not_deadlock_a_household_writer_recording_actor_audit',
  'profile::simultaneous_profile_preferences_updates_preserve_both_supplied_fields',
  'account_sessions::audit_failure_rolls_back_revocation_and_logout_instead_of_returning_no_content',
  'account_sessions::logout_is_idempotent_and_revokes_each_presented_credential_kind',
  'account_sessions::revocation_audit_identifies_the_target_credential_and_actor_without_secrets',
  'account_sessions::same_application_mobile_revocations_identify_each_grant_in_audit',
  'me::me_audit_failure_does_not_return_or_cache_personal_data',
  'me::me_rejects_malformed_household_ids_without_caching',
  'me::me_rejects_revoked_membership_and_credentials',
  'me::me_rejects_unauthenticated_and_missing_household_reads',
  'me::me_returns_current_account_person_and_selected_membership',
  'me::me_returns_own_person_when_membership_lives_in_another_household',
  'ui_capabilities::ui_capabilities_reflects_current_membership_and_active_grants',
  'ui_capabilities::ui_capabilities_rejects_unauthenticated_and_missing_household_reads',
  'ui_capabilities::ui_capabilities_reports_full_controls_for_household_administrator',
];

function requestTranscript(passed, ignored = ['api_v1::acceptance']) {
  return [
    `running ${passed.length + ignored.length} tests`,
    ...passed.map(name => `test ${name} ... ok`),
    ...ignored.map(name => `test ${name} ... ignored`),
    '',
    `test result: ok. ${passed.length} passed; 0 failed; ${ignored.length} ignored; 0 measured; 0 filtered out; finished in 0.00s`,
    '',
  ].join('\n');
}

test('API v1 completion rejects Cargo success with zero executed credited tests', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /contract|credited|evidence|executed/i);
    assert.ok(state.calls.some(call => call.command === 'cargo' && call.args.includes('requests')));
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--run-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: 'running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n',
  });
});

test('API v1 completion rejects an ignored credited case despite Cargo success', async () => {
  const ignored = [creditedRequestTests.at(-1), 'api_v1::acceptance'];
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /head_export_uses_the_get_export_bucket|ignored|missing.*test|evidence/i);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--run-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests.slice(0, -1), ignored),
  });
});

test('API v1 completion rejects a passing prefix of an unexecuted credited case', async () => {
  const passed = [...creditedRequestTests.slice(0, -1), `${creditedRequestTests.at(-1)}_other`];
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /head_export_uses_the_get_export_bucket|missing.*test|evidence/i);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--run-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(passed),
  });
});

test('API v1 completion rejects a missing care API proof before the final gate', async () => {
  const omitted = creditedCareTests.at(-1);
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, new RegExp(`Missing executed contract evidence: care_api::${omitted}`));
    const cargo = state.calls.filter(call => call.command === 'cargo');
    assert.deepEqual(cargo.map(call => call.args[call.args.indexOf('--test') + 1]), ['requests', 'care_api']);
    assert.equal(cargo.some(call => call.args.includes('--run-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
    SLICE_FIXTURE_CARE_TRANSCRIPT: requestTranscript(creditedCareTests.slice(0, -1), []),
  });
});

test('API v1 completion rejects a missing JUnit report and cleans up', async () => {
  const transcript = [
    `running ${creditedRequestTests.length} tests`,
    ...creditedRequestTests.map(name => `test ${name} ... ok`),
    '',
  ].join('\n');
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /missing.*JUnit/i);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--run-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, { SLICE_FIXTURE_CARGO_TRANSCRIPT: transcript });
});

test('API v1 completion rejects an inconsistent libtest summary and cleans up', async () => {
  const transcript = requestTranscript(creditedRequestTests)
    .replace(`${creditedRequestTests.length} passed; 0 failed; 1 ignored`, `${creditedRequestTests.length + 1} passed; 0 failed; 1 ignored`);
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /summary|count|transcript/i);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--run-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, { SLICE_FIXTURE_CARGO_TRANSCRIPT: transcript });
});

test('API v1 completion bounds JUnit report size and cleans up', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /limit|large|budget|size/i);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--run-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
    SLICE_FIXTURE_CARGO_OVERFLOW_BYTES: String(1024 * 1024 + 1),
  });
});

test('missing contract report stops proof and cleans the owned database', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /missing.*JUnit/i);
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'up', 'port', 'exec', 'down']);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    assert.equal(state.calls.filter(call => call.command === 'cargo').length, 1);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--run-ignored')), false);
  }, 'slice:contracts', ['GROUP=api-v1']);
});

test('nonzero nextest execution cleans the owned database', async () => {
  await withProcessFixture(42, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /Slice Cargo execution exited 42/);
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'up', 'port', 'exec', 'down']);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    assert.equal(state.calls.filter(call => call.command === 'cargo').length, 1);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--run-ignored')), false);
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
  });
});

test('API v1 completion observes exact credited cases and runs the existing acceptance gate', async () => {
  await withProcessFixture(42, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    const cargo = state.calls.filter(call => call.command === 'cargo');
    assert.deepEqual(cargo.map(call => call.args[call.args.indexOf('--test') + 1]), ['requests', 'care_api', 'requests']);
    const requests = cargo.filter(call => call.args.includes('requests'));
    assert.ok(requests.some(call => !call.args.includes('--run-ignored')));
    assert.deepEqual(requests[0].args.slice(0, 8),
      ['nextest', 'run', '--locked', '--no-tests=fail', '-p', 'med-tracker', '--test', 'requests']);
    assert.deepEqual(cargo[1].args.slice(0, 10),
      ['nextest', 'run', '--locked', '--no-tests=fail', '-p', 'med-tracker', '--test', 'care_api', '--test-threads', '2']);
    assert.ok(requests.at(-1).args.includes('--run-ignored'));
    assert.ok(cargo.every(call => call.args.includes('--config-file') && call.args.includes('contract-proof') && call.args.includes('--retries')));
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
    SLICE_FIXTURE_FAIL_MATCH: '--run-ignored',
  });
});

test('API v1 completion requires its exact gate case and nonempty final selection', async () => {
  for (const [label, completion] of [
    ['absent', requestTranscript(creditedRequestTests, [])],
    ['renamed', requestTranscript([...creditedRequestTests, 'api_v1::acceptance_renamed'], [])],
    ['ignored', requestTranscript(creditedRequestTests)],
    ['zero', 'running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n'],
  ]) {
    await withProcessFixture(0, (result, state) => {
      assert.notEqual(result.status, 0, label);
      assert.match(result.stderr, label === 'zero' ? /empty.*JUnit/i : /Missing executed API v1 acceptance gate/, label);
      assert.equal(state.calls.filter(call => call.command === 'cargo').length, 3, label);
      assert.ok(state.calls.at(-2).args.includes('--run-ignored'), label);
      assert.deepEqual(state.resources, { unrelated: ['existing-volume'] }, label);
    }, 'slice:contracts', ['GROUP=api-v1'], 0, {
      SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
      SLICE_FIXTURE_COMPLETION_TRANSCRIPT: completion,
    });
  }
});

test('API v1 completion accepts the exact gate after its evidence selection', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    const requests = state.calls.filter(call => call.command === 'cargo');
    assert.equal(requests.length, 3);
    assert.ok(requests.at(-1).args.includes('--run-ignored'));
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
    SLICE_FIXTURE_COMPLETION_TRANSCRIPT: requestTranscript([...creditedRequestTests, 'api_v1::acceptance'], []),
  });
});

test('an unrelated ignored reason preserves credited passes and reaches the acceptance gate', async () => {
  const transcript = requestTranscript(creditedRequestTests).replace(
    'test api_v1::acceptance ... ignored',
    'test api_v1::acceptance ... ignored, full API v1 acceptance remains incomplete during migration',
  );
  await withProcessFixture(42, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /Slice Cargo execution exited 42/);
    const requests = state.calls.filter(call => call.command === 'cargo' && call.args.includes('requests'));
    assert.equal(requests.length, 2);
    assert.ok(requests.at(-1).args.includes('--run-ignored'));
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: transcript,
    SLICE_FIXTURE_FAIL_MATCH: '--run-ignored',
  });
});

test('a credited test with an ignored reason never earns execution credit', async () => {
  const credited = creditedRequestTests.at(-1);
  const transcript = requestTranscript(creditedRequestTests.slice(0, -1), [credited, 'api_v1::acceptance']).replace(
    `test ${credited} ... ignored`,
    `test ${credited} ... ignored, case remains unavailable`,
  );
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, new RegExp(`Missing executed contract evidence: requests::${credited}`));
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--run-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, { SLICE_FIXTURE_CARGO_TRANSCRIPT: transcript });
});

test('API v1 completion removes inherited RUST_TEST_NOCAPTURE only from its Cargo children', async () => {
  const parentValue = process.env.RUST_TEST_NOCAPTURE;
  await withProcessFixture(42, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    const cargo = state.calls.filter(call => call.command === 'cargo');
    assert.ok(cargo.length >= 2);
    assert.ok(cargo.every(call => call.rust_test_nocapture === undefined));
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    RUST_TEST_NOCAPTURE: '1',
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
    SLICE_FIXTURE_FAIL_MATCH: '--run-ignored',
  });
  assert.equal(process.env.RUST_TEST_NOCAPTURE, parentValue);
});

test('ordinary slice tests retain the developer RUST_TEST_NOCAPTURE setting', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.equal(state.calls.find(call => call.command === 'cargo').rust_test_nocapture, '1');
  }, 'slice:test', ['TARGET=persistence'], 0, { RUST_TEST_NOCAPTURE: '1' });
});

test('JUnit execution proof credits passed cases without crediting ignored cases', () => {
  const xml = '<testsuites tests="2" failures="0" errors="0"><testsuite name="med-tracker::requests" tests="2" failures="0" errors="0"><testcase name="api_v1::baseline::anonymous_access" classname="med-tracker::requests"/><testcase name="api_v1::acceptance" classname="med-tracker::requests"><skipped/></testcase></testsuite></testsuites>';
  const result = parseNextestJUnit(xml, 'med-tracker::requests');
  assert.deepEqual([...result.passed], ['api_v1::baseline::anonymous_access']);
  assert.deepEqual([...result.ignored], ['api_v1::acceptance']);
});

test('JUnit execution proof recognises a passing should-panic case', () => {
  const xml = '<testsuites tests="1" failures="0" errors="0"><testsuite name="med-tracker::requests" tests="1" failures="0" errors="0"><testcase name="contract::tests::contract_helper_rejects_duplicate_items" classname="med-tracker::requests"/></testsuite></testsuites>';
  assert.deepEqual([...parseNextestJUnit(xml, 'med-tracker::requests').passed], ['contract::tests::contract_helper_rejects_duplicate_items']);
});

test('JUnit execution proof rejects a count inconsistent with cases', () => {
  const xml = '<testsuites tests="2" failures="0" errors="0"><testsuite name="med-tracker::requests" tests="2" failures="0" errors="0"><testcase name="api_v1::baseline::anonymous_access" classname="med-tracker::requests"/></testsuite></testsuites>';
  assert.throws(() => parseNextestJUnit(xml, 'med-tracker::requests'), /count/i);
});

test('JUnit execution proof rejects truncated or appended XML', () => {
  const xml = '<testsuites tests="1" failures="0" errors="0"><testsuite name="med-tracker::requests" tests="1" failures="0" errors="0"><testcase name="api_v1::baseline::anonymous_access" classname="med-tracker::requests"/></testsuite></testsuites>';
  for (const invalid of [xml.slice(0, -14), `${xml}<extra/>`]) {
    assert.throws(() => parseNextestJUnit(invalid, 'med-tracker::requests'), /xml|closed|expected|text|root|tag/i);
  }
});

test('API v1 inventory task selects all request and fixture cases', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(state.calls.find(call => call.command === 'cargo').args,
      ['nextest', 'run', '--locked', '--no-tests=fail', '-p', 'med-tracker', '--test', 'requests', '--no-capture', 'api_v1']);
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
      ['nextest', '--no-report'],
      ['nextest', '--no-report'],
      ['nextest', '--no-report'],
      ['report', '--lcov'],
    ]);
    assert.ok(coverageCalls[1].args.includes('--lib'));
    assert.ok(coverageCalls[2].args.includes('api_v1'));
    assert.ok(coverageCalls[2].args.includes('requests'));
    assert.ok(coverageCalls[3].args.includes('care_api'));
    assert.ok(coverageCalls[3].args.includes('--test-threads'));
    assert.ok(coverageCalls.slice(1, 4).every(call => call.args.includes('--no-tests=fail') && call.args.includes('--config-file')));
    assert.equal(state.calls.filter(call => call.command === 'cargo' && call.args[0] === 'nextest' && call.args[1] === 'list').length, 3);
    assert.ok(coverageCalls.every(call => call.coverage_target_dir === join(root, 'target/coverage/instrumented/target')));
    assert.ok(coverageCalls.every(call => call.coverage_build_dir === join(root, 'target/coverage/instrumented/intermediates')));
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
    assert.deepEqual(manifest.execution.map(item => [item.binary, item.passed, item.ignored]), [
      ['med-tracker', ['unit_case'], ['held_case']],
      ['med-tracker::requests', ['api_v1::baseline::anonymous_access'], ['held_case']],
      ['med-tracker::care_api', ['care_case'], []],
    ]);
    assert.equal(manifest.status, 'complete');
    assert.equal(manifest.cargo_nextest, 'cargo-nextest 0.9.148');
    assert.match(manifest.source_state.sha256, /^[a-f0-9]{64}$/);
    assert.equal(typeof manifest.source_state.dirty, 'boolean');
  }, 'test:coverage', directory => [`OUTPUT_DIR=${join(directory, 'coverage')}`], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never' });
});

test('coverage preserves cached dependencies after successful and failed runs', async () => {
  const cachedDirectory = join(root, 'target/coverage/instrumented/intermediates/llvm-cov-target');
  const cachedDependency = join(cachedDirectory, `cached-dependency-${randomUUID()}.rlib`);
  await mkdir(cachedDirectory, { recursive: true });
  await writeFile(cachedDependency, 'compiled dependency');
  try {
    for (const exitCode of [0, 42]) {
      await withProcessFixture(exitCode, async (result, state) => {
        assert.equal(result.status === 0, exitCode === 0, result.stderr);
        assert.equal(await readFile(cachedDependency, 'utf8'), 'compiled dependency');
        const release = await acquireCoverageLock(join(root, 'target/coverage/instrumented/run.flock'));
        await release();
        const clean = state.calls.find(call => call.command === 'cargo' && call.args[1] === 'clean');
        assert.deepEqual(clean.args, ['llvm-cov', 'clean', '--workspace']);
      }, 'test:coverage', directory => [`OUTPUT_DIR=${join(directory, 'coverage')}`], 0, { SLICE_FIXTURE_FAIL_MATCH: exitCode === 0 ? 'never' : '--test care_api' });
    }
  } finally {
    await rm(cachedDependency, { force: true });
  }
});

test('coverage report under an unignored checkout directory does not change its own source identity', async () => {
  const outputDir = `docs/coverage-source-identity-${randomUUID()}`;
  try {
    await withProcessFixture(0, async (result, state) => {
      assert.equal(result.status, 0, result.stderr);
      const reportCall = state.calls.find(call => call.command === 'cargo' && call.args.includes('report'));
      const reportPath = reportCall.args[reportCall.args.indexOf('--output-path') + 1];
      assert.ok(reportPath.startsWith(join(root, outputDir, '')));
      assert.match(await readFile(reportPath, 'utf8'), /SF:src\/lib.rs/);
      const manifest = JSON.parse(await readFile(join(reportPath, '..', 'manifest.json'), 'utf8'));
      assert.equal(manifest.status, 'complete');
    }, 'test:coverage', [`OUTPUT_DIR=${outputDir}`], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never' });
  } finally {
    await rm(join(root, outputDir), { recursive: true, force: true });
  }
});

test('coverage still rejects an unrelated source change in the output parent', async () => {
  const outputDir = `docs/coverage-source-identity-${randomUUID()}`;
  const unrelatedPath = join(root, outputDir, 'unrelated.txt');
  try {
    await withProcessFixture(0, async (result, state) => {
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, /Source changed while measuring Rust coverage/);
      const reportCall = state.calls.find(call => call.command === 'cargo' && call.args.includes('report'));
      const reportPath = reportCall.args[reportCall.args.indexOf('--output-path') + 1];
      assert.deepEqual(await readdir(join(reportPath, '..')).catch(() => []), []);
      assert.equal(await readFile(unrelatedPath, 'utf8'), 'unrelated source change\n');
    }, 'test:coverage', [`OUTPUT_DIR=${outputDir}`], 0, {
      SLICE_FIXTURE_FAIL_MATCH: 'never',
      SLICE_FIXTURE_EXTRA_REPORT_PATH: unrelatedPath,
    });
  } finally {
    await rm(join(root, outputDir), { recursive: true, force: true });
  }
});

test('source identity excludes only owned report files while retaining tracked and untracked edits', async () => {
  const checkout = await mkdtemp(join(tmpdir(), 'medtracker-source-identity-'));
  const git = args => {
    const result = spawnSync('git', args, { cwd: checkout, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
  };
  try {
    git(['init', '-q']);
    await writeFile(join(checkout, 'tracked.txt'), 'original\n');
    git(['add', 'tracked.txt']);
    git(['-c', 'user.name=MedTracker Test', '-c', 'user.email=test@example.invalid', '-c', 'commit.gpgsign=false', 'commit', '-qm', 'baseline']);
    const baseline = await sourceIdentity(process.env, undefined, checkout);
    const ownedDirectory = join(checkout, 'docs', 'coverage', randomUUID());
    await mkdir(ownedDirectory, { recursive: true });
    await writeFile(join(ownedDirectory, 'lcov.info'), 'owned report\n');
    const withReport = await sourceIdentity(process.env, ownedDirectory, checkout);
    assert.equal(withReport.sha256, baseline.sha256);
    assert.equal(withReport.dirty, baseline.dirty);
    await writeFile(join(checkout, 'tracked.txt'), 'changed\n');
    assert.notEqual((await sourceIdentity(process.env, ownedDirectory, checkout)).sha256, baseline.sha256);
    await writeFile(join(checkout, 'tracked.txt'), 'original\n');
    await writeFile(join(checkout, 'docs', 'coverage', 'unrelated.txt'), 'unrelated\n');
    assert.notEqual((await sourceIdentity(process.env, ownedDirectory, checkout)).sha256, baseline.sha256);
  } finally {
    await rm(checkout, { recursive: true, force: true });
  }
});

test('source identity retains raw Git status bytes for a tracked non-UTF-8 filename', async () => {
  const checkout = await mkdtemp(join(tmpdir(), 'medtracker-source-identity-'));
  const git = args => {
    const result = spawnSync('git', args, { cwd: checkout, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
  };
  try {
    git(['init', '-q']);
    const blob = spawnSync('git', ['hash-object', '-w', '--stdin'], { cwd: checkout, input: 'original\n', encoding: 'utf8' });
    assert.equal(blob.status, 0, blob.stderr);
    const indexEntry = Buffer.concat([Buffer.from(`100644 ${blob.stdout.trim()} 0\ttracked-`), Buffer.from([0xff, 0])]);
    const update = spawnSync('git', ['update-index', '-z', '--index-info'], { cwd: checkout, input: indexEntry });
    assert.equal(update.status, 0, update.stderr?.toString());
    git(['-c', 'user.name=MedTracker Test', '-c', 'user.email=test@example.invalid', '-c', 'commit.gpgsign=false', 'commit', '-qm', 'baseline']);
    const before = await sourceIdentity(process.env, undefined, checkout);
    const ownedDirectory = join(checkout, 'docs', 'coverage', randomUUID());
    await mkdir(ownedDirectory, { recursive: true });
    await writeFile(join(ownedDirectory, 'lcov.info'), 'owned report\n');
    const after = await sourceIdentity(process.env, ownedDirectory, checkout);
    assert.equal(after.sha256, before.sha256);
    assert.equal(after.dirty, true);
  } finally {
    await rm(checkout, { recursive: true, force: true });
  }
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

for (const mode of ['missing', 'wrong-name', 'skipped']) {
  test(`coverage rejects ${mode} nextest proof before LCOV or complete manifest`, async () => {
    await withProcessFixture(0, async (result, state, directory) => {
      assert.notEqual(result.status, 0);
      assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('report')), false);
      assert.deepEqual(await readdir(join(directory, 'coverage')), []);
      assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    }, 'test:coverage', directory => [`OUTPUT_DIR=${join(directory, 'coverage')}`], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never', SLICE_FIXTURE_COVERAGE_PROOF_MODE: mode });
  });
}

test('coverage rejects zero selected tests before an instrumented run', async () => {
  await withProcessFixture(0, async (result, state, directory) => {
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /inventory|selected no tests/i);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args[0] === 'llvm-cov' && call.args[1] === 'nextest'), false);
    assert.deepEqual(await readdir(join(directory, 'coverage')), []);
  }, 'test:coverage', directory => [`OUTPUT_DIR=${join(directory, 'coverage')}`], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never', SLICE_FIXTURE_COVERAGE_EMPTY_LIST: '1' });
});

test('separate coverage invocations reuse build directories but keep reports separate', async () => {
  const targets = [];
  const reports = [];
  for (let index = 0; index < 2; index += 1) {
    await withProcessFixture(0, (result, state) => {
      assert.equal(result.status, 0, result.stderr);
      const coverageCalls = state.calls.filter(call => call.command === 'cargo' && call.args[0] === 'llvm-cov' && call.args.includes('nextest'));
      assert.equal(new Set(coverageCalls.map(call => call.coverage_target_dir)).size, 1);
      assert.equal(new Set(coverageCalls.map(call => call.coverage_build_dir)).size, 1);
      targets.push(coverageCalls[0].coverage_target_dir);
      const report = state.calls.find(call => call.command === 'cargo' && call.args.includes('--output-path'));
      reports.push(report.args[report.args.indexOf('--output-path') + 1]);
    }, 'test:coverage', [], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never' });
  }
  assert.equal(targets[0], targets[1]);
  assert.notEqual(reports[0], reports[1]);
});

test('coverage refuses to clean or run tests while another invocation owns its build directory', async () => {
  const lockDirectory = join(root, 'target/coverage/instrumented');
  await mkdir(lockDirectory, { recursive: true });
  const release = await acquireCoverageLock(join(lockDirectory, 'run.flock'));
  try {
    await withProcessFixture(0, async (result, state) => {
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, /Coverage build directory is already in use/);
      assert.ok(!state.calls.some(call => call.command === 'docker' || call.args.includes('clean') || call.args.includes('--no-report')));
    }, 'test:coverage', [], 0, { SLICE_FIXTURE_FAIL_MATCH: 'never' });
  } finally {
    await release();
  }
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

test('coverage prerequisite rejects a different nextest version before provisioning', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /cargo-nextest 0\.9\.148/);
    assert.equal(state.calls.some(call => call.command === 'docker'), false);
  }, 'test:coverage', [], 0, { SLICE_FIXTURE_NEXTEST_VERSION: '0.9.149' });
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
