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
  event.waitUntil((async () => {
    if (!await currentAccount(match[1], data.account_id)) return;
    await self.registration.showNotification(data.title, { body: data.body, icon: '/static/icon.png', data: { path, account_id: data.account_id } });
  })());
});
self.addEventListener('notificationclick', event => {
  event.notification.close();
  let url;
  try { url = new URL(event.notification.data?.path || '/', self.location.origin); } catch { return; }
  const match = url.origin === self.location.origin && url.pathname.match(/^\/households\/([^/]+)\//);
  if (!match) return;
  event.waitUntil((async () => {
    if (!await currentAccount(match[1], event.notification.data?.account_id)) return;
    await self.clients.openWindow(url.href);
  })());
});
async function currentAccount(slug, expected) {
  if (!/^\d+$/.test(expected)) return false;
  try {
    const response = await fetch(`/households/${slug}/profile/push`, { credentials: 'same-origin', cache: 'no-store' });
    if (!response.ok) return false;
    return (await response.json()).account_id === expected;
  } catch { return false; }
}
