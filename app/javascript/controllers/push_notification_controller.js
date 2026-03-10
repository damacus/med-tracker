import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["status", "subscribeButton", "unsubscribeButton", "testButton"]
  static values = { endpoint: String }

  async connect() {
    if (!("serviceWorker" in navigator) || !("PushManager" in window)) {
      this.updateStatus("Push notifications are not supported in this browser.")
      return
    }

    await this.updateUI()
  }

  async subscribe() {
    const permission = await Notification.requestPermission()
    if (permission !== "granted") {
      this.updateStatus("Notification permission denied.")
      return
    }

    try {
      const registration = await navigator.serviceWorker.ready
      const vapidKey = document.querySelector('meta[name="vapid-public-key"]')?.content
      if (!vapidKey) {
        this.updateStatus("Push notifications are not configured.")
        return
      }

      const subscription = await registration.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: this.urlBase64ToUint8Array(vapidKey)
      })

      await this.saveSubscription(subscription)
      await this.updateUI()
    } catch (error) {
      this.updateStatus("Failed to subscribe: " + error.message)
    }
  }

  async unsubscribe() {
    try {
      const registration = await navigator.serviceWorker.ready
      const subscription = await registration.pushManager.getSubscription()

      if (subscription) {
        await this.deleteSubscription(subscription)
        await subscription.unsubscribe()
      }

      await this.updateUI()
    } catch (error) {
      this.updateStatus("Failed to unsubscribe: " + error.message)
    }
  }

  async sendTest() {
    try {
      const registration = await navigator.serviceWorker.ready
      const subscription = await registration.pushManager.getSubscription()

      if (Notification.permission !== "granted" || !subscription) {
        this.updateStatus("Enable notifications on this device before sending a test.")
        this.showButton("subscribe")
        return
      }

      const token = document.querySelector('meta[name="csrf-token"]')?.content
      const response = await fetch("/push_subscription/test", {
        method: "POST",
        credentials: "same-origin",
        headers: {
          "Accept": "application/json",
          "Content-Type": "application/json",
          "X-CSRF-Token": token
        }
      })

      if (response.ok) {
        this.updateStatus("Test notification requested from server.")
      } else {
        throw new Error(await this.errorMessageFor(response, "send test"))
      }
    } catch (error) {
      this.updateStatus("Failed to send test notification: " + error.message)
    }
  }

  async updateUI() {
    if (!("serviceWorker" in navigator) || !("PushManager" in window)) {
      console.log("[PushNotification] Not supported")
      this.updateStatus("Push notifications are not supported in this browser.")
      this.showButton("none")
      this.showTestButton(false)
      return
    }

    const permission = Notification.permission
    const registration = await navigator.serviceWorker.ready
    const subscription = await registration.pushManager.getSubscription()

    console.log("[PushNotification] Permission:", permission)
    console.log("[PushNotification] Subscription:", subscription)

    if (permission === "denied") {
      this.updateStatus("Notifications are blocked in your browser settings.")
      this.showButton("none")
      this.showTestButton(false)
    } else if (permission === "granted") {
      if (subscription) {
        this.updateStatus("Notifications are fully enabled.")
        this.showButton("unsubscribe")
        this.showTestButton(true)
      } else {
        this.updateStatus("Notifications are permitted, but push subscription is missing.")
        this.showButton("subscribe")
        this.showTestButton(false)
      }
    } else {
      this.updateStatus("Notifications are not enabled.")
      this.showButton("subscribe")
      this.showTestButton(false)
    }
  }

  async saveSubscription(subscription) {
    console.log("[PushNotification] Saving subscription:", subscription)
    const data = subscription.toJSON()
    const token = document.querySelector('meta[name="csrf-token"]')?.content

    const response = await fetch("/push_subscription", {
      method: "POST",
      credentials: "same-origin",
      headers: {
        "Accept": "application/json",
        "Content-Type": "application/json",
        "X-CSRF-Token": token
      },
      body: JSON.stringify({
        endpoint: data.endpoint,
        keys: data.keys
      })
    })

    if (!response.ok) {
      throw new Error(await this.errorMessageFor(response, "subscribe"))
    }
  }

  async deleteSubscription(subscription) {
    console.log("[PushNotification] Deleting subscription:", subscription.endpoint)
    const token = document.querySelector('meta[name="csrf-token"]')?.content

    const response = await fetch("/push_subscription", {
      method: "DELETE",
      credentials: "same-origin",
      headers: {
        "Accept": "application/json",
        "Content-Type": "application/json",
        "X-CSRF-Token": token
      },
      body: JSON.stringify({ endpoint: subscription.endpoint })
    })

    if (!response.ok) {
      throw new Error(await this.errorMessageFor(response, "unsubscribe"))
    }
  }

  updateStatus(message) {
    if (this.hasStatusTarget) {
      this.statusTarget.textContent = message
    }
  }

  showButton(which) {
    if (this.hasSubscribeButtonTarget) {
      this.subscribeButtonTarget.hidden = which !== "subscribe"
    }
    if (this.hasUnsubscribeButtonTarget) {
      this.unsubscribeButtonTarget.hidden = which !== "unsubscribe"
    }
  }

  showTestButton(visible) {
    if (this.hasTestButtonTarget) {
      this.testButtonTarget.hidden = !visible
    }
  }

  urlBase64ToUint8Array(base64String) {
    const padding = "=".repeat((4 - (base64String.length % 4)) % 4)
    const base64 = (base64String + padding).replace(/-/g, "+").replace(/_/g, "/")
    const rawData = window.atob(base64)
    return Uint8Array.from([...rawData].map(char => char.charCodeAt(0)))
  }

  async errorMessageFor(response, action) {
    const contentType = response.headers.get("content-type") || ""

    if (contentType.includes("application/json")) {
      try {
        const payload = await response.json()
        if (payload?.error) {
          return payload.error
        }
      } catch (_error) {
      }
    }

    if (response.status === 422 && !response.redirected) {
      return `Failed to ${action}: your session expired. Refresh the page and try again.`
    }

    const body = await response.text()
    return `Failed to ${action}: ${body || `server returned ${response.status}`}`
  }
}
