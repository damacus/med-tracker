import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const checker = fileURLToPath(new URL('../loco_test_modules.mjs', import.meta.url));
const repository = fileURLToPath(new URL('../../..', import.meta.url));

function check(files, { cargo = '' } = {}) {
  const cwd = mkdtempSync(join(tmpdir(), 'medtracker-ci-modules-'));
  try {
    if (cargo) writeFileSync(join(cwd, 'Cargo.toml'), cargo);
    for (const [name, source] of Object.entries(files)) {
      mkdirSync(dirname(join(cwd, name)), { recursive: true });
      writeFileSync(join(cwd, name), source);
    }
    return spawnSync(process.execPath, [checker, cwd], { encoding: 'utf8' });
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}

test('every Rust test file is reachable through a mod chain', () => {
  const result = spawnSync(process.execPath, [checker, repository], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
});

test('a file declared through #[path] from a crate root is reachable', () => {
  const result = check({
    'tests/api.rs': '#[path = "api/widget.rs"]\nmod widget;\n',
    'tests/api/widget.rs': '#[test]\nfn works() {}\n',
  });
  assert.equal(result.status, 0, result.stderr);
});

test('a file never declared by any reachable module is reported', () => {
  const result = check({
    'tests/api.rs': 'mod declared;\n',
    'tests/api/declared.rs': '',
    'tests/api/orphan.rs': '',
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /tests\/api\/orphan\.rs/);
  assert.doesNotMatch(result.stderr, /declared\.rs/);
});

test('a [[test]] manifest entry counts as a crate root', () => {
  const result = check(
    { 'tests/rehearsal/cutover.rs': '#[test]\nfn works() {}\n' },
    { cargo: '[[test]]\nname = "cutover"\npath = "tests/rehearsal/cutover.rs"\nrequired-features = ["x"]\n' },
  );
  assert.equal(result.status, 0, result.stderr);
});

test('files outside any declared [[test]] target are still orphans', () => {
  const result = check(
    {
      'tests/rehearsal/cutover.rs': '',
      'tests/rehearsal/helper.rs': '',
    },
    { cargo: '[[test]]\nname = "cutover"\npath = "tests/rehearsal/cutover.rs"\n' },
  );
  assert.equal(result.status, 1);
  assert.match(result.stderr, /helper\.rs/);
});

test('a [[test]] path after an array-valued key is still a crate root', () => {
  const result = check(
    { 'tests/rehearsal/cutover.rs': '' },
    { cargo: '[[test]]\nname = "cutover"\nrequired-features = ["cutover-rehearsal"]\npath = "tests/rehearsal/cutover.rs"\n' },
  );
  assert.equal(result.status, 0, result.stderr);
});

test('a file module inside an inline module is reachable', () => {
  const result = check({
    'tests/api.rs': 'mod nested {\n    mod helper;\n}\n',
    'tests/api/nested/helper.rs': '',
  });
  assert.equal(result.status, 0, result.stderr);
});

test('a declaration inside a block comment is not reachable', () => {
  const result = check({
    'tests/api.rs': '/*\nmod hidden;\n*/\n',
    'tests/api/hidden.rs': '',
    'tests/api/commented.rs': '// mod commented;\n',
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /hidden\.rs/);
  assert.match(result.stderr, /commented\.rs/);
});

test('a never-active #[cfg] declaration is not reachable', () => {
  const result = check({
    'tests/api.rs': '#[cfg(any())]\nmod ghost;\n',
    'tests/api/ghost.rs': '',
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /ghost\.rs/);
});

test('a feature-gated declaration still counts as reachable', () => {
  const result = check({
    'tests/api.rs': '#[cfg(feature = "gate")]\nmod gated;\n',
    'tests/api/gated.rs': '',
  });
  assert.equal(result.status, 0, result.stderr);
});
