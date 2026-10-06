import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const output = 'docs/plans/loco-migration-20261005/source-inventory.json';
const roots = ['rails/app', 'rails/config', 'rails/db', 'rails/spec', 'rails/lib', 'rails/public', 'rails/vendor/fonts', 'rails/bin', 'rails/scripts', 'rails/tasks', 'rust/api/src', 'rust/web/src', 'rust/ui-preview/src', 'rust/ui-preview/public', 'rust/contract-tests', 'docs/api/openapi.v1.yaml', 'mobile/android', 'client-tools'];
const revision = existsSync(output) ? JSON.parse(readFileSync(output, 'utf8')).revision : execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const movedRoots = new Set('app bin config db lib public spec test data vendor sorbet compose script storage log tmp Gemfile Gemfile.lock Dockerfile compose.yaml config.ru Rakefile Procfile.dev run .rspec .ruby-version .simplecov .rubocop.yml .dockerignore package.json package-lock.json .kamal .mutant'.split(' '));
const original = execFileSync('git', ['ls-tree', '-r', '--name-only', revision], { encoding: 'utf8' }).trim().split('\n');
const relocatedTracked = new Set(original.map(path => movedRoots.has(path.split('/')[0]) ? `rails/${path}` : path));
const current = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '-z'], { encoding: 'utf8' }).split('\0').filter(Boolean);
const sourceFiles = [...new Set([...relocatedTracked, ...current])].sort().filter(path => {
  if (!roots.some(root => path === root || path.startsWith(`${root}/`))) return false;
  if (!existsSync(path) || !statSync(path).isFile()) return false;
  if (!relocatedTracked.has(path) && /(^|\/)(target|node_modules|builds|build|\.gradle|\.kotlin)\//.test(path)) return false;
  return true;
});
const entries = Object.fromEntries(sourceFiles.map(path => [path, createHash('sha256').update(readFileSync(path)).digest('hex')]));
const record = {
  revision,
  roots,
  files: entries,
  routes: readFileSync('rails/config/routes.rb', 'utf8').split('\n').filter(line => /^\s*(mount|namespace|scope|resources?|get|post|put|patch|delete|root)\b/.test(line)),
  jobs: sourceFiles.filter(path => path.startsWith('rails/app/jobs/')),
  security: ['rails/app/misc/rodauth_main.rb', 'rails/config/initializers/content_security_policy.rb', 'rails/app/policies/application_policy.rb', 'rust/api/src/oauth.rs'],
};
if (process.argv.includes('--write')) {
  writeFileSync(output, `${JSON.stringify(record, null, 2)}\n`);
} else {
  const saved = JSON.parse(readFileSync(output, 'utf8'));
  assert.deepEqual(entries, saved.files, 'Migration input changed: reconcile capabilities and regenerate inventory');
  assert.deepEqual(record.routes, saved.routes);
  assert.deepEqual(record.jobs, saved.jobs);
}
