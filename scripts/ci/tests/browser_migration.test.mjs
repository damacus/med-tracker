import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

for (const inherited of [undefined, 'postgres://synthetic:password@127.0.0.1:5432/service_database']) {
  test(`browser migration targets its owned database with ${inherited ? 'a conflicting' : 'no'} inherited database`, t => {
    const fixture = mkdtempSync(join(tmpdir(), 'medtracker-browser-migration-'));
    t.after(() => rmSync(fixture, { recursive: true, force: true }));
    mkdirSync(join(fixture, 'debug'));
    writeFileSync(join(fixture, 'cargo'), `#!/usr/bin/env node
if (process.argv.slice(2).join(' ') !== 'metadata --locked --no-deps --format-version 1') process.exit(91);
process.stdout.write(JSON.stringify({ target_directory: process.env.SYNTHETIC_TARGET_DIRECTORY }));
`, { mode: 0o755 });
    writeFileSync(join(fixture, 'debug', 'med-tracker'), `#!/usr/bin/env node
process.stdout.write(JSON.stringify({ database: process.env.DATABASE_URL, args: process.argv.slice(2) }));
`, { mode: 0o755 });
    const environment = { ...process.env, PATH: `${fixture}:${process.env.PATH}`, SYNTHETIC_TARGET_DIRECTORY: fixture };
    delete environment.DATABASE_URL;
    if (inherited) environment.DATABASE_URL = inherited;
    const owned = 'postgres://synthetic:password@127.0.0.1:65432/owned_reference';
    const result = spawnSync('task', ['browser-care:migrate', `MIGRATION_DATABASE_URL=${owned}`, 'LOCO_ENV=test'], {
      env: environment, encoding: 'utf8', timeout: 10000,
    });
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.deepEqual(JSON.parse(result.stdout), { database: owned, args: ['db', 'migrate', '--environment', 'test'] });
  });
}
