import assert from 'node:assert/strict';
import { existsSync, readFileSync, readlinkSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import test from 'node:test';

test('Rails rollback has its own complete runtime root', () => {
  for (const path of ['app', 'config/application.rb', 'db', 'Gemfile', 'bin/rails', 'Dockerfile', 'compose.yaml']) {
    assert.ok(existsSync(`rails/${path}`), `Missing Rails runtime path rails/${path}`);
  }
  for (const path of ['app', 'Gemfile', 'config/application.rb', 'bin/rails']) {
    assert.ok(!existsSync(path), `Rails still owns root ${path}`);
  }
  assert.ok(existsSync('docs/api/openapi.v1.yaml'));
  assert.ok(existsSync('mobile/android/Taskfile.yml'));
});

test('root commands use Loco and Rails commands are namespaced', () => {
  const listing = JSON.parse(execFileSync('task', ['--list-all', '--json'], { encoding: 'utf8' }));
  const names = new Set(listing.tasks.map(task => task.name));
  for (const name of ['dev', 'build', 'test', 'check', 'lint', 'fmt', 'routes', 'db:migrate', 'db:seed', 'worker', 'browser', 'release-image', 'ci', 'rails:test', 'rails:dev:up']) {
    assert.ok(names.has(name), `Missing task ${name}`);
  }
  assert.ok(!names.has('dev:up'));
  assert.ok(!names.has('rubocop'));
  const root = readFileSync('Taskfile.yml', 'utf8');
  assert.ok(!root.includes('flatten: true\n  rust:'));
  assert.ok(!root.includes('legacy-rails:'));
});

test('Rails helpers resolve in root and standalone namespaces', () => {
  for (const name of ['dev:up', 'test', 'audit:verify']) {
    for (const args of [['--dry', `rails:${name}`], ['--dir', 'rails', '--dry', name]]) {
      const output = execFileSync('task', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
      assert.ok(!output.includes('cargo run'));
    }
  }
});

test('repository hooks target the relocated Rails commands', () => {
  const hooks = readFileSync('lefthook.yml', 'utf8');
  assert.ok(hooks.includes('task rails:rubocop'));
  assert.ok(hooks.includes('task rails:test'));
  assert.ok(hooks.includes('task rails:gems:audit'));
  assert.ok(hooks.includes('rails/config/locales/**/*.yml'));
  assert.ok(!hooks.includes('run: bundle'));
});

test('shared CI scripts have one canonical root implementation', () => {
  assert.equal(readlinkSync('rails/scripts/ci'), '../../scripts/ci');
  assert.equal(readlinkSync('rails/docs'), '../docs');
  for (const name of ['classify.mjs', 'policy.json', 'gate.mjs', 'shard.mjs']) {
    assert.ok(existsSync(`scripts/ci/${name}`));
    assert.equal(readFileSync(`rails/scripts/ci/${name}`, 'utf8'), readFileSync(`scripts/ci/${name}`, 'utf8'));
  }
});
