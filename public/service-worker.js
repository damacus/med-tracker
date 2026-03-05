// Add a service worker for processing Web Push notifications:

const CACHE_VERSION = 'v2'
const STATIC_CACHE = `medtracker-static-${CACHE_VERSION}`
const PRECACHE_URLS = [
  '/',
  '/manifest.webmanifest'
]

// The install event is fired when the service worker is first installed
self.addEventListener('install', function(event) {
    event.waitUntil(
      Promise.all([
        self.skipWaiting(),
        caches.open(STATIC_CACHE).then((cache) => cache.addAll(PRECACHE_URLS))
      ])
    )
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
  if (event.request.method !== 'GET') {
    return
  }

  event.respondWith(
    caches.match(event.request).then((cachedResponse) => {
      if (cachedResponse) {
        return cachedResponse
      }

      return fetch(event.request).then((response) => {
        if (!response || response.status !== 200 || response.type !== 'basic') {
          return response
        }

        const responseToCache = response.clone()
        caches.open(STATIC_CACHE).then((cache) => {
          cache.put(event.request, responseToCache)
        })

        return response
      }).catch(() => cachedResponse)
    })
  )
})

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
