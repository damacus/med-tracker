import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

function worker(currentAccount = '71001') {
  const handlers = {};
  const shown = [];
  const opened = [];
  const self = {
    location: { origin: 'https://medtracker.example.test' },
    addEventListener: (name, callback) => { handlers[name] = callback; },
    registration: { showNotification: async (title, options) => { shown.push({ title, ...options }); } },
    clients: { openWindow: async url => { opened.push(url); } },
  };
  const fetched = [];
  const fetch = async path => {
    fetched.push(path);
    return { ok: true, json: async () => ({ account_id: currentAccount }) };
  };
  runInNewContext(readFileSync('assets/static/js/profile-push-worker.js', 'utf8'), { self, URL, fetch });
  return { handlers, shown, opened, fetched };
}

test('push worker suppresses private reminder content after a browser account switch', async () => {
  const state = worker('71002');
  let pending;
  state.handlers.push({ data: { json: () => ({ title: 'Medication reminder', body: 'Private medication name', path: '/households/synthetic/profile#notifications', account_id: '71001' }) }, waitUntil: value => { pending = value; } });
  await pending;
  assert.deepEqual(state.shown, []);
  assert.deepEqual(state.fetched, ['/households/synthetic/profile/push']);
});

test('push worker suppresses legacy and signed-out reminder payloads', async () => {
  const state = worker(null);
  let pending;
  state.handlers.push({ data: { json: () => ({ title: 'Medication reminder', body: 'Private medication name', path: '/households/synthetic/dashboard', account_id: '71001' }) }, waitUntil: value => { pending = value; } });
  await pending;
  state.handlers.push({ data: { json: () => ({ title: 'Medication reminder', body: 'Private medication name', path: '/households/synthetic/dashboard' }) }, waitUntil: value => { pending = value; } });
  assert.deepEqual(state.shown, []);
});

test('push worker displays the supplied message and opens only its same-origin destination', async () => {
  const state = worker();
  let pending;
  state.handlers.push({ data: { json: () => ({ title: 'MedTracker', body: 'A dose is due.', path: '/households/synthetic/profile#notifications', account_id: '71001' }) }, waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(state.shown[0].body, 'A dose is due.');
  let closed = false;
  state.handlers.notificationclick({ notification: { data: state.shown[0].data, close: () => { closed = true; } }, waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(closed, true);
  assert.deepEqual(state.opened, ['https://medtracker.example.test/households/synthetic/profile#notifications']);
  assert.deepEqual(state.fetched, ['/households/synthetic/profile/push', '/households/synthetic/profile/push']);
});

test('push worker does not open a retained notification after the account changes', async () => {
  const state = worker('71002');
  let pending;
  state.handlers.notificationclick({ notification: { data: { path: '/households/synthetic/dashboard', account_id: '71001' }, close() {} }, waitUntil: value => { pending = value; } });
  await pending;
  assert.deepEqual(state.opened, []);
});

test('push worker cannot send a notification click to a foreign origin and ignores malformed destinations', async () => {
  const state = worker();
  let pending;
  state.handlers.push({ data: { json: () => ({ title: 'MedTracker', body: 'A dose is due.', path: 'https://foreign.example.test/', account_id: '71001' }) }, waitUntil: value => { pending = value; } });
  assert.deepEqual(state.shown, []);
  assert.doesNotThrow(() => state.handlers.push({ data: { json: () => ({ title: 'MedTracker', body: 'A dose is due.', path: 'https://[' }) }, waitUntil() {} }));
  assert.doesNotThrow(() => state.handlers.notificationclick({ notification: { data: { path: 'https://[' }, close() {} }, waitUntil() {} }));
  state.handlers.notificationclick({ notification: { data: { path: 'https://foreign.example.test/' }, close() {} }, waitUntil() {} });
  assert.deepEqual(state.opened, []);
});
