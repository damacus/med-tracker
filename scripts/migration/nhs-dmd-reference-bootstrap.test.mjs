import assert from 'node:assert/strict';
import { test } from 'node:test';
import { runReferenceImport } from './nhs-dmd-reference-bootstrap.mjs';

test('the isolated reference schema exists before Rails imports the release', async () => {
  const calls = [];
  await runReferenceImport({
    url: 'postgres://medtracker:medtracker_password@127.0.0.1:5432/medtracker_loco',
    provision: async () => calls.push('provision'),
    runCommand: async (command, args) => calls.push(`${command}:${args[0]}`),
    railsArguments: ['run', '--rm'],
    environment: {},
  });
  assert.deepEqual(calls, ['provision', 'task:db:migrate', 'docker:run']);
});

test('a failed reference migration prevents the Rails import', async () => {
  const calls = [];
  await assert.rejects(runReferenceImport({
    url: 'postgres://medtracker:medtracker_password@127.0.0.1:5432/medtracker_loco',
    provision: async () => calls.push('provision'),
    runCommand: async command => {
      calls.push(command);
      throw new Error('migration failed');
    },
    railsArguments: ['run', '--rm'],
    environment: {},
  }), /migration failed/);
  assert.deepEqual(calls, ['provision', 'task']);
});
