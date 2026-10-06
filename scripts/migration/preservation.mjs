import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, lstatSync, readFileSync, readlinkSync, writeFileSync } from 'node:fs';
import { execFileSync, spawnSync } from 'node:child_process';

const output = 'docs/plans/loco-migration-20261005/preservation.json';
const revision = existsSync(output) ? JSON.parse(readFileSync(output, 'utf8')).revision : execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const original = execFileSync('git', ['ls-tree', '-r', '--name-only', revision], { encoding: 'utf8' }).trim().split('\n');
const modes = new Map(execFileSync('git', ['ls-tree', '-r', revision], { encoding: 'utf8' }).trim().split('\n').map(line => [line.split('\t')[1], line.split(' ')[0]]));
const roots = new Set('app bin config db lib public spec test data vendor sorbet compose script storage log tmp Gemfile Gemfile.lock Dockerfile compose.yaml config.ru Rakefile Procfile.dev run .rspec .ruby-version .simplecov .rubocop.yml .dockerignore package.json package-lock.json .kamal .mutant'.split(' '));
const hash = value => createHash('sha256').update(value).digest('hex');
const comments = value => value.toString().split('\n').filter(line => /^\s*(#|\/\/|\/\*|\*|<!--)/.test(line)).join('\n');
const entries = [];
for (const source of original) {
  const moved = roots.has(source.split('/')[0]) || (source.startsWith('scripts/') && !source.startsWith('scripts/ci/'));
  const destination = moved ? `rails/${source}` : source;
  if (!moved && !source.startsWith('rails/') && !source.startsWith('Taskfiles/') && !source.startsWith('rust/') && !source.startsWith('scripts/ci/') && !source.startsWith('.github/') && source !== 'lefthook.yml' && source !== 'Taskfile.yml') continue;
  assert.ok(existsSync(destination), `Missing preserved input ${source} -> ${destination}`);
  const before = execFileSync('git', ['show', `${revision}:${source}`], { maxBuffer: 32 * 1024 * 1024 });
  const stat = lstatSync(destination);
  const currentMode = stat.isSymbolicLink() ? '120000' : (stat.mode & 0o111) ? '100755' : '100644';
  assert.equal(currentMode, modes.get(source), `Source mode changed: ${source}`);
  const after = stat.isSymbolicLink() ? Buffer.from(readlinkSync(destination)) : readFileSync(destination);
  if (modes.get(source) !== '120000' && (/\.(rb|rs|js|mjs|css|fish|sh|yml|yaml)$/.test(source) || source.includes('Dockerfile'))) {
    assert.equal(comments(after), comments(before), `Source comments changed: ${source}`);
  }
  const ignored = moved && spawnSync('git', ['check-ignore', '--no-index', destination], { stdio: 'ignore' }).status === 0;
  entries.push({ source, destination, original_mode: modes.get(source), current_mode: currentMode, original_sha256: hash(before), current_sha256: hash(after), unchanged: before.equals(after), force_stage_tracked_relocation: ignored });
}
const report = { revision, unchanged: entries.filter(entry => entry.unchanged).length, modified: entries.filter(entry => !entry.unchanged).length, entries };
const canonical = ({ entries, ...report }) => ({ ...report, entries: entries.map(({ force_stage_tracked_relocation: _advisory, ...entry }) => entry) });
if (process.argv.includes('--write')) writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
else assert.deepEqual(canonical(report), canonical(JSON.parse(readFileSync(output, 'utf8'))), 'Preservation input changed: reconcile edits and regenerate ledger');
console.log(`Preserved ${entries.length} inputs; ${report.unchanged} unchanged; ${report.modified} path/tooling edits; existing comments unchanged`);
