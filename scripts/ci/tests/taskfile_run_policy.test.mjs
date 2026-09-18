import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const repoRoot = fileURLToPath(new URL('../../..', import.meta.url));
const internalTaskfile = fileURLToPath(new URL('../../../Taskfiles/internal.yml', import.meta.url));
const rootTaskfile = fileURLToPath(new URL('../../../Taskfile.yml', import.meta.url));
const testTaskfile = fileURLToPath(new URL('../../../Taskfiles/test.yml', import.meta.url));

function dryRun(...args) {
  const result = spawnSync('task', ['--dry', '--verbose', ...args], { cwd: repoRoot, encoding: 'utf8' });
  const output = (result.stdout ?? '') + (result.stderr ?? '');
  assert.equal(result.status, 0, output);
  return output;
}

function taskBlock(filePath, taskName) {
  const lines = readFileSync(filePath, 'utf8').split('\n');
  const tasksStart = lines.findIndex(line => /^tasks:/.test(line));
  assert.notEqual(tasksStart, -1, `${filePath} has no tasks: section`);
  const namePattern = new RegExp(`^  ${taskName}:\\s*$`);
  const start = lines.findIndex((line, index) => index > tasksStart && namePattern.test(line));
  assert.notEqual(start, -1, `task "${taskName}" not found in ${filePath}`);
  const rest = lines.slice(start + 1);
  const end = rest.findIndex(line => /^ {0,2}\S/.test(line));
  return (end === -1 ? rest : rest.slice(0, end)).join('\n');
}

function taskSection(block, key) {
  const lines = block.split('\n');
  const start = lines.findIndex(line => new RegExp(`^    ${key}:`).test(line));
  assert.notEqual(start, -1, `expected a ${key}: section`);
  const rest = lines.slice(start + 1);
  const end = rest.findIndex(line => /^ {0,4}\S/.test(line));
  return (end === -1 ? rest : rest.slice(0, end)).join('\n');
}

test('test:exec runs every internal:run call, including the caller command', () => {
  const output = dryRun('test:exec', 'CMD=pwd');
  const tailwind = output.indexOf('web-test rails tailwindcss:build');
  const command = output.indexOf('web-test pwd');
  assert.notEqual(tailwind, -1, 'expected the tailwind build container command');
  assert.notEqual(command, -1, 'expected the CMD container command');
  assert.ok(command > tailwind, 'expected CMD to run after the tailwind build');
  assert.ok(!output.includes('skipping execution'), output);
});

test('internal:run declares env passthroughs for vars-transported values', () => {
  const env = taskSection(taskBlock(internalTaskfile, 'run'), 'env');
  for (const name of ['DESTINATION', 'REASON', 'CMD']) {
    assert.ok(
      env.includes(`${name}: '{{ .${name} | default "" }}'`),
      `internal:run env must default ${name} to ""`,
    );
  }
});

test('household-lifecycle:download forwards DESTINATION through internal:run vars', () => {
  const block = taskBlock(rootTaskfile, 'household-lifecycle:download');
  assert.ok(!/^    env:/m.test(block), 'task-level env: does not propagate through task: calls');
  assert.ok(block.includes("DOCKER_RUN_ARGS: '-e DESTINATION'"), 'expected -e DESTINATION forwarding');
  assert.ok(block.includes("DESTINATION: '{{ .DESTINATION }}'"), 'expected DESTINATION passed via vars');
});

test('household-lifecycle:hold forwards REASON through internal:run vars', () => {
  const block = taskBlock(rootTaskfile, 'household-lifecycle:hold');
  assert.ok(!/^    env:/m.test(block), 'task-level env: does not propagate through task: calls');
  assert.ok(block.includes("DOCKER_RUN_ARGS: '-e REASON'"), 'expected -e REASON forwarding');
  assert.ok(block.includes("REASON: '{{ .REASON }}'"), 'expected REASON passed via vars');
});

test('stop-all stops the dev, test, and prod profiles', () => {
  const output = dryRun('stop-all');
  for (const env of ['dev', 'test', 'prod']) {
    assert.match(output, new RegExp(`--profile ${env} stop web-${env} migrate-${env} db-${env}`),
      `expected the ${env} stop command`);
  }
  assert.ok(!output.includes('skipping execution'), output);
});

test('test:assets-rebuild runs every internal:run call', () => {
  const output = dryRun('test:assets-rebuild');
  assert.match(output, /find public\/assets/, 'expected the asset cleanup command');
  assert.match(output, /assets:precompile/, 'expected the precompile command');
  assert.ok(!output.includes('skipping execution'), output);
});

test('every task in Taskfiles/internal.yml declares run: always', () => {
  const lines = readFileSync(internalTaskfile, 'utf8').split('\n');
  const start = lines.findIndex(line => /^tasks:/.test(line));
  assert.notEqual(start, -1, 'Taskfiles/internal.yml has no tasks: section');
  const tasks = new Map();
  let current;
  for (const line of lines.slice(start + 1)) {
    if (/^\S/.test(line)) break;
    if (!line.trim() || line.trimStart().startsWith('#')) continue;
    const name = /^  (\S[^:]*):/.exec(line);
    if (name) {
      current = name[1].trim();
      tasks.set(current, false);
      continue;
    }
    if (current && /^    run: always\s*(#.*)?$/.test(line)) tasks.set(current, true);
  }
  assert.ok(tasks.size > 0, 'found no tasks under tasks:');
  for (const [name, always] of tasks) {
    assert.ok(always, `internal task "${name}" does not declare run: always`);
  }
});
