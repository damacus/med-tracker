import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["label"]
  static values = {
    readyLabel: String,
    preparingLabel: String
  }

  connect() {
    this.reset = this.reset.bind(this)
    this.element.addEventListener("turbo:before-cache", this.reset)
  }

  disconnect() {
    window.clearTimeout(this.resetTimeout)
    this.element.removeEventListener("turbo:before-cache", this.reset)
  }

  prepare(event) {
    if (this.busy) {
      event.preventDefault()
      return
    }

    this.busy = true
    this.element.setAttribute("aria-busy", "true")
    this.element.setAttribute("aria-disabled", "true")
    this.labelTarget.textContent = this.preparingLabelValue
    this.resetTimeout = window.setTimeout(this.reset, 1000)
  }

  reset() {
    window.clearTimeout(this.resetTimeout)
    this.busy = false
    this.element.setAttribute("aria-busy", "false")
    this.element.setAttribute("aria-disabled", "false")
    this.labelTarget.textContent = this.readyLabelValue
  }
}
