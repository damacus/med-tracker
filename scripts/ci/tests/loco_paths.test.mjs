import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { classify } from '../classify.mjs';

test('Loco source and framework configuration select the Loco job', () => {
  for (const path of ['src/app.rs', 'build.rs', 'Cargo.toml', 'Cargo.lock', 'migration/src/lib.rs', 'config/development.yaml', 'assets/views/home/index.html', 'scripts/migration/foundation.test.mjs']) {
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
