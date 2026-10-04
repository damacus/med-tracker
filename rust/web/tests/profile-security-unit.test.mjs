import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

const script = await readFile(fileURLToPath(new URL('../src/profile-security.js', import.meta.url)), 'utf8');

test('passkey credential bytes use browser-compatible base64url', () => {
  const { pack, unpack } = runInNewContext(`${script}\n({ pack, unpack })`, {
    btoa,
    atob,
    Uint8Array,
    document: { querySelector: () => null },
  });
  const bytes = Uint8Array.from([0xfb, 0xff]);
  assert.equal(pack(bytes.buffer), '-_8');
  assert.deepEqual(Array.from(unpack('-_8')), Array.from(bytes));
});
