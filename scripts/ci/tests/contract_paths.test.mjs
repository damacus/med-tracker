import assert from 'node:assert/strict';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import test from 'node:test';

test('policy workflow installs Fish before running Fish-based ownership tests', () => {
  const workflow = readFileSync('.github/workflows/ci.yml', 'utf8');
  const job = workflow.slice(workflow.indexOf('  policy_check:'), workflow.indexOf('  markdown_check:'));
  assert.match(job, /apt-get install[^\n]*\bfish\b/, 'The policy job cannot execute the real Fish runner tests');
  assert.match(job, /apt-get install[^\n]*\buuid-runtime\b/, 'The policy job cannot generate owned runner project IDs');
  assert.ok(job.indexOf('apt-get install') < job.indexOf('task ci:check'));
});

test('contract runner prepares writable Rails tmp before container startup in a clean checkout', () => {
  const root = process.cwd();
  const fixture = mkdtempSync(join(tmpdir(), 'medtracker-clean-contract-'));
  const executable = join(fixture, 'rtk');
  mkdirSync(join(fixture, 'rails'));
  writeFileSync(executable, `#!/usr/bin/env node
const fs = require('node:fs'), cp = require('node:child_process');
const args = process.argv.slice(2);
if (args[0] === 'proxy') {
  const result = cp.spawnSync(args[1], args.slice(2), { stdio: 'inherit' });
  process.exit(result.status ?? 99);
}
if (args[0] === 'task' && args[1] === 'contract:prepare-db') {
  try { fs.writeFileSync('rails/tmp/local_secret.txt', 'Synthetic writable-directory probe'); }
  catch { process.exit(43); }
  process.exit(42);
}
process.exit(0);
`);
  chmodSync(executable, 0o755);
  try {
    const result = spawnSync('fish', ['--no-config', join(root, 'rust/contract-tests/run.fish'), 'rails', 'doses'], {
      cwd: fixture, env: { ...process.env, PATH: `${fixture}:${process.env.PATH}` }, encoding: 'utf8', timeout: 15000
    });
    assert.equal(result.status, 42, `Rails tmp was not prepared before startup: ${result.stderr}`);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});

test('contract runner emits absolute authoritative screenshot binds for both Rust browser branches', () => {
  const root = process.cwd();
  const fixture = mkdtempSync(join(tmpdir(), 'medtracker-screenshot-paths-'));
  const capture = join(fixture, 'capture.json');
  const executable = join(fixture, 'rtk');
  writeFileSync(executable, `#!/usr/bin/env node
const fs = require('node:fs'), path = require('node:path'), cp = require('node:child_process');
const args = process.argv.slice(2);
if (args[0] === 'proxy') {
  if (args[1] === 'docker') process.exit(98);
  const result = cp.spawnSync(args[1], args.slice(2), { stdio: 'inherit' });
  process.exit(result.status ?? 99);
}
if (args[0] !== 'task') process.exit(97);
if (args[1] === 'api:contract-source-snapshot') {
  const destination = args.find(arg => arg.startsWith('CONTRACT_SOURCE_DIR=')).slice('CONTRACT_SOURCE_DIR='.length);
  fs.mkdirSync(destination, { recursive: true });
  fs.writeFileSync(path.join(destination, 'synthetic.txt'), 'Synthetic source snapshot');
}
if (args[1] === 'contract:prepare-db') {
  fs.writeFileSync(process.env.CONTRACT_PATH_CAPTURE, JSON.stringify({ screenshots: process.env.CONTRACT_RUST_BROWSER_SCREENSHOT_DIR }));
  process.exit(42);
}
`);
  chmodSync(executable, 0o755);
  try {
    for (const [selector, directory] of [['browser-dashboard-rust', 'dashboard-rust'], ['browser-journey-rust', 'journey-medication-rust']]) {
      const result = spawnSync('fish', ['--no-config', 'rust/contract-tests/run.fish', 'rails', selector], {
        cwd: root,
        env: { ...process.env, PATH: `${fixture}:${process.env.PATH}`, CONTRACT_PATH_CAPTURE: capture },
        encoding: 'utf8',
        timeout: 15000
      });
      assert.equal(result.status, 42, result.stdout + result.stderr);
      assert.equal(JSON.parse(readFileSync(capture, 'utf8')).screenshots, join(root, 'docs/screenshots', directory));
    }
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});

function withShimFixture(callback) {
  const root = process.cwd();
  const fixture = mkdtempSync(join(tmpdir(), 'medtracker-real-shim-'));
  mkdirSync(join(root, 'tmp/contract-tests'), { recursive: true });
  const run = mkdtempSync(join(root, 'tmp/contract-tests/run.'));
  const latest = join(fixture, 'latest-run');
  const project = 'mtcontract-1234567890abcdef';
  writeFileSync(latest, relative(root, run));
  writeFileSync(join(run, 'owner'), `${project}\n`);
  const environment = { ...process.env, CONTRACT_FAKE_RUN_DIR_FILE: latest, CONTRACT_FAKE_FAIL_STEP: '', CONTRACT_FAKE_FAIL_AT: '', CONTRACT_FAKE_REQUIRE_RELATIVE_CLEANUP: '0', CONTRACT_FIXTURE_DIR: fixture };
  const invoke = (name, extra = {}) => spawnSync('fish', ['--no-config', 'rust/contract-tests/test_support/rtk', 'task', name, `CONTRACT_PROJECT=${project}`], { cwd: root, env: { ...environment, ...extra }, encoding: 'utf8', timeout: 15000 });
  try { callback({ root, fixture, run, invoke }); } finally {
    rmSync(run, { recursive: true, force: true });
    rmSync(fixture, { recursive: true, force: true });
  }
}

test('ownership shim provisions the fixture for the namespaced Rails task', () => withShimFixture(({ run, invoke }) => {
  const result = invoke('rails:test:exec');
  assert.equal(result.status, 0, result.stderr);
  assert.ok(existsSync(join(run, 'fixture.json')), 'Namespaced Rails execution skipped fixture provisioning');
}));

test('ownership shim returns a usable port for the namespaced Rails task', () => withShimFixture(({ invoke }) => {
  const result = invoke('rails:test:port');
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim(), '43017');
}));

test('ownership shim accepts the relocated captured API source', () => withShimFixture(({ fixture, invoke }) => {
  for (const directory of ['rust/api', 'rails/config', 'rails/vendor/fonts']) mkdirSync(join(fixture, directory), { recursive: true });
  for (const file of ['rust/api/Cargo.toml', 'rails/config/nhs_dmd_curated_products.yml', 'rails/vendor/fonts/OFL-1.1.txt']) writeFileSync(join(fixture, file), 'Synthetic source input');
  const key = execFileSync('openssl', ['genpkey', '-algorithm', 'EC', '-pkeyopt', 'ec_paramgen_curve:prime256v1'], { encoding: 'utf8' });
  const result = invoke('api:contract-up', { CONTRACT_API_BUILD_CONTEXT: fixture, CONTRACT_APNS_PRIVATE_KEY: key });
  assert.equal(result.status, 0, `Relocated snapshot was rejected with ${result.status}: ${result.stderr}`);
}));

test('ownership shim accepts absolute screenshot and composed runner paths', () => withShimFixture(({ root, invoke }) => {
  const result = invoke('api:contract-browser-rust', {
    CONTRACT_RUST_BROWSER_SCREENSHOT_DIR: join(root, 'docs/screenshots/journey-medication-rust'),
    COMPOSE_FILE: `${join(root, 'rails/compose.yaml')}:${join(root, 'rust/contract-tests/runner.compose.yaml')}`,
    CONTRACT_FAKE_REQUIRE_BROWSER_SNAPSHOT: '0'
  });
  assert.equal(result.status, 0, `Absolute browser inputs were rejected with ${result.status}: ${result.stderr}`);
}));

test('dashboard ownership shim rejects a wrong screenshot bind', () => withShimFixture(({ root, invoke }) => {
  const result = invoke('api:contract-browser-dashboard-rust', {
    CONTRACT_RUST_BROWSER_SCREENSHOT_DIR: join(root, 'docs/screenshots/journey-medication-rust'),
    COMPOSE_FILE: join(root, 'rust/contract-tests/runner.compose.yaml')
  });
  assert.equal(result.status, 47, 'Dashboard branch silently skipped screenshot validation');
}));

test('dashboard ownership shim validates its captured browser source', () => withShimFixture(({ root, fixture, invoke }) => {
  const web = join(fixture, 'rust/web');
  const trace = join(fixture, 'trace');
  mkdirSync(join(web, 'tests'), { recursive: true });
  writeFileSync(join(web, 'Dockerfile.smoke'), 'Synthetic Dockerfile');
  writeFileSync(join(web, 'tests/example.mjs'), 'Synthetic browser source');
  const result = invoke('api:contract-browser-dashboard-rust', {
    CONTRACT_RUST_BROWSER_SCREENSHOT_DIR: join(root, 'docs/screenshots/dashboard-rust'),
    COMPOSE_FILE: join(root, 'rust/contract-tests/runner.compose.yaml'),
    CONTRACT_API_BUILD_CONTEXT: fixture,
    CONTRACT_BROWSER_BUILD_CONTEXT: web,
    CONTRACT_FAKE_REQUIRE_BROWSER_SNAPSHOT: '1',
    CONTRACT_FAKE_TRACE: trace,
    BROWSER_TEST_FILES: 'tests/example.mjs'
  });
  assert.equal(result.status, 0, result.stderr);
  assert.ok(existsSync(trace), 'Dashboard branch skipped captured-source validation');
  assert.match(readFileSync(trace, 'utf8'), /browser-source-snapshot-verified/);
}));
