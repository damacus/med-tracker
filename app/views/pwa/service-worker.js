self.addEventListener("push", async (event) => {
  const { title, options } = await event.data.json()
  event.waitUntil(self.registration.showNotification(title, options))
})

self.addEventListener("notificationclick", (event) => {
  event.notification.close()
  event.waitUntil(
    clients.matchAll({ type: "window" }).then((clientList) => {
      for (const client of clientList) {
        if ((new URL(client.url)).pathname === event.notification.data?.path && "focus" in client)
          return client.focus()
      }
      if (clients.openWindow) return clients.openWindow(event.notification.data?.path || "/")
    })
  )
})
