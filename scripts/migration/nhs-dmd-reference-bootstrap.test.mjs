import assert from 'node:assert/strict';
import { test } from 'node:test';
import { runReferenceImport } from './nhs-dmd-reference-bootstrap.mjs';

test('Loco snapshots an empty migrated catalogue before Rails imports the release', async () => {
  const calls = [];
  const reference = { barcodeCount: 0 };
  await runReferenceImport({
    url: 'postgres://medtracker:medtracker_password@127.0.0.1:5432/medtracker_loco',
    provision: async () => calls.push('provision'),
    runCommand: async (command, args) => {
      calls.push(`${command}:${args[0]}`);
      if (command === 'cargo') assert.equal(reference.barcodeCount, 0);
      if (command === 'docker') reference.barcodeCount = 84_410;
    },
    locoArguments: ['test', '--locked'],
    railsArguments: ['run', '--rm'],
    environment: {},
  });
  assert.deepEqual(calls, ['provision', 'task:db:migrate', 'cargo:test', 'docker:run']);
  assert.equal(reference.barcodeCount, 84_410);
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
    locoArguments: ['test', '--locked'],
    railsArguments: ['run', '--rm'],
    environment: {},
  }), /migration failed/);
  assert.deepEqual(calls, ['provision', 'task']);
});

test('a failed Loco snapshot prevents Rails from mutating the reference database', async () => {
  const calls = [];
  await assert.rejects(runReferenceImport({
    url: 'postgres://medtracker:medtracker_password@127.0.0.1:5432/medtracker_loco',
    provision: async () => calls.push('provision'),
    runCommand: async command => {
      calls.push(command);
      if (command === 'cargo') throw new Error('Loco failed');
    },
    locoArguments: ['test', '--locked'],
    railsArguments: ['run', '--rm'],
    environment: {},
  }), /Loco failed/);
  assert.deepEqual(calls, ['provision', 'task', 'cargo']);
});

test('reference import rejects databases outside its owned loopback target before provisioning', async () => {
  for (const url of [
    'postgres://medtracker:password@database.example:5432/medtracker_loco',
    'postgres://medtracker:password@127.0.0.1:5432/medtracker_other',
    'postgres://medtracker:password@127.0.0.1/medtracker_loco',
  ]) {
    let called = false;
    await assert.rejects(runReferenceImport({
      url,
      provision: async () => { called = true; },
      runCommand: async () => { called = true; },
      locoArguments: [],
      railsArguments: [],
      environment: {},
    }), /owned loopback database/);
    assert.equal(called, false);
  }
});
