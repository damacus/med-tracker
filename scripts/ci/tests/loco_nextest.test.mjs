import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { archiveArguments, buildArchiveArtifact, filterForSelection, parseNextestInventory, runArchiveStage, selectedTests, validateArchiveArtifact } from '../loco-nextest.mjs';

const listing = {
  'test-count': 5,
  'rust-suites': {
    'med-tracker': { 'package-name': 'med-tracker', 'binary-id': 'med-tracker', kind: 'lib', status: 'listed', testcases: { unit_case: { ignored: false } } },
    'med-tracker::care_api': { 'package-name': 'med-tracker', 'binary-id': 'med-tracker::care_api', kind: 'test', status: 'listed', testcases: {
      'contract::tests::schema': { ignored: false },
      'profile::reads_profile': { ignored: false },
      'api_v1::acceptance': { ignored: true },
    } },
    'migration::identity_schema': { 'package-name': 'migration', 'binary-id': 'migration::identity_schema', kind: 'test', status: 'listed', testcases: { structural_identity: { ignored: false } } },
  },
};

test('archive compiles the full locked workspace through nextest', () => {
  assert.deepEqual(archiveArguments('target/ci/loco-tests.tar.zst'), ['nextest', 'archive', '--locked', '--workspace', '--archive-file', 'target/ci/loco-tests.tar.zst']);
});

test('inventory preserves migration and the database-free partition while omitting ignored and already-run library cases', () => {
  const inventory = parseNextestInventory(listing);
  assert.deepEqual(inventory.map(item => `${item.binary}/${item.name}`), [
    'med-tracker::care_api/contract::tests::schema',
    'med-tracker::care_api/profile::reads_profile',
    'migration::identity_schema/structural_identity',
  ]);
  assert.deepEqual(selectedTests(inventory, 'unit').map(item => item.name), ['contract::tests::schema']);
  assert.deepEqual(selectedTests(inventory, 'integration', '1/1').map(item => item.name), ['profile::reads_profile', 'structural_identity']);
  const shards = ['1/2', '2/2'].flatMap(shard => selectedTests(inventory, 'integration', shard).map(item => item.identity));
  assert.deepEqual(shards.sort(), selectedTests(inventory, 'integration', '1/1').map(item => item.identity).sort());
});

test('inventory rejects missing, duplicate, malformed and zero selections', () => {
  for (const invalid of [{}, { ...listing, 'test-count': 0 }, { ...listing, 'test-count': 6 }, { ...listing, 'rust-suites': {} }]) {
    assert.throws(() => parseNextestInventory(invalid));
  }
  const duplicate = structuredClone(listing);
  duplicate['rust-suites']['med-tracker::care_api']['binary-id'] = 'migration::identity_schema';
  assert.throws(() => parseNextestInventory(duplicate));
});

test('exact nextest filters include only selected names from one binary', () => {
  assert.equal(filterForSelection('med-tracker::care_api', ['profile::reads_profile', 'contract::tests::schema']),
    'binary_id(=med-tracker::care_api) & (test(=profile::reads_profile) | test(=contract::tests::schema))');
  assert.throws(() => filterForSelection('med-tracker::care_api', []));
  assert.throws(() => filterForSelection('med-tracker::care_api', ['ok', 'ok']));
  assert.throws(() => filterForSelection('med-tracker::care_api', ['ok) | all()']));
});

