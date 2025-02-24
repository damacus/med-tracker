// Add a service worker for processing Web Push notifications:

// The install event is fired when the service worker is first installed
self.addEventListener('install', function(event) {
    console.log('Service Worker installed');
  });

// The activate event is fired after the install event when the service worker is actually controlling the page
self.addEventListener('activate', function(event) {
  console.log('Service Worker activated');
});

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
