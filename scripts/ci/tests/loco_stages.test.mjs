import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { existsSync, mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

test('hosted relocation checks run after their tools are installed', () => {
  const workflow = readFileSync('.github/workflows/ci.yml', 'utf8').split('  loco_foundation:')[1].split('  loco_lint:')[0];
  const install = workflow.indexOf('sudo apt-get install --yes fish uuid-runtime ripgrep');
  assert.ok(install >= 0, 'Install every relocation and runner prerequisite');
  assert.ok(install < workflow.indexOf('run: task ci:paths foundation:test-relocation fmt'));
});

test('hosted archive workers install the pinned runner before Task and lint installs machete', () => {
  const workflow = readFileSync('.github/workflows/ci.yml', 'utf8');
  const pinned = readFileSync('mise.toml', 'utf8');
  const nextest = pinned.match(/"cargo:cargo-nextest" = "([^"]+)"/)?.[1];
  const machete = pinned.match(/"cargo:cargo-machete" = "([^"]+)"/)?.[1];
  for (const [jobName, nextJob] of [['loco_foundation', 'loco_lint'], ['loco_coverage', 'loco_integration'], ['loco_integration', 'loco_browser']]) {
    const job = workflow.split(`  ${jobName}:`)[1].split(`  ${nextJob}:`)[0];
    assert.ok(job.indexOf(`cargo-nextest@${nextest}`) >= 0);
    const task = jobName === 'loco_foundation' ? 'run: task test:unit' : jobName === 'loco_coverage' ? 'run: task test:coverage' : 'run: task test:integration';
    assert.ok(job.indexOf(`cargo-nextest@${nextest}`) < job.indexOf(task));
    assert.match(job, /taiki-e\/install-action@9983c65e42da123ff25d1f78505eb6de315aa172/);
  }
  const lint = workflow.split('  loco_lint:')[1].split('  loco_coverage:')[0];
  assert.ok(lint.includes(`cargo-machete@${machete}`));
});

test('root lint checks dependency advisories, bans and sources with the pinned deny tool', () => {
  const plan = spawnSync('task', ['--dry', '--force', '--verbose', 'lint'], { encoding: 'utf8' });
  assert.equal(plan.status, 0, plan.stderr);
  const commands = plan.stdout + plan.stderr;
  const clippy = commands.indexOf('cargo clippy --locked --workspace --all-targets --all-features -- -D warnings');
  const machete = commands.indexOf('scripts/migration/check-unused-deps.mjs');
  const deny = commands.indexOf('cargo deny --workspace --locked --all-features check advisories bans sources');
  assert.ok(clippy >= 0 && machete > clippy && deny > machete, 'Root lint must run Clippy, machete and deny in order');

  const workflow = readFileSync('.github/workflows/ci.yml', 'utf8');
  const lint = workflow.split('  loco_lint:')[1].split('  loco_coverage:')[0];
  const version = readFileSync('mise.toml', 'utf8').match(/^"cargo:cargo-deny" = "([^"]+)"$/m)?.[1];
  assert.ok(version, 'mise.toml must pin cargo-deny');
  const install = lint.indexOf(`cargo-deny@${version}`);
  assert.ok(install >= 0 && install < lint.indexOf('run: task fmt lint'), 'Hosted lint must install pinned deny before Task');
});

test('the owned populated-state cutover journey uses nextest with an exact nonempty target', () => {
  const plan = spawnSync('task', ['--dry', '--force', '--verbose', 'release:cutover:loco-journey', 'CUTOVER_DATABASE_URL=postgres://medtracker:medtracker_password@127.0.0.1:5432/medtracker_reference', 'MEDTRACKER_OWNED_DATABASE=1'], { encoding: 'utf8' });
  assert.equal(plan.status, 0, plan.stderr);
  const commands = plan.stdout + plan.stderr;
  assert.match(commands, /cargo nextest run --locked --features cutover-rehearsal --test cutover --no-tests=fail/);
  assert.doesNotMatch(commands, /cargo test --locked --features cutover-rehearsal --test cutover/);
});

