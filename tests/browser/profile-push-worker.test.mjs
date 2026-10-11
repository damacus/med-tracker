import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

function worker() {
  const handlers = {};
  const shown = [];
  const opened = [];
  const self = {
    location: { origin: 'https://medtracker.example.test' },
    addEventListener: (name, callback) => { handlers[name] = callback; },
    registration: { showNotification: async (title, options) => { shown.push({ title, ...options }); } },
    clients: { openWindow: async url => { opened.push(url); } },
  };
  runInNewContext(readFileSync('assets/static/js/profile-push-worker.js', 'utf8'), { self, URL });
  return { handlers, shown, opened };
}

test('push worker displays the supplied message and opens only its same-origin destination', async () => {
  const state = worker();
  let pending;
  state.handlers.push({ data: { json: () => ({ title: 'MedTracker', body: 'A dose is due.', path: '/households/synthetic/profile#notifications' }) }, waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(state.shown[0].body, 'A dose is due.');
  let closed = false;
  state.handlers.notificationclick({ notification: { data: state.shown[0].data, close: () => { closed = true; } }, waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(closed, true);
  assert.deepEqual(state.opened, ['https://medtracker.example.test/households/synthetic/profile#notifications']);
});

test('push worker cannot send a notification click to a foreign origin and ignores malformed destinations', async () => {
  const state = worker();
  let pending;
  state.handlers.push({ data: { json: () => ({ title: 'MedTracker', body: 'A dose is due.', path: 'https://foreign.example.test/' }) }, waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(state.shown[0].data.path, '/');
  assert.doesNotThrow(() => state.handlers.push({ data: { json: () => ({ title: 'MedTracker', body: 'A dose is due.', path: 'https://[' }) }, waitUntil() {} }));
  assert.doesNotThrow(() => state.handlers.notificationclick({ notification: { data: { path: 'https://[' }, close() {} }, waitUntil() {} }));
  state.handlers.notificationclick({ notification: { data: { path: 'https://foreign.example.test/' }, close() {} }, waitUntil() {} });
  assert.deepEqual(state.opened, []);
});
