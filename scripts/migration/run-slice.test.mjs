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

const root = fileURLToPath(new URL('../../', import.meta.url));
const ambientUrl = 'postgres://unowned.invalid:1/unowned';

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
  state.child_identity_key_synthetic = process.env.MEDTRACKER_SESSION_KEY === Buffer.alloc(64, 7).toString('base64');
  state.child_verification_key_synthetic = process.env.RAILS_SECRET_KEY_BASE === 'synthetic-slice-rails-verification-secret';
  state.child_compose_environment_removed = ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES'].every(name => process.env[name] === undefined);
  state.child_docker_environment_removed = ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH'].every(name => process.env[name] === undefined);
  if (process.env.DATABASE_URL === ${JSON.stringify(ambientUrl)}) state.foreign_resources_touched += 1;
  state.calls.push({ command, args, rust_test_nocapture: process.env.RUST_TEST_NOCAPTURE, coverage_target_dir: process.env.CARGO_TARGET_DIR, coverage_build_dir: process.env.CARGO_BUILD_BUILD_DIR, coverage_profile_override: process.env.LLVM_PROFILE_FILE, llvm_cov_target_override: process.env.CARGO_LLVM_COV_TARGET_DIR, llvm_cov_build_override: process.env.CARGO_LLVM_COV_BUILD_DIR, llvm_cov_setup_override: process.env.CARGO_LLVM_COV_SETUP, llvm_cov_deprecated_flags_override: process.env.CARGO_LLVM_COV_FLAGS, llvm_profdata_deprecated_flags_override: process.env.CARGO_LLVM_PROFDATA_FLAGS, llvm_cov_tool_override: process.env.LLVM_COV, llvm_profdata_tool_override: process.env.LLVM_PROFDATA, llvm_cov_flags_override: process.env.LLVM_COV_FLAGS, llvm_profdata_flags_override: process.env.LLVM_PROFDATA_FLAGS, llvm_profile_name_override: process.env.LLVM_PROFILE_FILE_NAME });
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
    if (process.env.SLICE_FIXTURE_EXTRA_REPORT_PATH) fs.writeFileSync(process.env.SLICE_FIXTURE_EXTRA_REPORT_PATH, 'unrelated source change\\n');
  }
  const complete = () => {
    const transcript = args.includes('--include-ignored') && process.env.SLICE_FIXTURE_COMPLETION_TRANSCRIPT
      ? process.env.SLICE_FIXTURE_COMPLETION_TRANSCRIPT : args.includes('care_api') && process.env.SLICE_FIXTURE_CARE_TRANSCRIPT
        ? process.env.SLICE_FIXTURE_CARE_TRANSCRIPT : process.env.SLICE_FIXTURE_CARGO_TRANSCRIPT;
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
      env: { ...process.env, PATH: `${directory}:${process.env.PATH}`, DATABASE_URL: ambientUrl, MEDTRACKER_SESSION_KEY: 'ambient-identity-fixture-key-must-not-be-used', COMPOSE_FILE: '/unowned/compose.yaml', COMPOSE_PROJECT_NAME: 'unrelated', DOCKER_HOST: 'tcp://unowned.invalid:2376', DOCKER_CONTEXT: 'unrelated', DOCKER_CONFIG: '/unowned/docker-config', DOCKER_TLS_VERIFY: '1', DOCKER_CERT_PATH: '/unowned/certs', SLICE_FILTER: 'unrelated_filter', SLICE_FIXTURE_STATE: statePath, SLICE_FIXTURE_EXIT: String(exitCode), SLICE_FIXTURE_PROVISION_EXIT: String(provisionExit), CARGO_TARGET_DIR: '/unowned/target', CARGO_BUILD_BUILD_DIR: '/unowned/intermediates', LLVM_PROFILE_FILE: '/unowned/profile.profraw', CARGO_LLVM_COV_TARGET_DIR: '/unowned/llvm-cov-target', CARGO_LLVM_COV_BUILD_DIR: '/unowned/llvm-cov-build', CARGO_LLVM_COV_SETUP: 'skip', CARGO_LLVM_COV_FLAGS: '--unowned', CARGO_LLVM_PROFDATA_FLAGS: '--unowned', LLVM_COV: '/unowned/llvm-cov', LLVM_PROFDATA: '/unowned/llvm-profdata', LLVM_COV_FLAGS: '--unowned', LLVM_PROFDATA_FLAGS: '--unowned', LLVM_PROFILE_FILE_NAME: 'unowned.profraw', ...(taskName === 'slice:contracts' && taskAssignments.includes('GROUP=api-v1') ? { SLICE_FIXTURE_CARE_TRANSCRIPT: requestTranscript(creditedCareTests, []) } : {}), ...extraEnvironment, ...(gatePath ? { SLICE_FIXTURE_CARGO_GATE: gatePath } : {}), ...(preloadPath ? { NODE_OPTIONS: `${process.env.NODE_OPTIONS ?? ''} --import=${preloadPath}`.trim() } : {}) },
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

const creditedRequestTests = [
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
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--include-ignored')), false);
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
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--include-ignored')), false);
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
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--include-ignored')), false);
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
    assert.equal(cargo.some(call => call.args.includes('--include-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
    SLICE_FIXTURE_CARE_TRANSCRIPT: requestTranscript(creditedCareTests.slice(0, -1), []),
  });
});

test('API v1 completion rejects all credited lines without a final libtest summary and cleans up', async () => {
  const transcript = [
    `running ${creditedRequestTests.length} tests`,
    ...creditedRequestTests.map(name => `test ${name} ... ok`),
    '',
  ].join('\n');
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /summary|incomplete|transcript/i);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--include-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, { SLICE_FIXTURE_CARGO_TRANSCRIPT: transcript });
});

