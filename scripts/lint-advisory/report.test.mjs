import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, mkdir, copyFile, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { collectFindings, renderReport, commandArguments } from './report.mjs';

const diagnostic = (file, code = 'clippy::unwrap_used', options = {}) => ({
  reason: 'compiler-message',
  package_id: 'path+file:///workspace#med-tracker@0.1.0',
  target: { kind: ['lib'] },
  message: {
    level: 'warning', code: { code }, message: 'Do not include SECRET_VALUE in the report',
    spans: [{ file_name: file, line_start: 17, is_primary: true, text: [{ text: 'SECRET_VALUE' }] }],
    ...options,
  },
});

test('deduplicates repeated lib/test diagnostics and groups by lint and source category', () => {
  const rows = collectFindings([
    diagnostic('src/care.rs'), diagnostic('src/care.rs'),
    diagnostic('tests/care_api.rs'), diagnostic('src/models/_entities/user.rs'),
    diagnostic('migration/src/lib.rs', 'clippy::cast_sign_loss'),
    diagnostic('/outside/dep.rs'),
  ], '/workspace');
  assert.equal(rows.length, 4);
  assert.deepEqual(rows.map(row => row.category).sort(), ['generated', 'production', 'production', 'test']);
  const text = renderReport(rows, { exitCode: 0, completed: true, revision: 'abc', toolchain: 'rustc test' });
  assert.match(text, /clippy::unwrap_used/);
  assert.match(text, /migration\/src\/lib.rs/);
  assert.doesNotMatch(text, /SECRET_VALUE|outside/);
});

test('reports an incomplete run rather than claiming no findings on compiler failure', () => {
  const text = renderReport([], { exitCode: 101, completed: false, revision: 'abc', toolchain: 'rustc test' });
  assert.match(text, /Incomplete/);
  assert.match(text, /101/);
  assert.doesNotMatch(text, /No selected findings/);
});

test('workspace selection includes migration and flags stay advisory without relaxing required lints', () => {
  const args = commandArguments();
  assert.ok(args.includes('--workspace'));
  assert.ok(args.includes('--all-targets'));
  assert.ok(args.includes('--locked'));
  assert.ok(args.includes('--message-format=json'));
  assert.ok(args.includes('clippy::cast_possible_truncation'));
  assert.ok(args.includes('clippy::too_many_lines'));
  assert.ok(!args.some(arg => /cap-lints|-A|--fix/.test(arg)));
});

test('rejects spans outside the workspace and diagnostics without a primary location', () => {
  assert.deepEqual(collectFindings([
    diagnostic('../dep/lib.rs'), diagnostic('target/debug/build/generated.rs'),
    diagnostic('src/lib.rs', 'clippy::unwrap_used', { spans: [] }),
    diagnostic('src/lib.rs', 'unused_imports'),
  ], '/workspace'), []);
});

test('unit-test files are marked as test without treating all lib-test diagnostics as test code', () => {
  const rows = collectFindings([diagnostic('src/authorization/tests.rs'), diagnostic('src/lib.rs')], '/workspace');
  assert.deepEqual(rows.map(row => row.category).sort(), ['production', 'test']);
});

for (const cargoExit of [0, 101]) {
  test(`end-to-end report records Cargo exit ${cargoExit} without printing source or compiling Rust`, async () => {
    const directory = await mkdtemp(path.join(tmpdir(), 'medtracker-lint-report-'));
    try {
      const tooling = path.join(directory, 'scripts/lint-advisory');
      const bin = path.join(directory, 'bin');
      await mkdir(tooling, { recursive: true });
      await mkdir(bin);
      await copyFile(new URL('./report.mjs', import.meta.url), path.join(tooling, 'report.mjs'));
      const cargo = `#!${process.execPath}\nif (!process.argv.includes('--workspace') || !process.env.CLIPPY_CONF_DIR.endsWith('/scripts/lint-advisory/')) process.exit(2);\nconsole.log(${JSON.stringify(JSON.stringify(diagnostic('src/care.rs')))});\nconsole.log(JSON.stringify({reason:'build-finished',success:${cargoExit === 0}}));\nprocess.exit(${cargoExit});\n`;
      await writeFile(path.join(bin, 'cargo'), cargo, { mode: 0o755 });
      await writeFile(path.join(bin, 'rustc'), `#!${process.execPath}\nconsole.log('rustc fixture');\n`, { mode: 0o755 });
      await writeFile(path.join(bin, 'git'), `#!${process.execPath}\nif(process.argv.includes('rev-parse')) console.log('fixture-revision');\n`, { mode: 0o755 });
      const result = spawnSync(process.execPath, [path.join(tooling, 'report.mjs')], {
        cwd: directory, env: { ...process.env, PATH: bin }, encoding: 'utf8',
      });
      assert.equal(result.status, cargoExit === 0 ? 0 : 1, result.stderr);
      const text = await readFile(path.join(directory, 'test-results/lint-advisory/report.md'), 'utf8');
      assert.match(text, cargoExit === 0 ? /\*\*Complete\*\*/ : /\*\*Incomplete\*\*/);
      assert.match(text, /src\/care.rs/);
      assert.match(text, /fixture-revision/);
      assert.doesNotMatch(text + result.stdout + result.stderr, /SECRET_VALUE/);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
}
