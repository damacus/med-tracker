import assert from 'node:assert/strict';
import { test } from 'node:test';
import { assertCleanBenchmarkSource, compareSnapshots, comparisonInput, runReferenceImport } from './nhs-dmd-reference-bootstrap.mjs';

test('synthetic comparison selects only its pinned fixture and checksum', () => {
  const root = '/checkout';
  const selection = comparisonInput('synthetic', undefined, root);
  assert.equal(selection.release, `${root}/tests/browser/fixtures/nhs-dmd-synthetic-supplementary.zip`);
  assert.equal(selection.expectedChecksum, '5d83eef3d1377d3489bd0e43e3f79abfc779b7ee20ae97fad1c598efeb1a1646');
  assert.throws(() => comparisonInput('synthetic', '/other.zip', root), /NHS_DMD_RELEASE_PATH/);
  assert.throws(() => comparisonInput('official', undefined, root), /NHS_DMD_RELEASE_PATH/);
  assert.equal(comparisonInput('official', '/official.zip', root).release, '/official.zip');
});

test('comparison detects supplementary relationship and release metadata changes despite equal counts', () => {
  const reference = {
    status: 4, created: 1, updated: 0, unchanged: 0, skipped_expired: 0,
    skipped_missing_name: 0, skipped_invalid: 0, trade_families: 1,
    trade_family_groups: 1, trade_memberships: 1,
    barcodes: [['5016298210989', '777', 'A1', 'Scanner imported tablets', null, 'nhs_dmd', 'ampp']],
    relationships: [['777', 'A1']],
    trade_family_group_rows: [['G1', 'Group']],
    trade_family_rows: [['F1', 'Family', 'G1']],
    trade_membership_rows: [['A1', 'F1']],
    supplementary_release_rows: [['2026-10-31']],
  };
  const candidate = structuredClone(reference);
  candidate.trade_family_group_rows = [['G1', 'Changed group']];
  candidate.trade_family_rows = [['F1', 'Family', null]];
  candidate.trade_membership_rows = [['A1', 'F2']];
  candidate.supplementary_release_rows = [['2026-11-01']];
  const result = compareSnapshots(reference, candidate);
  assert.deepEqual(result.countDifferences, {});
  for (const key of ['trade_family_group_rows', 'trade_family_rows', 'trade_membership_rows']) {
    assert.equal(result.rowDifferences[key].mismatches, 1, key);
  }
  assert.equal(result.rowDifferences.supplementary_release_rows.mismatches, 2);
});

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

test('reference comparison pins UTC for both runtimes despite an inherited timezone', async () => {
  await runReferenceImport({
    url: 'postgres://medtracker:medtracker_password@127.0.0.1:5432/medtracker_loco',
    provision: async () => {},
    runCommand: async (command, args, environment) => {
      if (command === 'cargo') assert.equal(environment.TZ, 'UTC');
      if (command === 'docker') {
        assert.equal(environment.TZ, 'UTC');
        assert.deepEqual(args.slice(0, 3), ['run', '--env', 'TZ=UTC']);
      }
    },
    locoArguments: ['test'],
    railsArguments: ['run', '--rm'],
    environment: { TZ: 'Europe/London' },
    locoEnvironment: { TZ: 'Europe/London' },
  });
});

test('benchmark uses the prepared executable without compiling between sequential imports', async () => {
  const calls = [];
  await runReferenceImport({
    url: 'postgres://medtracker:password@127.0.0.1:5432/medtracker_loco',
    provision: async () => calls.push('provision'),
    runCommand: async (command) => calls.push(command),
    locoCommand: '/checkout/target/ci/care_api',
    locoArguments: ['nhs_dmd_official_release_comparison_snapshot', '--ignored'],
    railsArguments: ['run', '--rm'],
    environment: {},
  });
  assert.deepEqual(calls, ['provision', 'task', '/checkout/target/ci/care_api', 'docker']);
  assert.throws(() => assertCleanBenchmarkSource('?? src/new_import.rs\n'), /clean worktree/);
  assert.throws(() => assertCleanBenchmarkSource(' M src/models/nhs_dmd/import.rs\n'), /clean worktree/);
  assert.doesNotThrow(() => assertCleanBenchmarkSource(''));
});