test('archive builder uses supported nextest archive/list and records app and source identity', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'medtracker-archive-contract-'));
  const calls = [];
  const source = { revision: randomUUID(), sha256: randomUUID() };
  try {
    const run = async (command, args) => {
      calls.push([command, args]);
      if (args[0] === 'nextest' && args[1] === 'archive') {
        await writeFile(args[args.indexOf('--archive-file') + 1], 'archive');
        return '';
      }
      if (args[0] === 'build') return `${JSON.stringify({ reason: 'compiler-artifact', target: { name: 'med-tracker', kind: ['bin'] }, profile: { test: false }, executable: join(directory, 'built-app') })}\n`;
      if (args[0] === 'nextest' && args[1] === 'list') return JSON.stringify(listing);
      throw new Error(`Unexpected build command: ${args.join(' ')}`);
    };
    await writeFile(join(directory, 'built-app'), 'application');
    await mkdir(join(directory, 'assets', 'static', 'css'), { recursive: true });
    await mkdir(join(directory, 'assets', 'static', 'js'), { recursive: true });
    await writeFile(join(directory, 'assets', 'static', 'css', 'compiled.css'), 'compiled styles');
    await writeFile(join(directory, 'assets', 'static', 'js', 'html5-qrcode.min.js'), 'scanner library');
    await buildArchiveArtifact({ directory: join(directory, 'ci'), root: directory, run, identity: async () => source });
    const manifest = JSON.parse(await readFile(join(directory, 'ci', 'tests.json'), 'utf8'));
    assert.equal(manifest.revision, source.revision);
    assert.equal(manifest.sourceSha256, source.sha256);
    assert.equal(manifest.inventory.length, 3);
    assert.deepEqual(calls.map(([, args]) => args.slice(0, 2)), [['nextest', 'archive'], ['build', '--locked'], ['nextest', 'list']]);
    await validateArchiveArtifact(join(directory, 'ci'), source, directory);
    await writeFile(join(directory, 'assets', 'static', 'css', 'compiled.css'), 'stale styles');
    await assert.rejects(validateArchiveArtifact(join(directory, 'ci'), source, directory), /asset/i);
    await writeFile(join(directory, 'assets', 'static', 'css', 'compiled.css'), 'compiled styles');
    await assert.rejects(validateArchiveArtifact(join(directory, 'ci'), { ...source, sha256: 'changed' }, directory), /source/i);
    await writeFile(join(directory, 'ci', 'loco-tests.tar.zst'), 'stale archive');
    await assert.rejects(validateArchiveArtifact(join(directory, 'ci'), source, directory), /archive/i);
    await writeFile(join(directory, 'ci', 'loco-tests.tar.zst'), 'archive');
    await writeFile(join(directory, 'ci', 'med-tracker'), 'wrong application');
    await assert.rejects(validateArchiveArtifact(join(directory, 'ci'), source, directory), /application/i);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('integration archive run credits exactly the selected cases across both binaries', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'medtracker-archive-run-'));
  const ci = join(directory, 'ci');
  const target = join(directory, 'target');
  const source = { revision: randomUUID(), sha256: randomUUID() };
  const hash = value => createHash('sha256').update(value).digest('hex');
  const archive = 'archive';
  const application = 'application';
  const calls = [];
  try {
    await mkdir(ci);
    await mkdir(target);
    await mkdir(join(directory, 'assets', 'static', 'css'), { recursive: true });
    await mkdir(join(directory, 'assets', 'static', 'js'), { recursive: true });
    await writeFile(join(directory, 'assets', 'static', 'css', 'compiled.css'), 'compiled styles');
    await writeFile(join(directory, 'assets', 'static', 'js', 'html5-qrcode.min.js'), 'scanner library');
    await writeFile(join(ci, 'loco-tests.tar.zst'), archive);
    await writeFile(join(ci, 'med-tracker'), application, { mode: 0o755 });
    const manifest = { version: 1, revision: source.revision, sourceSha256: source.sha256, archiveSha256: hash(archive), applicationSha256: hash(application), assetsSha256: { 'assets/static/css/compiled.css': hash('compiled styles'), 'assets/static/js/html5-qrcode.min.js': hash('scanner library') }, inventory: parseNextestInventory(listing) };
    await writeFile(join(ci, 'tests.json'), JSON.stringify(manifest));
    let reportXml = '<testsuites tests="2" failures="0" errors="0"><testsuite name="med-tracker::care_api" tests="1" failures="0" errors="0"><testcase name="profile::reads_profile" classname="med-tracker::care_api"/></testsuite><testsuite name="migration::identity_schema" tests="1" failures="0" errors="0"><testcase name="structural_identity" classname="migration::identity_schema"/></testsuite></testsuites>';
    let failRun = false;
    const own = async callback => callback('postgres://medtracker:medtracker_password@127.0.0.1:5432/owned', async () => { calls.push('provision'); }, { storageEnvironment: {}, storage: async () => { calls.push('storage'); return { S3_ENDPOINT: 'http://127.0.0.1:9000' }; } });
    const run = async (command, args, { env } = {}) => {
      calls.push([command, args]);
      if (args[1] === 'list') return JSON.stringify(listing);
      assert.deepEqual(args.slice(0, 4), ['nextest', 'run', '--archive-file', join(ci, 'loco-tests.tar.zst')]);
      assert.equal(env.MEDTRACKER_OWNED_DATABASE, '1');
      if (failRun) throw new Error('nextest failed');
      const config = await readFile(args[args.indexOf('--config-file') + 1], 'utf8');
      const report = join(target, 'nextest', 'contract-proof', config.match(/path = "([^"]+)"/)[1]);
      await writeFile(report, reportXml);
    };
    await runArchiveStage({ stage: 'integration', shard: '1/1', directory: ci, root: directory, run, own, identity: async () => source, environment: { CARGO_TARGET_DIR: target } });
    assert.deepEqual(calls.filter(call => typeof call === 'string'), ['provision', 'storage']);
    assert.equal(calls.filter(call => Array.isArray(call) && call[1][1] === 'run').length, 1);
    assert.match(calls.find(call => Array.isArray(call) && call[1][1] === 'run')[1].join(' '), /migration::identity_schema/);
    const selections = JSON.parse(await readFile(join(directory, 'test-results', 'integration-selection-1.json'), 'utf8'));
    assert.deepEqual(selections.sort(), ['med-tracker::care_api/profile::reads_profile', 'migration::identity_schema/structural_identity']);
    reportXml = reportXml.replace('name="structural_identity"', 'name="unselected_case"');
    await assert.rejects(runArchiveStage({ stage: 'integration', shard: '1/1', directory: ci, root: directory, run, own, identity: async () => source, environment: { CARGO_TARGET_DIR: target } }), /exact selection/);
    failRun = true;
    await assert.rejects(runArchiveStage({ stage: 'integration', shard: '1/1', directory: ci, root: directory, run, own, identity: async () => source, environment: { CARGO_TARGET_DIR: target } }), /nextest failed/);
    failRun = false;
    for (const mutate of [
      inventory => inventory.filter(item => item.name !== 'structural_identity'),
      inventory => [...inventory, { ...inventory[0], name: 'extra_case', identity: `${inventory[0].binary}/extra_case` }],
      inventory => inventory.map(item => item.name === 'structural_identity' ? { ...item, databaseFree: undefined } : item),
      inventory => inventory.map(item => item.name === 'profile::reads_profile' ? { ...item, binaryName: 'not_care_api' } : item),
    ]) {
      const before = calls.length;
      await writeFile(join(ci, 'tests.json'), JSON.stringify({ ...manifest, inventory: mutate(manifest.inventory) }));
      await assert.rejects(runArchiveStage({ stage: 'integration', shard: '1/1', directory: ci, root: directory, run, own, identity: async () => source, environment: { CARGO_TARGET_DIR: target } }), /inventory.*archive|archive.*inventory/i);
      assert.equal(calls.slice(before).filter(call => call === 'provision' || call === 'storage' || (Array.isArray(call) && call[1][1] === 'run')).length, 0);
    }
    await writeFile(join(ci, 'tests.json'), JSON.stringify(manifest));
    let cleanups = 0;
    await assert.rejects(runArchiveStage({ stage: 'integration', shard: '1/1', directory: ci, root: directory, run, own: async () => { throw new Error('provisioning failed'); }, identity: async () => source, environment: { CARGO_TARGET_DIR: target }, proofFactory: async () => ({ cleanup: async () => { cleanups += 1; } }) }), /provisioning failed/);
    assert.equal(cleanups, 1, 'A failed owned fixture must release its unused proof report');
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
