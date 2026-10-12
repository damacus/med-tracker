import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

function worker(currentAccount = '71001', gate = { blocked: null }, statusWait = Promise.resolve(), statusEntered = () => {}) {
  const handlers = {};
  const shown = [];
  const opened = [];
  const self = {
    location: { origin: 'https://medtracker.example.test' },
    addEventListener: (name, callback) => { handlers[name] = callback; },
    registration: {
      showNotification: async (title, options) => { shown.push({ title, ...options, close() {} }); },
      getNotifications: async () => shown,
    },
    clients: { openWindow: async url => { opened.push(url); } },
  };
  self.indexedDB = { open: () => {
    const request = {};
    queueMicrotask(() => {
      request.result = {
        createObjectStore() {},
        close() {},
        transaction() {
          const transaction = { abort() {}, objectStore: () => ({
            get: () => {
              const item = { result: gate.blocked };
              queueMicrotask(() => item.onsuccess?.());
              return item;
            },
            put: value => {
              gate.blocked = value;
              queueMicrotask(() => transaction.oncomplete?.());
              return {};
            },
          }) };
          return transaction;
        },
      };
      request.onsuccess?.();
    });
    return request;
  } };
  const fetched = [];
  const fetch = async path => {
    fetched.push(path);
    statusEntered();
    await statusWait;
    return { ok: true, json: async () => ({ account_id: currentAccount }) };
  };
  runInNewContext(readFileSync('assets/static/js/profile-push-worker.js', 'utf8'), { self, URL, fetch, AbortSignal });
  return { handlers, shown, opened, fetched, self };
}

test('auth transition drains an in-flight display before acknowledging tray cleanup', async () => {
  const state = worker();
  let entered;
  const started = new Promise(resolve => { entered = resolve; });
  let release;
  const held = new Promise(resolve => { release = resolve; });
  let closed = false;
  state.self.registration.showNotification = async (title, options) => {
    entered();
    await held;
    state.shown.push({ title, ...options, close: () => { closed = true; } });
  };
  state.self.registration.getNotifications = async () => state.shown;
  let pushDone;
  state.handlers.push({ data: { json: () => ({ title: 'Medication reminder', body: 'Private medication name', path: '/households/synthetic/dashboard', account_id: '71001' }) }, waitUntil: value => { pushDone = value; } });
  await started;
  let transitionDone;
  let acknowledged = false;
  state.handlers.message({ data: { type: 'auth-transition-begin', id: 'tab-one' }, ports: [{ postMessage: () => { acknowledged = true; } }], waitUntil: value => { transitionDone = value; } });
  assert.equal(acknowledged, false);
  release();
  await pushDone;
  await transitionDone;
  assert.equal(closed, true);
  assert.equal(acknowledged, true);
});

test('auth transition drains a status check that began before logout', async () => {
  let releaseStatus;
  const statusWait = new Promise(resolve => { releaseStatus = resolve; });
  let statusEntered;
  const started = new Promise(resolve => { statusEntered = resolve; });
  const state = worker('71001', { blocked: null }, statusWait, statusEntered);
  let pushDone;
  state.handlers.push({ data: { json: () => ({ title: 'Medication reminder', body: 'Private medication name', path: '/households/synthetic/dashboard', account_id: '71001' }) }, waitUntil: value => { pushDone = value; } });
  await started;
  let transitionDone;
  let acknowledged = false;
  state.handlers.message({ data: { type: 'auth-transition-begin', id: 'tab-one' }, ports: [{ postMessage: () => { acknowledged = true; } }], waitUntil: value => { transitionDone = value; } });
  assert.equal(acknowledged, false);
  releaseStatus();
  await pushDone;
  await transitionDone;
  assert.deepEqual(state.shown, []);
  assert.equal(acknowledged, true);
});

test('transition remains blocked after worker restart and releases on authenticated navigation', async () => {
  const gate = { blocked: null };
  const first = worker('71001', gate);
  let pending;
  first.handlers.message({ data: { type: 'auth-transition-begin', id: 'tab-one' }, ports: [{ postMessage() {} }], waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(gate.blocked.id, 'tab-one');
  const restarted = worker('71001', gate);
  restarted.handlers.push({ data: { json: () => ({ title: 'Medication reminder', body: 'Private medication name', path: '/households/synthetic/dashboard', account_id: '71001' }) }, waitUntil: value => { pending = value; } });
  await pending;
  assert.deepEqual(restarted.shown, []);
  let otherTabReply;
  restarted.handlers.message({ data: { type: 'auth-transition-end', id: 'tab-two' }, ports: [{ postMessage: value => { otherTabReply = value; } }], waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(otherTabReply.ok, false);
  assert.equal(gate.blocked.id, 'tab-one');
  restarted.handlers.message({ data: { type: 'auth-transition-end', id: 'tab-one' }, ports: [{ postMessage() {} }], waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(gate.blocked, null);
  restarted.handlers.push({ data: { json: () => ({ title: 'Medication reminder', body: 'A dose is due.', path: '/households/synthetic/dashboard', account_id: '71001' }) }, waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(restarted.shown[0].body, 'A dose is due.');
});

test('an abandoned transition expires without disabling push permanently', async () => {
  const gate = { blocked: { id: 'abandoned-tab', expiresAt: Date.now() - 1 } };
  const state = worker('71001', gate);
  let pending;
  state.handlers.push({ data: { json: () => ({ title: 'Medication reminder', body: 'A dose is due.', path: '/households/synthetic/dashboard', account_id: '71001' }) }, waitUntil: value => { pending = value; } });
  await pending;
  assert.equal(state.shown[0].body, 'A dose is due.');
});

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
