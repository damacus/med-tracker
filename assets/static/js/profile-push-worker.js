let transitionStarting = false;
const pendingDisplays = new Set();

self.addEventListener('install', event => { event.waitUntil(self.skipWaiting()); });
self.addEventListener('activate', event => {
  event.waitUntil((async () => {
    (await self.registration.getNotifications()).forEach(notification => notification.close());
    await self.clients.claim();
  })());
});

self.addEventListener('message', event => {
  const type = event.data?.type;
  const id = event.data?.id;
  if (type === 'activate') { event.waitUntil(self.skipWaiting()); return; }
  if (!['auth-transition-begin', 'auth-transition-status', 'auth-transition-end'].includes(type) || !event.ports?.[0]) return;
  if (type !== 'auth-transition-status' && (typeof id !== 'string' || !id)) return;
  if (type === 'auth-transition-begin') transitionStarting = true;
  event.waitUntil((async () => {
    try {
      if (type === 'auth-transition-begin') {
        await transitionState(id, null);
        transitionStarting = false;
        await Promise.allSettled([...pendingDisplays]);
        (await self.registration.getNotifications()).forEach(notification => notification.close());
      } else if (type === 'auth-transition-end') {
        await transitionState(null, id);
      }
      event.ports[0].postMessage({ ok: true, protocol: 1, id: await transitionState() });
    } catch {
      if (await transitionState().catch(() => null)) transitionStarting = false;
      event.ports[0].postMessage({ ok: false });
    }
  })());
});

self.addEventListener('push', event => {
  if (!event.data) return;
  let data;
  try { data = event.data.json(); } catch { return; }
  if (typeof data.title !== 'string' || typeof data.body !== 'string' || !/^\d+$/.test(data.account_id)) return;
  let url;
  try { url = new URL(typeof data.path === 'string' ? data.path : '/', self.location.origin); } catch { return; }
  const match = url.origin === self.location.origin && url.pathname.match(/^\/households\/([^/]+)\//);
  if (!match) return;
  const path = url.pathname + url.search + url.hash;
  const display = (async () => {
    if (!await currentAccount(match[1], data.account_id) || transitionStarting || await transitionState()) return;
    await self.registration.showNotification(data.title, { body: data.body, icon: '/static/icon.png', data: { path, account_id: data.account_id } });
  })();
  pendingDisplays.add(display);
  display.then(() => pendingDisplays.delete(display), () => pendingDisplays.delete(display));
  event.waitUntil(display);
});
self.addEventListener('notificationclick', event => {
  event.notification.close();
  let url;
  try { url = new URL(event.notification.data?.path || '/', self.location.origin); } catch { return; }
  const match = url.origin === self.location.origin && url.pathname.match(/^\/households\/([^/]+)\//);
  if (!match) return;
  event.waitUntil((async () => {
    if (!await currentAccount(match[1], event.notification.data?.account_id) || transitionStarting || await transitionState()) return;
    await self.clients.openWindow(url.href);
  })());
});
async function currentAccount(slug, expected) {
  if (!/^\d+$/.test(expected)) return false;
  try {
    if (transitionStarting || await transitionState()) return false;
    const response = await fetch(`/households/${slug}/profile/push`, { credentials: 'same-origin', cache: 'no-store', signal: AbortSignal.timeout(3000) });
    if (!response.ok) return false;
    return (await response.json()).account_id === expected;
  } catch { return false; }
}

function transitionState(value, expected) {
  return new Promise((resolve, reject) => {
    const open = self.indexedDB.open('medtracker-push-transition', 1);
    open.onupgradeneeded = () => open.result.createObjectStore('state');
    open.onerror = () => reject(open.error);
    open.onsuccess = () => {
      const database = open.result;
      const transaction = database.transaction('state', value === undefined ? 'readonly' : 'readwrite');
      const store = transaction.objectStore('state');
      const request = store.get('blocked');
      request.onerror = () => { database.close(); reject(request.error); };
      transaction.onerror = () => { database.close(); reject(transaction.error); };
      request.onsuccess = () => {
        const saved = request.result;
        const current = saved?.expiresAt > Date.now() ? saved.id : null;
        if (value === undefined) { database.close(); resolve(current); return; }
        if ((value === null && current !== expected) || (value !== null && current && current !== value)) {
          transaction.abort();
          database.close();
          reject(Error('Another tab owns the transition'));
          return;
        }
        store.put(value === null ? null : { id: value, expiresAt: Date.now() + 30 * 60 * 1000 }, 'blocked');
      };
      if (value !== undefined) transaction.oncomplete = () => { database.close(); resolve(value); };
    };
  });
}
