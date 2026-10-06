import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import test from 'node:test';

for (const directory of ['rust/api', 'rust/web', 'rust/ui-preview', 'rust/contract-tests', 'client-tools']) {
  test(`${directory} remains an independent migration input workspace`, () => {
    const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1', '--manifest-path', `${directory}/Cargo.toml`], { encoding: 'utf8' }));
    assert.equal(metadata.workspace_root, resolve(directory));
    assert.ok(!metadata.workspace_members.some(name => name.includes('#med-tracker@')));
  });
}
