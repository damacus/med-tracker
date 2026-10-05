import assert from 'node:assert/strict';
import { chmodSync, cpSync, existsSync, mkdtempSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { execFileSync, spawnSync } from 'node:child_process';
import test from 'node:test';

test('root CI verifies the application without rerunning relocation snapshot audits', () => {
  const result = spawnSync('task', ['--dry', '--force', '--verbose', 'ci'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const output = result.stdout + result.stderr;
  assert.match(output, /cargo clippy/);
  assert.match(output, /run-slice\.mjs --all/);
  assert.match(output, /foundation-database\.test\.mjs/);
  assert.match(output, /process-cleanup\.test\.mjs/);
  assert.doesNotMatch(output, /workspaces\.test\.mjs|relocation\.test\.mjs|preservation(?:\.test)?\.mjs|inventory\.mjs/);
});

test('relocation snapshot audits remain available through an explicit task', () => {
  const result = spawnSync('task', ['--dry', '--force', '--verbose', 'migration:audit'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const output = result.stdout + result.stderr;
  for (const filename of ['workspaces.test.mjs', 'relocation.test.mjs', 'preservation.test.mjs', 'inventory.mjs', 'preservation.mjs']) {
    assert.ok(output.includes(filename), `Missing explicit relocation audit ${filename}`);
  }
  assert.doesNotMatch(output, /cargo clippy|run-slice\.mjs --all/);
});

test('Git attributes follow Rails schema, vendor and credential paths', () => {
  const output = execFileSync('git', ['check-attr', 'diff', 'linguist-generated', 'linguist-vendored', '--', 'rails/config/credentials/example.yml.enc', 'rails/config/credentials.yml.enc', 'rails/db/schema.rb', 'rails/vendor/example.js'], { encoding: 'utf8' });
  assert.match(output, /rails\/config\/credentials\/example.yml.enc: diff: rails_credentials/);
  assert.match(output, /rails\/config\/credentials.yml.enc: diff: rails_credentials/);
  assert.match(output, /rails\/db\/schema.rb: linguist-generated: set/);
  assert.match(output, /rails\/vendor\/example.js: linguist-vendored: set/);
});

test('release metadata locates its existing Rails version source', () => {
  const config = JSON.parse(readFileSync('release-please-config.json', 'utf8'));
  assert.ok(existsSync(config.packages['.']['version-file']));
});

test('agent guides remain identical', () => {
  assert.deepEqual(readFileSync('AGENTS.md'), readFileSync('agents.md'));
});

test('host Rails shares the canonical generated-source ignore rules', () => {
  assert.equal(realpathSync('rails/.gitignore'), realpathSync('.gitignore'));
  assert.deepEqual(readFileSync('rails/.gitignore'), readFileSync('.gitignore'));
});

test('documentation agent context globs select existing Ruby source', () => {
  const source = readFileSync('.github/agents/documentation.yml', 'utf8');
  const context = source.split('\ncontext:\n')[1].split('\ncapabilities:')[0];
  const globs = context.split('\n').filter(line => line.endsWith('/**/*.rb')).map(line => line.trim().slice(2));
  assert.equal(globs.length, 5);
  for (const glob of globs) {
    const directory = glob.slice(0, -'/**/*.rb'.length);
    assert.ok(existsSync(directory), `Documentation source context missing: ${glob}`);
    assert.match(execFileSync('rg', ['--files', directory], { encoding: 'utf8' }), /\.rb\n/);
  }
});

test('Git textconv dispatch resolves the relocated Rails entrypoint using synthetic input', () => {
  const fixture = mkdtempSync(join(tmpdir(), 'medtracker-attributes-'));
  const environment = { ...process.env, GIT_AUTHOR_NAME: 'Attribute fixture', GIT_AUTHOR_EMAIL: 'fixture@example.test', GIT_COMMITTER_NAME: 'Attribute fixture', GIT_COMMITTER_EMAIL: 'fixture@example.test' };
  delete environment.GIT_DIR;
  delete environment.GIT_WORK_TREE;
  const git = (...args) => execFileSync('git', args, { cwd: fixture, env: environment, encoding: 'utf8' });
  try {
    for (const path of ['rails/bin', 'rails/config']) mkdirSync(join(fixture, path), { recursive: true });
    cpSync('.gitattributes', join(fixture, '.gitattributes'));
    const executable = join(fixture, 'rails/bin/rails');
    writeFileSync(executable, `#!/usr/bin/env node\nconst fs=require('node:fs'); if(process.argv[2]!=='credentials:diff'||!fs.existsSync(process.argv[3]))process.exit(2); process.stdout.write('SYNTHETIC DRIVER '+fs.readFileSync(process.argv[3],'utf8'));\n`);
    chmodSync(executable, 0o755);
    const input = join(fixture, 'rails/config/credentials.yml.enc');
    writeFileSync(input, 'synthetic-before\n');
    git('init', '--quiet');
    git('add', '.gitattributes', 'rails');
    git('-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'Synthetic attribute baseline');
    git('config', '--local', 'diff.rails_credentials.textconv', 'rails/bin/rails credentials:diff');
    writeFileSync(input, 'synthetic-after\n');
    const output = git('diff', '--', 'rails/config/credentials.yml.enc');
    assert.match(output, /SYNTHETIC DRIVER synthetic-before/);
    assert.match(output, /SYNTHETIC DRIVER synthetic-after/);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});
