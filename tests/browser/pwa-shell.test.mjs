import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import vm from 'node:vm';

test('the public shell advertises an account-neutral standalone application', async () => {
  const layout = await readFile('assets/views/layouts/base.html', 'utf8');
  assert.match(layout, /rel="manifest"[^>]*href="\/manifest\.webmanifest"/);
  const manifest = JSON.parse(await readFile('assets/static/manifest.webmanifest', 'utf8'));
  assert.equal(manifest.id, '/');
  assert.equal(manifest.start_url, '/');
  assert.equal(manifest.scope, '/');
  assert.equal(manifest.display, 'standalone');
  for (const size of ['192x192', '512x512']) {
    assert.ok(manifest.icons.some(icon => icon.sizes === size && icon.type === 'image/png'));
  }
  assert.ok(manifest.icons.some(icon => icon.purpose === 'maskable'));
});

test('offline recovery is limited to failed page navigations', async () => {
  const handlers = new Map();
  const self = { addEventListener: (type, listener) => handlers.set(type, listener), location: { origin: 'https://example.test' } };
  const context = { self, URL, Response, fetch: async () => { throw new TypeError('offline'); } };
  vm.runInNewContext(await readFile('assets/static/js/profile-push-worker.js', 'utf8'), context);
  const handler = handlers.get('fetch');
  assert.equal(typeof handler, 'function');
  let result;
  handler({ request: { method: 'GET', mode: 'navigate', url: 'https://example.test/households/one' }, respondWith: promise => { result = promise; } });
  assert.match(await (await result).text(), /Connect to the internet to use MedTracker/);
  result = undefined;
  handler({ request: { method: 'POST', mode: 'cors', url: 'https://example.test/api/v1/doses' }, respondWith: promise => { result = promise; } });
  assert.equal(result, undefined);
  let apiOptions;
  context.fetch = async (_, options) => { apiOptions = options; return new Response('Forbidden', { status: 403 }); };
  handler({ request: { method: 'GET', mode: 'navigate', url: 'https://example.test/api/v1/doses' }, respondWith: promise => { result = promise; } });
  assert.equal((await result).status, 403);
  assert.equal(apiOptions.cache, 'no-store');
  context.fetch = async (_, options) => { assert.equal(options.cache, 'no-store'); return new Response('Forbidden', { status: 403 }); };
  handler({ request: { method: 'GET', mode: 'navigate', url: 'https://example.test/households/one' }, respondWith: promise => { result = promise; } });
  assert.equal((await result).status, 403);
});
