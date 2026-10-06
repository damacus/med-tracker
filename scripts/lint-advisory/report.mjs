import { spawn, spawnSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { realpathSync } from 'node:fs';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

export const selectedLints = [
  'unwrap_used', 'expect_used', 'panic', 'todo', 'unimplemented', 'dbg_macro',
  'cast_possible_truncation', 'cast_sign_loss', 'cast_precision_loss',
  'too_many_lines', 'excessive_nesting', 'fn_params_excessive_bools',
  'await_holding_lock', 'await_holding_refcell_ref',
].map(name => `clippy::${name}`);

const defaultAsyncLints = new Set(['clippy::await_holding_lock', 'clippy::await_holding_refcell_ref']);

export function commandArguments() {
  return ['clippy', '--locked', '--workspace', '--all-targets', '--message-format=json', '--',
    ...selectedLints.flatMap(lint => ['-W', lint])];
}

function sourceCategory(file) {
  if (/(^|\/)tests?(\/|\.rs$)|(^|\/)[^/]+_test(s)?\.rs$/.test(file)) return 'test';
  if (/(^|\/)(_entities|generated)(\/|$)/.test(file)) return 'generated';
  return 'production';
}

export function collectFindings(messages, workspace) {
  const findings = new Map();
  for (const row of messages) {
    if (row.reason !== 'compiler-message' || !selectedLints.includes(row.message?.code?.code)) continue;
    const span = row.message.spans?.find(item => item.is_primary);
    if (!span) continue;
    const file = path.relative(workspace, path.resolve(workspace, span.file_name)).replaceAll(path.sep, '/');
    if (file.startsWith('../') || !/^(src|tests|migration)\//.test(file)) continue;
    const lint = row.message.code.code;
    const category = sourceCategory(file);
    const line = span.line_start;
    findings.set(`${lint}:${file}:${line}`, { lint, file, line, category });
  }
  return [...findings.values()].sort((a, b) => a.lint.localeCompare(b.lint) || a.category.localeCompare(b.category)
    || a.file.localeCompare(b.file) || a.line - b.line);
}

export function renderReport(findings, metadata) {
  const lines = [
    '# Clippy advisory report', '',
    `Run: **${metadata.completed ? 'Complete' : 'Incomplete'}**; Cargo exit: ${metadata.exitCode}.`,
    `Revision: ${metadata.revision}; working tree: ${metadata.dirty ? 'has local changes' : 'clean'}.`,
    `Toolchain: ${metadata.toolchain}.`, '',
    'Scope: locked application and migration workspace, all targets. Excluded client-tools retain their separate required check.',
    'This report does not change the required lint gate. An incomplete run is not proof of zero findings.', '',
    'Thresholds: 100 function lines, nesting depth 4, at most one boolean parameter.',
    'No source snippets, diagnostic messages, credentials or fixes are included.', '',
    '## Coverage', '',
    'The two async guard lints already run by default. They are included explicitly for visibility, not presented as new checks.',
    'Crash-surface, numeric conversion and function/boolean checks are selected advisory warnings. Nesting is measured with the isolated threshold.',
    'Test files and generated paths are separated below; diagnostics from lib and lib-test compilations are deduplicated.',
    'Inline unit tests within production files retain the production classification; review the reported location before making changes.', '',
    '## Findings', '',
  ];
  if (findings.length === 0) {
    lines.push(metadata.completed ? 'No selected findings were emitted.' : 'Findings are unavailable or partial because the run did not complete.');
  } else {
    const groups = Map.groupBy(findings, row => `${row.lint}|${row.category}|${row.file}`);
    lines.push('| Lint | Existing default async check | Category | File | Lines | Count |',
      '| --- | --- | --- | --- | --- | ---: |');
    for (const rows of groups.values()) {
      const row = rows[0];
      lines.push(`| ${row.lint} | ${defaultAsyncLints.has(row.lint) ? 'yes' : 'no'} | ${row.category} | ${row.file} | ${rows.map(item => item.line).join(', ')} | ${rows.length} |`);
    }
  }
  if (metadata.invalidDiagnostics) lines.push('', `Unrecognised diagnostics or lint configuration: ${metadata.invalidDiagnostics}. Report is incomplete.`);
  lines.push('', '## Promotion', '',
    'Fix concrete correctness and numeric risks first. The feature owner handles live files. Use a narrow lint attribute with a concrete reason only for a justified exception.',
    'Promote an agreed lint at a normal integration checkpoint after its existing findings are fixed or justified and the relevant journeys pass.', '');
  return lines.join('\n');
}

async function main() {
  const workspace = fileURLToPath(new URL('../../', import.meta.url));
  const output = path.join(workspace, 'test-results/lint-advisory');
  const version = spawnSync('rustc', ['--version'], { cwd: workspace, encoding: 'utf8' });
  const revision = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: workspace, encoding: 'utf8' });
  const dirty = spawnSync('git', ['status', '--porcelain'], { cwd: workspace, encoding: 'utf8' });
  const messages = [];
  let invalidDiagnostics = 0;
  let buildFinished = false;
  const command = spawn('cargo', commandArguments(), {
    cwd: workspace,
    env: { ...process.env, CLIPPY_CONF_DIR: fileURLToPath(new URL('.', import.meta.url)) },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  command.stderr.resume();
  const completion = new Promise(resolve => {
    command.once('error', () => resolve(-1));
    command.once('close', code => resolve(code ?? -1));
  });
  for await (const line of createInterface({ input: command.stdout, crlfDelay: Infinity })) {
    try {
      const message = JSON.parse(line);
      if (message.reason === 'compiler-message') {
        messages.push(message);
        if (message.message?.code?.code === 'unknown_lints') invalidDiagnostics++;
      }
      if (message.reason === 'build-finished') buildFinished = message.success;
    } catch {
      invalidDiagnostics++;
    }
  }
  const exitCode = await completion;
  const completed = exitCode === 0 && buildFinished && invalidDiagnostics === 0;
  const findings = collectFindings(messages, workspace);
  const metadata = {
    exitCode, completed, invalidDiagnostics,
    revision: revision.status === 0 ? revision.stdout.trim() : 'unknown',
    toolchain: version.status === 0 ? version.stdout.trim() : 'unavailable',
    dirty: dirty.status !== 0 || dirty.stdout.trim().length > 0,
  };
  await mkdir(output, { recursive: true });
  await writeFile(path.join(output, 'report.md'), renderReport(findings, metadata));
  await writeFile(path.join(output, 'findings.json'), JSON.stringify({ metadata, findings }, null, 2) + '\n');
  console.log(`Clippy advisory: ${completed ? 'complete' : 'incomplete'}, ${findings.length} selected findings. See test-results/lint-advisory/report.md`);
  process.exitCode = completed ? 0 : 1;
}

if (process.argv[1] && realpathSync(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
