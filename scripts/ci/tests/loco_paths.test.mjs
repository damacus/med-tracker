import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { classify } from '../classify.mjs';

test('Loco source and framework configuration select the Loco job', () => {
  for (const path of ['src/app.rs', 'Cargo.toml', 'Cargo.lock', 'migration/src/lib.rs', 'config/development.yaml', 'assets/views/home/index.html', 'scripts/migration/foundation.test.mjs']) {
    const selected = classify([path]).selected;
    assert.equal(selected.loco, true, path);
    assert.equal(selected.rails, false, path);
  }
});

test('relocated Rails UI still requires browser quality checks', () => {
  assert.equal(classify(['rails/app/components/dashboard.rb']).selected.lighthouse, true);
  const selected = classify(['rails/app/models/medication.rb']).selected;
  assert.equal(selected.rails, true);
  assert.equal(selected.loco, false);
});

test('root persistence tests and SQL fixtures select only the Loco runtime', () => {
  for (const path of ['tests/persistence.rs', 'tests/fixtures/persistence.sql']) {
    const selected = classify([path]).selected;
    assert.equal(selected.loco, true, path);
    assert.equal(selected.rails, false, path);
    assert.equal(selected.rust_port, false, path);
  }
});

test('root browser tooling and dependencies select the Loco runtime', () => {
  for (const path of ['Taskfiles/browser-care.yml', 'Taskfiles/frontend.yml', 'playwright.config.mjs', 'package.json', 'package-lock.json']) {
    const selected = classify([path]).selected;
    assert.equal(selected.loco, true, path);
    assert.equal(selected.rails, false, path);
    assert.equal(selected.rust_port, false, path);
  }
});

test('advisory lint files remain classified as Loco tooling', () => {
  for (const path of ['Taskfiles/lint-advisory.yml', 'scripts/lint-advisory/clippy.toml', 'scripts/lint-advisory/report.mjs', 'scripts/lint-advisory/report.test.mjs']) {
    const selected = classify([path]).selected;
    assert.equal(selected.loco, true, path);
    assert.equal(selected.rails, false, path);
    assert.equal(selected.rust_port, false, path);
  }
});

test('Loco CI provisions the compiler components required by root checks', () => {
  const workflow = readFileSync(new URL('../../../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const job = workflow.match(/\n  loco_foundation:\n([\s\S]*?)(?=\n  [a-z_]+:\n)/)?.[1];
  assert.ok(job);
  assert.match(job, /components: rustfmt, clippy/);
  assert.match(job, /run: task ci/);
  assert.match(job, /image: postgres:18-alpine/);
});

test('Loco CI avoids historical audit setup during application verification', () => {
  const workflow = readFileSync(new URL('../../../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const job = workflow.match(/\n  loco_foundation:\n([\s\S]*?)(?=\n  [a-z_]+:\n)/)?.[1];
  assert.ok(job);
  assert.doesNotMatch(job, /apt-get|fetch-depth: 0/);
  assert.match(job, /run: task ci/);
});

test('dashboard fixtures validate layouts without a Fish dependency', () => {
  const repoRoot = fileURLToPath(new URL('../../..', import.meta.url));
  const taskPath = spawnSync('which', ['task'], { encoding: 'utf8' }).stdout.trim();
  assert.ok(taskPath);
  const directory = mkdtempSync(join(tmpdir(), 'dashboard-task-'));
  writeFileSync(join(directory, 'psql'), '#!' + process.execPath + '\nprocess.stdout.write(JSON.stringify(process.argv.slice(2)));\n', { mode: 0o755 });
  const run = variant => spawnSync(taskPath, [
    '--taskfile', 'Taskfiles/browser-care.yml', 'dashboard-variant',
    'CARE_DATABASE_URL=postgres://unused.invalid/fixture',
    'DASHBOARD_VARIANT=' + variant,
  ], { cwd: repoRoot, env: { ...process.env, PATH: directory }, encoding: 'utf8' });
  try {
    for (const variant of ['current', 'time_first', 'family_lanes', 'calm_focus']) {
      const result = run(variant);
      assert.equal(result.status, 0, result.stderr);
      const args = JSON.parse(result.stdout);
      assert.equal(args[0], 'postgres://unused.invalid/fixture');
      assert.ok(args.at(-1).includes("jsonb_build_object('dashboard_variant','" + variant + "')"));
    }
    for (const variant of ['', 'unknown', "current'); SELECT 1; --"]) {
      const result = run(variant);
      assert.notEqual(result.status, 0);
      assert.equal(result.stdout, '', 'invalid layout must not reach psql');
    }
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
