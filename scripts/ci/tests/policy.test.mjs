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

for (const [suite, job] of [['lighthouse', 'lighthouse'], ['rails', 'test_non_system'], ['rails', 'coverage'], ['rust_port', 'rust_port'], ['rust_port', 'rust_dashboard_browser'], ['rust_port', 'rust_household_browser']]) {
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

test('isolated browser runner matches the checkout owner and reports migration failure', () => {
  const workflow = readFileSync(new URL('../../../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const compose = readFileSync(new URL('../../../compose.yaml', import.meta.url), 'utf8');
  const runner = readFileSync(new URL('../../../rust/contract-tests/run.fish', import.meta.url), 'utf8');
  assert.match(workflow, /TEST_IMAGE_UID=\$\(id -u\) task api:browser-dashboard-rust/);
  assert.equal((compose.match(/UID: \$\{TEST_IMAGE_UID:-1000\}/g) ?? []).length, 2);
  assert.match(runner, /docker compose -p \$contract_project --profile test logs --no-color --tail=80 migrate-test/);
});

test('household journeys run in CI with isolated permission and calendar fixtures', () => {
  const workflow = readFileSync(new URL('../../../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const job = workflow.match(/\n  rust_household_browser:\n(?<body>[\s\S]*?)(?=\n  [\w-]+:\n)/)?.groups?.body;
  assert.ok(job, 'household browser job was not found');
  assert.ok(policy.jobs.rust_port.includes('rust_household_browser'));
  assert.match(workflow, /\n      - rust_household_browser\n/);
  assert.match(job, /fail-fast: false/);
  assert.match(job, /TEST_IMAGE_UID=\$\(id -u\) task api:browser-rust/);
  for (const name of ['household_treatment_acceptance', 'household_treatment_permissions', 'household_final_acceptance', 'household_minor_readiness']) {
    assert.ok(job.includes(name), name);
  }
  for (const name of ['household-routes', 'household-workflows', 'household-completion-medication', 'household-completion-locales', 'household-dosage-options', 'household-stock', 'household-treatments', 'household-treatment-administration', 'household-treatment-calendar', 'household-taper-times', 'household-inventory-completion', 'household-minor-readiness']) {
    assert.ok(job.includes(`tests/${name}.test.mjs`), name);
  }
  assert.match(job, /clock: "2026-03-29T00:30:00Z"/);
  assert.match(job, /CONTRACT_DASHBOARD_NOW: \$\{\{ matrix\.clock \}\}/);
  const rows = new Map([...job.matchAll(/^          - journey: ([\w-]+)\n([\s\S]*?)(?=^          - journey:|^    env:)/gm)]
    .map(match => [match[1], match[2]]));
  for (const name of ['calendar', 'taper-times']) {
    assert.match(rows.get(name), /acceptance: "false"/);
    assert.match(rows.get(name), /clock: "2026-03-29T00:30:00Z"/);
    assert.doesNotMatch(rows.get(name), /target:/);
  }
  assert.match(rows.get('treatments'), /target: household_treatment_acceptance/);
  assert.match(rows.get('treatments'), /tests\/household-treatment-administration\.test\.mjs/);
  assert.match(rows.get('inventory-completion'), /target: household_final_acceptance\n/);
  assert.match(rows.get('inventory-completion'), /tests\/household-inventory-completion\.test\.mjs/);
  assert.match(rows.get('minor-person-access'), /target: household_minor_readiness\n/);
  assert.match(rows.get('minor-person-access'), /tests\/household-minor-readiness\.test\.mjs/);
  assert.doesNotMatch(rows.get('minor-person-access'), /completion:|stock:/);
  assert.match(job, /name: Verify minor fixture failure handling\n\s+if: \$\{\{ matrix\.journey == 'minor-person-access' \}\}\n\s+run: task api:contract-minor-wrapper-test/);
  for (const [name, row] of rows) {
    if (!['calendar', 'taper-times', 'standalone-login'].includes(name)) assert.doesNotMatch(row, /clock:|acceptance:/, name);
  }
  for (const [name, target] of [['dosage-permissions', 'household_dosage_permissions'], ['stock-permissions', 'household_stock_permissions'], ['treatment-permissions', 'household_treatment_permissions']]) {
    assert.match(rows.get(name), new RegExp(`target: ${target}\\n`));
    assert.doesNotMatch(rows.get(name), /browsers:|completion:|stock:/, name);
  }
});

for (const [name, target] of [['dose', 'doses'], ['schedule', 'schedules']]) {
  test(`${name} compatibility checks have their own fixture and use the current clock`, () => {
    const workflow = readFileSync(new URL('../../../.github/workflows/ci.yml', import.meta.url), 'utf8');
    const job = workflow.match(/\n  rust_household_browser:\n(?<body>[\s\S]*?)(?=\n  [\w-]+:\n)/)?.groups?.body;
    assert.ok(job, 'household browser job was not found');
    const row = job.match(new RegExp(`^          - journey: ${name}-compatibility\\n(?<body>[\\s\\S]*?)(?=^          - journey:|^    env:)`, 'm'))?.groups?.body;
    assert.ok(row, `${name} compatibility fixture was not found`);
    assert.match(row, new RegExp(`^            target: ${target}$`, 'm'));
    assert.match(row, /^            browsers: "tests\/household-routes\.test\.mjs"$/m);
    assert.doesNotMatch(row, /clock:|completion:|stock:|filter:|acceptance:/);
  });
}

test('standalone login uses a fresh fixture without preceding household writes', () => {
  const workflow = readFileSync(new URL('../../../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const job = workflow.match(/\n  rust_household_browser:\n(?<body>[\s\S]*?)(?=\n  [\w-]+:\n)/)?.groups?.body;
  assert.ok(job, 'household browser job was not found');
  const row = job.match(/^          - journey: standalone-login\n(?<body>[\s\S]*?)(?=^          - journey:|^    env:)/m)?.groups?.body;
  assert.ok(row, 'standalone login fixture was not found');
  assert.match(row, /^            acceptance: "false"$/m);
  assert.match(row, /^            browsers: "tests\/login\.smoke\.test\.mjs"$/m);
  assert.doesNotMatch(row, /clock:|target:|completion:|stock:|filter:/);
});

test('isolated Rails web server has a readiness healthcheck for Compose wait', () => {
  const compose = readFileSync(new URL('../../../compose.yaml', import.meta.url), 'utf8');
  const webTest = compose.match(/\n  web-test:\n(?<body>[\s\S]*?)(?=\n  [\w-]+:\n)/)?.groups?.body;
  assert.ok(webTest, 'web-test service was not found');
  assert.match(webTest, /healthcheck:\n\s+test: \["CMD-SHELL", "curl -f http:\/\/localhost:3000\/up \|\| exit 1"\]/);
  assert.doesNotMatch(webTest, /healthcheck:\n\s+disable: true/);
});

test('classification failure cannot produce a successful gate', () => {
  const needs = needsFor('rails');
  needs.changes.result = 'failure';
  assert.ok(evaluate(needs).includes('Change classification must succeed'));
});
