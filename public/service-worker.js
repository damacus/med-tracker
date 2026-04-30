// Add a service worker for processing Web Push notifications:

const CACHE_VERSION = 'v3'
const STATIC_CACHE = `medtracker-static-${CACHE_VERSION}`
const OFFLINE_PATH = '/offline'

// The install event is fired when the service worker is first installed
self.addEventListener('install', function(event) {
  event.waitUntil(self.skipWaiting())
  console.log('Service Worker installed');
});

// The activate event is fired after the install event when the service worker is actually controlling the page
self.addEventListener('activate', function(event) {
  event.waitUntil(
    caches.keys().then((cacheNames) => Promise.all([
      self.clients.claim(),
      ...cacheNames
        .filter((cacheName) => cacheName !== STATIC_CACHE)
        .map((cacheName) => caches.delete(cacheName))
    ]))
  )
  console.log('Service Worker activated');
});

self.addEventListener('fetch', function(event) {
  const request = event.request
  if (request.method !== 'GET') return

  const url = new URL(request.url)
  if (url.origin !== self.location.origin) return

  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request)
        .then((response) => {
          if (url.pathname === OFFLINE_PATH && response.ok) {
            const copy = response.clone()
            caches.open(STATIC_CACHE).then((cache) => cache.put(OFFLINE_PATH, copy))
          }
          return response
        })
        .catch(() => caches.match(OFFLINE_PATH).then((response) => response || fallbackOfflineResponse()))
    )
    return
  }

  if (cacheableAsset(url)) {
    event.respondWith(
      caches.match(request).then((cached) => {
        if (cached) return cached

        return fetch(request).then((response) => {
          if (response.ok) {
            const copy = response.clone()
            caches.open(STATIC_CACHE).then((cache) => cache.put(request, copy))
          }
          return response
        })
      })
    )
  }
})

function cacheableAsset(url) {
  return [
    '/assets/',
    '/icons/',
    '/fonts/',
    '/favicon.svg',
    '/manifest.webmanifest'
  ].some((prefix) => url.pathname.startsWith(prefix))
}

function fallbackOfflineResponse() {
  return new Response('<!doctype html><title>Offline</title><body><h1>Offline</h1><p>Open MedTracker while online to cache offline care.</p></body>', {
    headers: { 'Content-Type': 'text/html' },
    status: 503
  })
}

// The push event is fired when a push notification is received
self.addEventListener("push", async (event) => {
  const { title, options } = await event.data.json()
  event.waitUntil(self.registration.showNotification(title, options))
})

// The notificationclick event is fired when the user clicks on a notification
self.addEventListener("notificationclick", function(event) {
  event.notification.close()
  event.waitUntil(
    clients.matchAll({ type: "window" }).then((clientList) => {
      for (let i = 0; i < clientList.length; i++) {
        let client = clientList[i]
        let clientPath = (new URL(client.url)).pathname

        if (clientPath == event.notification.data.path && "focus" in client) {
          return client.focus()
        }
      }

      if (clients.openWindow) {
        return clients.openWindow(event.notification.data.path)
      }
    })
  )
})