test('API v1 completion rejects an inconsistent libtest summary and cleans up', async () => {
  const transcript = requestTranscript(creditedRequestTests)
    .replace(`${creditedRequestTests.length} passed; 0 failed; 1 ignored`, `${creditedRequestTests.length + 1} passed; 0 failed; 1 ignored`);
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /summary|count|transcript/i);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--include-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, { SLICE_FIXTURE_CARGO_TRANSCRIPT: transcript });
});

test('API v1 completion bounds libtest stdout and cleans up an oversized transcript', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /limit|large|budget|size/i);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--include-ignored')), false);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: 'running 1 tests\n',
    SLICE_FIXTURE_CARGO_OVERFLOW_BYTES: String(1024 * 1024 + 1),
  });
});

test('closed contract output stops Cargo and cleans the owned database', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /EPIPE|broken pipe/i);
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'exec', 'down']);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    assert.equal(state.calls.filter(call => call.command === 'cargo').length, 1);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--include-ignored')), false);
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
  }, { closeOutputBeforeCargo: true });
});

test('readable Cargo output error cleans the owned database', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    assert.match(result.stderr, /synthetic EIO/);
    assert.deepEqual(state.calls.filter(call => call.command === 'docker').map(call => call.operation), ['up', 'port', 'exec', 'down']);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
    assert.equal(state.calls.filter(call => call.command === 'cargo').length, 1);
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--include-ignored')), false);
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
  }, { injectReadableError: true });
});

test('API v1 completion observes exact credited cases and runs the existing acceptance gate', async () => {
  await withProcessFixture(42, (result, state) => {
    assert.notEqual(result.status, 0, result.stderr);
    const cargo = state.calls.filter(call => call.command === 'cargo');
    assert.deepEqual(cargo.map(call => call.args[call.args.indexOf('--test') + 1]), ['requests', 'care_api', 'requests']);
    const requests = cargo.filter(call => call.args.includes('requests'));
    assert.ok(requests.some(call => !call.args.includes('--include-ignored')));
    assert.deepEqual(requests[0].args,
      ['test', '--locked', '--test', 'requests', 'api_v1', '--', '--format=pretty', '--color=never']);
    assert.deepEqual(cargo[1].args,
      ['test', '--locked', '--test', 'care_api', '--', '--format=pretty', '--color=never']);
    assert.deepEqual(requests.at(-1).args,
      ['test', '--locked', '--test', 'requests', 'api_v1', '--', '--include-ignored', '--format=pretty', '--color=never']);
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: requestTranscript(creditedRequestTests),
    SLICE_FIXTURE_FAIL_MATCH: '--include-ignored',
  });
});

