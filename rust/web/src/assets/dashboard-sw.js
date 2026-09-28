const CACHE_NAME = 'medtracker-public-dashboard-v1';
const PUBLIC_PATHS = [
  '/dashboard.css',
  '/dashboard.js',
  '/manifest.webmanifest',
  '/icons/icon-192.png',
  '/icons/icon-512.png',
  '/fonts/inter-regular.woff2',
  '/fonts/inter-600.woff2',
  '/fonts/inter-700.woff2',
  '/offline'
];

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
  if (request.method === 'GET' && url.origin === self.location.origin && PUBLIC_PATHS.includes(url.pathname)) {
    event.respondWith(caches.open(CACHE_NAME).then(cache => cache.match(request)).then(cached => cached || fetch(request)));
  }
});
