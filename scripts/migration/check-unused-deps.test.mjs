import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { checkUnusedDeps } from './check-unused-deps.mjs';

test('root lint and unit stages run the workspace gate and its contract tests', () => {
  const tasks = readFileSync(new URL('../../Taskfile.yml', import.meta.url), 'utf8');
  const lint = tasks.split('  lint:')[1].split('  lint:fix:')[0];
  assert.match(lint, /cargo clippy --locked --all-targets -- -D warnings/);
  assert.match(lint, /task: machete/);
  const units = tasks.split('  test:unit:')[1].split('  test:archive-runner:')[0];
  assert.match(units, /task: machete:test/);
  assert.match(tasks, /cmds: \[node scripts\/migration\/check-unused-deps\.mjs\]/);
  assert.match(tasks, /cmds: \[node --test scripts\/migration\/check-unused-deps\.test\.mjs\]/);
});

function fixture(t, options = {}) {
  const root = mkdtempSync(join(tmpdir(), 'medtracker-machete-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'migration'));
  writeFileSync(join(root, 'Cargo.lock'), 'locked\n');
  const rootManifest = join(root, 'Cargo.toml');
  const migrationManifest = join(root, 'migration', 'Cargo.toml');
  const packages = [
    { id: 'root-id', manifest_path: rootManifest },
    { id: 'migration-id', manifest_path: migrationManifest },
    { id: 'excluded-id', manifest_path: join(root, 'client-tools', 'Cargo.toml') },
  ];
  const metadata = options.metadata ?? {
    workspace_root: root,
    workspace_members: ['root-id', 'migration-id'],
    workspace_default_members: ['root-id'],
    packages,
  };
  options.mutateMetadata?.(metadata);
  const calls = [];
  const output = [];
  const run = (command, args) => {
    calls.push({ command, args });
    if (command === 'cargo') return options.cargoResult ?? { status: 0, stdout: JSON.stringify(metadata), stderr: '' };
    options.onMachete?.();
    return options.macheteResult ?? { status: 0, stdout: 'Done!\n', stderr: '' };
  };
  const check = () => checkUnusedDeps({ root, run, write: text => output.push(text) });
  return { root, rootManifest, migrationManifest, calls, output, check };
}

test('checks every workspace member by absolute manifest and excludes nonmembers', t => {
  const f = fixture(t);
  assert.equal(f.check(), 0);
  assert.deepEqual(f.calls[0], { command: 'cargo', args: ['metadata', '--locked', '--no-deps', '--format-version', '1', '--manifest-path', f.rootManifest] });
  assert.deepEqual(f.calls[1].args, ['--with-metadata', '--skip-target-dir', f.rootManifest, f.migrationManifest]);
  assert.equal(f.calls[1].args.some(arg => arg.includes('client-tools')), false);
});

test('metadata failure stops before machete and preserves its exit status', t => {
  const f = fixture(t, { cargoResult: { status: 37, stdout: '', stderr: 'metadata failed' } });
  assert.equal(f.check(), 37);
  assert.equal(f.calls.length, 1);
});

test('partial analysis error fails even when machete exits zero', t => {
  const f = fixture(t, { macheteResult: { status: 0, stdout: 'error when handling /repo/Cargo.toml: cargo metadata exited with an error\nDone!\n', stderr: '' } });
  assert.equal(f.check(), 2);
});

test('partial analysis preserves machete nonzero status', t => {
  const f = fixture(t, { macheteResult: { status: 37, stdout: '', stderr: 'error when handling /repo/Cargo.toml: cargo metadata exited with an error\n' } });
  assert.equal(f.check(), 37);
  assert.match(f.output.join(''), /incomplete analysis/);
});

test('clean analysis passes', t => {
  const f = fixture(t);
  assert.equal(f.check(), 0);
});

test('unused dependency status propagates', t => {
  const f = fixture(t, { macheteResult: { status: 1, stdout: 'csv\n', stderr: '' } });
  assert.equal(f.check(), 1);
});

test('missing workspace package fails before machete', t => {
  const f = fixture(t, { mutateMetadata: metadata => { metadata.packages.splice(1, 1); } });
  assert.equal(f.check(), 2);
  assert.equal(f.calls.length, 1);
});

test('empty workspace membership fails before machete', t => {
  const f = fixture(t, { mutateMetadata: metadata => { metadata.workspace_members = []; } });
  assert.equal(f.check(), 2);
  assert.equal(f.calls.length, 1);
});

test('duplicate workspace membership fails before machete', t => {
  const f = fixture(t, { mutateMetadata: metadata => { metadata.workspace_members.push('migration-id'); } });
  assert.equal(f.check(), 2);
  assert.equal(f.calls.length, 1);
});

test('duplicate workspace manifest fails before machete', t => {
  const f = fixture(t, { mutateMetadata: metadata => { metadata.packages[1].manifest_path = metadata.packages[0].manifest_path; } });
  assert.equal(f.check(), 2);
  assert.equal(f.calls.length, 1);
});

test('identical duplicate package IDs fail before machete', t => {
  const f = fixture(t, { mutateMetadata: metadata => { metadata.packages.push({ ...metadata.packages[1] }); } });
  assert.equal(f.check(), 2);
  assert.equal(f.calls.length, 1);
});

test('conflicting duplicate package IDs fail before machete', t => {
  const f = fixture(t, { mutateMetadata: metadata => { metadata.packages.push({ id: 'migration-id', manifest_path: join(metadata.workspace_root, 'other', 'Cargo.toml') }); } });
  assert.equal(f.check(), 2);
  assert.equal(f.calls.length, 1);
});

test('malformed metadata JSON fails before machete', t => {
  const f = fixture(t, { cargoResult: { status: 0, stdout: '{', stderr: '' } });
  assert.equal(f.check(), 2);
  assert.equal(f.calls.length, 1);
});

test('metadata process error fails before machete', t => {
  const f = fixture(t, { cargoResult: { status: null, error: new Error('cargo missing'), stdout: '', stderr: '' } });
  assert.equal(f.check(), 2);
  assert.equal(f.calls.length, 1);
});

test('lockfile mutation fails and remains visible', t => {
  const f = fixture(t, { onMachete: () => writeFileSync(join(f.root, 'Cargo.lock'), 'mutated\n') });
  assert.equal(f.check(), 2);
  assert.equal(readFileSync(join(f.root, 'Cargo.lock'), 'utf8'), 'mutated\n');
});
