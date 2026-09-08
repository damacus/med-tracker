import { strict as assert } from 'node:assert';
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import path from 'node:path';

const repositoryRoot = process.cwd();
mkdirSync(path.join(repositoryRoot, 'tmp'), { recursive: true });
const temporaryRoot = mkdtempSync(path.join(repositoryRoot, 'tmp', 'task-exec-regression-'));
const stubPath = path.join(temporaryRoot, 'scripts', 'with_compose_lock.rb');

const runTask = (command, marker) => {
  const result = spawnSync(
    'task',
    ['--taskfile', path.join(temporaryRoot, 'Taskfile.yml'), 'test:exec', `CMD=${command}`],
    {
      cwd: temporaryRoot,
      env: { ...process.env, TASK_EXEC_MARKER: marker },
      encoding: 'utf8'
    }
  );
  assert.equal(result.error, undefined, `task could not start: ${result.error?.message}`);
  assert.notEqual(result.status, null, 'task did not return an exit status');
  return result;
};

try {
  cpSync(path.join(repositoryRoot, 'Taskfile.yml'), path.join(temporaryRoot, 'Taskfile.yml'));
  cpSync(path.join(repositoryRoot, 'Taskfiles'), path.join(temporaryRoot, 'Taskfiles'), { recursive: true });
  mkdirSync(path.join(temporaryRoot, 'mobile', 'android'), { recursive: true });
  cpSync(
    path.join(repositoryRoot, 'mobile', 'android', 'Taskfile.yml'),
    path.join(temporaryRoot, 'mobile', 'android', 'Taskfile.yml')
  );
  mkdirSync(path.dirname(stubPath), { recursive: true });
  writeFileSync(
    stubPath,
    `#!/bin/sh
exec node --input-type=module -e '
import { appendFileSync } from "node:fs";

const args = process.argv.slice(1);
const marker = process.env.TASK_EXEC_MARKER;
const command = args.slice(args.indexOf("web-test") + 1);
appendFileSync(marker, command.join(" ") + "\\n");
if (command.join(" ") === "rails tailwindcss:build") process.exit(0);
if (command.join(" ") === "TASK_EXEC_REGRESSION_SUCCESS") process.exit(0);
if (command.join(" ").includes("TASK_EXEC_REGRESSION_FAILURE")) process.exit(23);
process.exit(1);
' "$@"
`
  );
  chmodSync(stubPath, 0o755);

  const successMarker = path.join(temporaryRoot, 'success.log');
  const success = runTask('TASK_EXEC_REGRESSION_SUCCESS', successMarker);
  assert.equal(success.status, 0, `${success.stdout}\n${success.stderr}`);
  assert.deepEqual(readFileSync(successMarker, 'utf8').trim().split('\n'), [
    'rails tailwindcss:build',
    'TASK_EXEC_REGRESSION_SUCCESS'
  ]);

  const failureMarker = path.join(temporaryRoot, 'failure.log');
  const failure = runTask('TASK_EXEC_REGRESSION_FAILURE', failureMarker);
  assert.notEqual(failure.status, 0, 'test:exec must propagate a failing supplied command');
  assert.deepEqual(readFileSync(failureMarker, 'utf8').trim().split('\n'), [
    'rails tailwindcss:build',
    'TASK_EXEC_REGRESSION_FAILURE'
  ]);

  console.log('Task execution regression passed: preparation and supplied commands both ran; failure propagated.');
} finally {
  if (existsSync(temporaryRoot)) rmSync(temporaryRoot, { recursive: true, force: true });
}
