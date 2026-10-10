self.addEventListener('push', event => {
  if (!event.data) return;
  let data;
  try { data = event.data.json(); } catch { return; }
  if (typeof data.title !== 'string' || typeof data.body !== 'string') return;
  let url;
  try { url = new URL(typeof data.path === 'string' ? data.path : '/', self.location.origin); } catch { return; }
  const path = url.origin === self.location.origin && /^\/households\/[^/]+\//.test(url.pathname) ? url.pathname + url.search + url.hash : '/';
  event.waitUntil(self.registration.showNotification(data.title, { body: data.body, icon: '/static/icon.png', data: { path } }));
});
self.addEventListener('notificationclick', event => {
  event.notification.close();
  let url;
  try { url = new URL(event.notification.data?.path || '/', self.location.origin); } catch { return; }
  if (url.origin !== self.location.origin) return;
  event.waitUntil(self.clients.openWindow(url.href));
});