test('avatar bucket utility selects its ignored care API test from source or a validated archive', () => {
  const selector = 'binary_id(=med-tracker::care_api) & test(=avatar_storage::prepare_owned_bucket)';
  const source = spawnSync('task', ['--dry', '--force', '--verbose', 'browser-care:prepare-avatar-storage'], { encoding: 'utf8' });
  assert.equal(source.status, 0, source.stderr);
  const sourceCommands = source.stdout + source.stderr;
  assert.ok(sourceCommands.includes(`cargo nextest run --locked --test care_api --run-ignored only --no-tests=fail -E '${selector}'`), sourceCommands);
  assert.doesNotMatch(sourceCommands, /cargo test --locked/);

  const prebuilt = spawnSync('task', ['--dry', '--force', '--verbose', 'browser-care:prepare-avatar-storage', 'LOCO_PREBUILT=1'], { encoding: 'utf8' });
  assert.equal(prebuilt.status, 0, prebuilt.stderr);
  const prebuiltCommands = prebuilt.stdout + prebuilt.stderr;
  const validation = prebuiltCommands.indexOf('node scripts/ci/loco-validate.mjs');
  const archive = prebuiltCommands.indexOf('cargo nextest run --archive-file target/ci/loco-tests.tar.zst --workspace-remap ');
  assert.ok(validation >= 0 && archive > validation, prebuiltCommands);
  assert.ok(prebuiltCommands.includes(`--run-ignored only --no-tests=fail -E '${selector}'`), prebuiltCommands);
  assert.doesNotMatch(prebuiltCommands, /\.\/target\/ci\/care_api|cargo test --locked/);
});

test('legacy household shards wait for Loco browsers without skipping legacy-only changes', () => {
  const workflow = readFileSync('.github/workflows/ci.yml', 'utf8');
  const job = workflow.match(/\n  rust_household_browser:\n([\s\S]*?)(?=\n  [a-z_]+:\n)/)?.[1];
  const dependencies = job.match(/needs: \[([^\]]+)\]/)?.[1].split(',').map(value => value.trim());
  assert.ok(dependencies?.includes('changes'));
  assert.ok(dependencies.includes('loco_browser'), 'Reserve runners for the Loco chain before the legacy matrix');
  const condition = job.match(/if: \$\{\{ (.+) \}\}/)?.[1];
  const selected = new Function('needs', 'cancelled', `return (${condition});`);
  for (const result of ['success', 'skipped', 'failure', 'cancelled']) {
    const needs = { changes: { outputs: { rust_port: 'true', loco: 'false' } }, loco_browser: { result } };
    assert.equal(selected(needs, () => false), ['success', 'skipped'].includes(result), result);
    needs.changes.outputs.loco = 'true';
    assert.equal(selected(needs, () => false), result === 'success', 'Do not run legacy browsers after Loco unit failure');
    assert.equal(selected(needs, () => true), false, 'Do not start work after workflow cancellation');
    needs.changes.outputs.rust_port = 'false';
    assert.equal(selected(needs, () => false), false, 'Unselected legacy suites stay skipped');
  }
});

test('CI runs cheap contracts and units before preparation and integration or browsers', () => {
  const result = spawnSync('task', ['--dry', '--force', '--verbose', 'ci'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const output = result.stdout + result.stderr;
  const unit = output.indexOf('cargo nextest run --locked --workspace --lib --no-tests=fail');
  const prepare = output.indexOf('loco-build.mjs');
  assert.ok(unit >= 0, 'Library tests must be an explicit first stage');
  assert.ok(prepare > unit, 'Compile integration targets only after units');
  assert.ok(output.indexOf('tracked_paths.mjs') < unit);
  assert.ok(output.indexOf('relocation.test.mjs') < unit);
  assert.ok(output.indexOf('loco-integration.mjs') > prepare);
  assert.ok(output.indexOf('loco-browser.mjs') > prepare);
});

test('a real Task invocation stops before preparation or acceptance when units fail', () => {
  const directory = mkdtempSync(join(tmpdir(), 'loco-stage-failure-'));
  const log = join(directory, 'calls');
  const stub = `#!${process.execPath}
const fs = require('node:fs');
fs.appendFileSync(process.env.LOCO_STAGE_LOG, process.argv.slice(1).join(' ') + '\\n');
if (process.argv.includes('--lib')) process.exit(42);
`;
  try {
    for (const name of ['node', 'cargo']) writeFileSync(join(directory, name), stub, { mode: 0o755 });
    const result = spawnSync('task', ['--force', 'test'], { encoding: 'utf8', env: { ...process.env, PATH: `${directory}:${process.env.PATH}`, LOCO_STAGE_LOG: log } });
    assert.notEqual(result.status, 0);
    assert.ok(existsSync(log), result.stdout + result.stderr);
    const calls = readFileSync(log, 'utf8');
    assert.match(calls, /cargo nextest run --locked --workspace --lib --no-tests=fail/);
    assert.doesNotMatch(calls, /loco-build|loco-integration|loco-browser/);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
