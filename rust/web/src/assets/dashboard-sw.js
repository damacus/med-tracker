const CACHE_NAME = 'medtracker-public-dashboard-v2';
const PUBLIC_PATHS = [
  '/dashboard.css',
  '/dashboard.js',
  '/manifest.webmanifest',
  '/icons/icon-192.png',
  '/icons/icon-512.png',
  '/fonts/inter-regular.woff2',
  '/fonts/inter-500.woff2',
  '/fonts/inter-800.woff2',
  '/fonts/inter-600.woff2',
  '/fonts/inter-700.woff2',
  '/offline'
];
const PUBLIC_TYPES = {
  '/dashboard.css': 'text/css',
  '/dashboard.js': 'text/javascript',
  '/manifest.webmanifest': 'application/manifest+json',
  '/icons/icon-192.png': 'image/png',
  '/icons/icon-512.png': 'image/png',
  '/fonts/inter-regular.woff2': 'font/woff2',
  '/fonts/inter-500.woff2': 'font/woff2',
  '/fonts/inter-800.woff2': 'font/woff2',
  '/fonts/inter-600.woff2': 'font/woff2',
  '/fonts/inter-700.woff2': 'font/woff2',
  '/offline': 'text/html'
};

self.addEventListener('install', event => {
  event.waitUntil(caches.open(CACHE_NAME).then(cache => cache.addAll(PUBLIC_PATHS)).then(() => self.skipWaiting()));
});

self.addEventListener('activate', event => {
  event.waitUntil(Promise.all([
    caches.keys().then(keys => Promise.all(keys.filter(key => key.startsWith('medtracker-public-dashboard-') && key !== CACHE_NAME).map(key => caches.delete(key)))),
    self.clients.claim()
  ]));
});

self.addEventListener('fetch', event => {
  const request = event.request;
  if (request.mode === 'navigate') {
    event.respondWith(fetch(request).catch(() => caches.open(CACHE_NAME).then(cache => cache.match('/offline'))));
    return;
  }
  const url = new URL(request.url);
  if (request.method === 'GET' && url.origin === self.location.origin && !url.search && !url.hash && PUBLIC_PATHS.includes(url.pathname)) {
    event.respondWith((async () => {
      try {
        const response = await fetch(request);
        const responseUrl = response.url ? new URL(response.url) : null;
        const responseType = response.headers.get('content-type')?.split(';', 1)[0].trim().toLowerCase();
        if (response.ok && !response.redirected && responseUrl?.href === url.href && responseType === PUBLIC_TYPES[url.pathname]) {
          try {
            const cache = await caches.open(CACHE_NAME);
            await cache.put(request, response.clone());
          } catch {}
        }
        return response;
      } catch {
        const cache = await caches.open(CACHE_NAME);
        return cache.match(request);
      }
    })());
  }
});
