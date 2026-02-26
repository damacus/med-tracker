import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["status", "subscribeButton", "unsubscribeButton"]
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

  async updateUI() {
    const permission = Notification.permission
    const registration = await navigator.serviceWorker.ready
    const subscription = await registration.pushManager.getSubscription()

    if (permission === "denied") {
      this.updateStatus("Notifications are blocked. Please update your browser settings.")
      this.showButton("subscribe")
    } else if (subscription) {
      this.updateStatus("Notifications are enabled.")
      this.showButton("unsubscribe")
    } else {
      this.updateStatus("Notifications are not enabled.")
      this.showButton("subscribe")
    }
  }

  async saveSubscription(subscription) {
    const data = subscription.toJSON()
    const token = document.querySelector('meta[name="csrf-token"]')?.content

    await fetch("/push_subscription", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": token
      },
      body: JSON.stringify({
        endpoint: data.endpoint,
        keys: data.keys
      })
    })
  }

  async deleteSubscription(subscription) {
    const token = document.querySelector('meta[name="csrf-token"]')?.content

    await fetch("/push_subscription", {
      method: "DELETE",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": token
      },
      body: JSON.stringify({ endpoint: subscription.endpoint })
    })
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

  urlBase64ToUint8Array(base64String) {
    const padding = "=".repeat((4 - (base64String.length % 4)) % 4)
    const base64 = (base64String + padding).replace(/-/g, "+").replace(/_/g, "/")
    const rawData = window.atob(base64)
    return Uint8Array.from([...rawData].map(char => char.charCodeAt(0)))
  }
}
