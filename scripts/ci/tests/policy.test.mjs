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

for (const [suite, job] of [['lighthouse', 'lighthouse'], ['rails', 'test_non_system'], ['rails', 'coverage']]) {
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

test('classification failure cannot produce a successful gate', () => {
  const needs = needsFor('rails');
  needs.changes.result = 'failure';
  assert.ok(evaluate(needs).includes('Change classification must succeed'));
});
