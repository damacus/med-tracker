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

test('test:exec transports CMD through container env instead of the command line', () => {
  const output = dryRun('test:exec', 'CMD=pwd');
  const tailwind = output.indexOf('web-test rails tailwindcss:build');
  const command = output.indexOf('web-test sh -c \'eval "$CMD"\'');
  assert.notEqual(tailwind, -1, 'expected the tailwind build container command');
  assert.notEqual(command, -1, 'expected the container to eval the transported CMD');
  assert.ok(command > tailwind, 'expected CMD to run after the tailwind build');
  assert.match(output, /run --rm -e CMD web-test/, 'expected CMD to travel via docker run env');
  assert.ok(!output.includes('web-test pwd'), 'CMD must not be interpolated onto the host command line');
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

test('test:exec passes CMD to internal:run via vars and docker run env', () => {
  const block = taskBlock(testTaskfile, 'exec');
  assert.ok(block.includes("DOCKER_RUN_ARGS: '-e CMD'"), 'expected -e CMD forwarding');
  assert.ok(block.includes("CMD: '{{ .CMD }}'"), 'expected CMD passed via vars');
  assert.match(block, /COMMAND:.*eval/, 'expected the container command to eval CMD');
});

test('stop-all stops the dev, test, and prod profiles', () => {
  const output = dryRun('stop-all');
  for (const env of ['dev', 'test', 'prod']) {
    assert.match(output, new RegExp(`--profile ${env} stop web-${env} migrate-${env} db-${env}`),
      `expected the ${env} stop command`);
  }
  assert.ok(!output.includes('skipping execution'), output);
});

test('internal:run pre-starts the database inside the compose lock before one-off web runs', () => {
  const output = dryRun('test');
  const preUp = output.search(
    /with_compose_lock\.rb "[^"]+" docker compose -p \S+ --profile test up -d --wait db-test/,
  );
  const webRun = output.search(/run --rm\s+web-test/);
  assert.notEqual(preUp, -1, 'expected a lock-wrapped `up -d --wait db-test` pre-start command');
  assert.notEqual(webRun, -1, 'expected the `run --rm web-test` command');
  assert.ok(preUp < webRun, 'expected the db pre-start to run before the web run');
  const lockKeys = [...output.matchAll(/with_compose_lock\.rb "([^"]+)"/g)].map(match => match[1]);
  assert.ok(lockKeys.length >= 2, 'expected the pre-up and the run to both take the compose lock');
  assert.equal(new Set(lockKeys).size, 1, 'pre-up and run must share one lock key');
});

test('SERVICE callers like rubocop skip the database pre-start', () => {
  const output = dryRun('rubocop');
  assert.match(output, /run --rm\s+tools-test/, 'expected the tools-test run command');
  assert.ok(!/up -d/.test(output), 'SERVICE callers must not pre-start a database');
});

test('internal:run first cmd is the locked db pre-up gated on not .SERVICE', () => {
  const cmds = taskSection(taskBlock(internalTaskfile, 'run'), 'cmds');
  const lines = cmds.split('\n');
  const firstIndex = lines.findIndex(line => /^\s+- /.test(line));
  assert.notEqual(firstIndex, -1, 'expected a cmds entry');
  const itemPattern = new RegExp(`^${lines[firstIndex].match(/^\s*/)[0]}- `);
  const entry = [lines[firstIndex]];
  for (const line of lines.slice(firstIndex + 1)) {
    if (itemPattern.test(line)) break;
    entry.push(line);
  }
  const firstCmd = entry.join(' ');
  assert.ok(firstCmd.includes('{{ if not .SERVICE }}'), 'first cmd must be gated on `not .SERVICE`');
  assert.ok(firstCmd.includes('with_compose_lock.rb'), 'first cmd must run inside the compose lock');
  assert.ok(
    firstCmd.includes('"{{ .COMPOSE_PROJECT }}-{{ .ENVIRONMENT }}"'),
    'first cmd must use the shared compose lock key',
  );
  assert.ok(
    firstCmd.includes('up -d --wait {{ .DB_SERVICE }}'),
    'first cmd must pre-start and wait on DB_SERVICE',
  );
});

test('test:exec keeps metacharacter CMD text off the host command line', () => {
  const cmd = 'echo "a b" && exit 7';
  const output = dryRun('test:exec', `CMD=${cmd}`);
  assert.match(output, /run --rm\s+-e CMD\s+web-test/, 'expected CMD to travel via docker run env');
  assert.match(output, /web-test sh -c 'eval "\$CMD"'/, 'expected the container to eval the transported CMD');
  assert.ok(!output.includes(cmd), 'CMD text must not appear on the host command line');
});

test('test:exec tailwind run does not forward CMD', () => {
  const output = dryRun('test:exec', 'CMD=pwd');
  const runLines = output.split('\n').filter(line => /run --rm/.test(line));
  assert.ok(runLines.length >= 2, 'expected the tailwind and CMD run lines');
  assert.match(runLines[0], /web-test rails tailwindcss:build/, 'expected the first run to build tailwind');
  assert.ok(!runLines[0].includes('-e CMD'), 'the tailwind run must not forward CMD');
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
