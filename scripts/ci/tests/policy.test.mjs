import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { classify, policy } from '../classify.mjs';
import { evaluate } from '../gate.mjs';

function needsFor(suite) {
  const outputs = Object.fromEntries(Object.keys(policy.jobs).map(name => [name, String(name === suite)]));
  const jobs = Object.fromEntries(Object.values(policy.jobs).flat().map(name => [name, { result: 'skipped' }]));
  for (const job of policy.jobs[suite]) jobs[job] = { result: 'success' };
  return { changes: { result: 'success', outputs }, ...jobs };
}

test('workflow exports every selection required by the gate', () => {
  const workflow = readFileSync(new URL('../../../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const outputBlock = workflow.match(/\n  changes:\n[\s\S]*?\n    outputs:\n(?<outputs>(?:      [^\n]+\n)+)    steps:/)?.groups?.outputs;
  assert.ok(outputBlock, 'changes job outputs were not found');

  const outputs = new Set([...outputBlock.matchAll(/^      ([a-z_]+):/gm)].map(match => match[1]));
  const missingOutputs = Object.keys(policy.jobs).filter(suite => !outputs.has(suite));
  assert.deepEqual(missingOutputs, []);
});

test('UI changes select Lighthouse while model changes select only Rails', () => {
  assert.equal(classify(['app/components/dashboard.rb']).selected.lighthouse, true);
  const model = classify(['app/models/medication.rb']).selected;
  assert.equal(model.rails, true);
  assert.equal(model.lighthouse, false);
});

test('Rust port and contract infrastructure select their executable checks', () => {
  for (const path of ['rust/api/src/lib.rs', 'rust/web/src/lib.rs', 'rust/ui-preview/src/lib.rs', 'rust/contract-tests/tests/openapi_dosage_options.rs']) {
    const selected = classify([path]).selected;
    assert.equal(selected.rust_port, true, path);
    assert.equal(selected.rails, false, path);
  }
  for (const path of ['Taskfiles/contract.yml', 'scripts/contract_provision.rb', 'scripts/compose_port.fish']) {
    const selected = classify([path]).selected;
    assert.equal(selected.rust_port, true, path);
    assert.equal(selected.rails, true, path);
  }
});

for (const [suite, job] of [['lighthouse', 'lighthouse'], ['rails', 'test_non_system'], ['rails', 'coverage'], ['rust_port', 'rust_port'], ['rust_port', 'rust_dashboard_browser']]) {
  test(`${suite} succeeds when every selected job succeeds`, () => {
    assert.deepEqual(evaluate(needsFor(suite)), []);
  });

  for (const result of ['failure', 'cancelled', 'skipped', undefined]) {
    test(`${suite} fails closed when ${job} is ${result ?? 'missing'}`, () => {
      const needs = needsFor(suite);
      if (result) needs[job] = { result };
      else delete needs[job];
      assert.ok(evaluate(needs).some(error => error.includes(job)));
    });
  }
}

test('Rust CI builds the UI and runs both dashboard browser suites', () => {
  const workflow = readFileSync(new URL('../../../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const taskfile = readFileSync(new URL('../../../Taskfiles/ci.yml', import.meta.url), 'utf8');
  const browserTask = readFileSync(new URL('../../../rust/api/Taskfile.yml', import.meta.url), 'utf8');
  for (const command of ['fmt', 'test', 'lint', 'build']) {
    assert.ok(taskfile.includes(`task -d rust/ui-preview ${command}`), command);
  }
  assert.match(browserTask, /node --test tests\/dashboard\.test\.mjs tests\/leptodon-dashboard\.test\.mjs/);
  assert.match(workflow, /\n  rust_dashboard_browser:\n/);
  assert.match(workflow, /task api:browser-dashboard-rust/);
  assert.match(workflow, /\n      - rust_dashboard_browser\n/);
});

test('classification failure cannot produce a successful gate', () => {
  const needs = needsFor('rails');
  needs.changes.result = 'failure';
  assert.ok(evaluate(needs).includes('Change classification must succeed'));
});