test('API v1 completion requires its exact gate case to pass in the final selection', async () => {
  for (const [label, completion] of [
    ['absent', requestTranscript(creditedRequestTests, [])],
    ['renamed', requestTranscript([...creditedRequestTests, 'api_v1::acceptance_renamed'], [])],
    ['ignored', requestTranscript(creditedRequestTests)],
    ['zero', 'running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n'],
  ]) {
    await withProcessFixture(0, (result, state) => {
      assert.notEqual(result.status, 0, label);
      assert.match(result.stderr, /Missing executed API v1 acceptance gate/, label);
      assert.equal(state.calls.filter(call => call.command === 'cargo').length, 3, label);
      assert.ok(state.calls.at(-2).args.includes('--include-ignored'), label);
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
    assert.ok(requests.at(-1).args.includes('--include-ignored'));
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
    assert.ok(requests.at(-1).args.includes('--include-ignored'));
    assert.deepEqual(state.resources, { unrelated: ['existing-volume'] });
  }, 'slice:contracts', ['GROUP=api-v1'], 0, {
    SLICE_FIXTURE_CARGO_TRANSCRIPT: transcript,
    SLICE_FIXTURE_FAIL_MATCH: '--include-ignored',
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
    assert.equal(state.calls.some(call => call.command === 'cargo' && call.args.includes('--include-ignored')), false);
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
    SLICE_FIXTURE_FAIL_MATCH: '--include-ignored',
  });
  assert.equal(process.env.RUST_TEST_NOCAPTURE, parentValue);
});

test('ordinary slice tests retain the developer RUST_TEST_NOCAPTURE setting', async () => {
  await withProcessFixture(0, (result, state) => {
    assert.equal(result.status, 0, result.stderr);
    assert.equal(state.calls.find(call => call.command === 'cargo').rust_test_nocapture, '1');
  }, 'slice:test', ['TARGET=persistence'], 0, { RUST_TEST_NOCAPTURE: '1' });
});

test('libtest execution proof handles split names and summaries without crediting ignored cases', async () => {
  const { LibtestProof } = await import('./run-slice.mjs');
  assert.equal(typeof LibtestProof, 'function');
  const proof = new LibtestProof();
  for (const chunk of [
    Buffer.from('running 2 tests\ntest api_v1::baseline::anony'),
    Buffer.from('mous_access ... ok\ntest api_v1::acceptance ... ign'),
    Buffer.from('ored\n\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.'),
    Buffer.from('00s\n'),
  ]) proof.write(chunk);
  const result = proof.finish();
  assert.deepEqual([...result.passed], ['api_v1::baseline::anonymous_access']);
  assert.deepEqual([...result.ignored], ['api_v1::acceptance']);
});

test('libtest execution proof recognises a passing should-panic case', async () => {
  const { LibtestProof } = await import('./run-slice.mjs');
  const proof = new LibtestProof();
  proof.write(Buffer.from('running 1 test\ntest contract::tests::contract_helper_rejects_duplicate_items - should panic ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n'));
  assert.deepEqual([...proof.finish().passed], ['contract::tests::contract_helper_rejects_duplicate_items']);
});

test('libtest execution proof rejects a summary inconsistent with split case lines', async () => {
  const { LibtestProof } = await import('./run-slice.mjs');
  assert.equal(typeof LibtestProof, 'function');
  const proof = new LibtestProof();
  proof.write(Buffer.from('running 1 tests\ntest api_v1::baseline::anonymous_access ... ok\n'));
  proof.write(Buffer.from('test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n'));
  assert.throws(() => proof.finish(), /summary|count|transcript/i);
});

test('libtest execution proof rejects truncated or appended summary text', async () => {
  const { LibtestProof } = await import('./run-slice.mjs');
  for (const summary of [
    'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;',
    'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s extra',
  ]) {
    const proof = new LibtestProof();
    proof.write(Buffer.from(`running 1 test\ntest api_v1::baseline::anonymous_access ... ok\n${summary}\n`));
    assert.throws(() => proof.finish(), /summary|transcript|incomplete/i, summary);
  }
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
