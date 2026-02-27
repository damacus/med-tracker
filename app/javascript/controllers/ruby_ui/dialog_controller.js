import { Controller } from "@hotwired/stimulus"

// Connects to data-controller="dialog"
export default class extends Controller {
  static targets = ["content"]
  static values = {
    open: {
      type: Boolean,
      default: false
    },
  }

  connect() {
    this.portalElement = null

    if (this.openValue) {
      this.open()
    }
  }

  disconnect() {
    if (this.portalElement?.isConnected) {
      this.portalElement.remove()
      this.portalElement = null
    }

    this.updateBodyScrollLock()
  }

  open(e) {
    e?.preventDefault()

    if (this.portalElement?.isConnected) return

    const fragment = this.contentTarget.content.cloneNode(true)
    this.portalElement = fragment.firstElementChild
    document.body.appendChild(fragment)
    this.updateBodyScrollLock()
  }

  dismiss() {
    if (this.portalElement?.isConnected) {
      this.portalElement.remove()
      this.portalElement = null
    } else {
      this.element.remove()
    }

    this.updateBodyScrollLock()
  }

  updateBodyScrollLock() {
    const hasOpenDialog = document.body.querySelector(':scope > div[data-controller~="ruby-ui--dialog"]')

    if (hasOpenDialog) {
      document.body.classList.add('overflow-hidden')
    } else {
      document.body.classList.remove('overflow-hidden')
    }
  }
}
