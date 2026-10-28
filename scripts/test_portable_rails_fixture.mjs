import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

test('synthetic Rails portable fixtures match the retained serializers', () => {
  const result = spawnSync(process.env.RUBY ?? 'ruby', ['tests/fixtures/portable_rails/generate.rb', '--check'], { encoding: 'utf8' });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stdout + result.stderr);
});
