import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["item", "menu", "toggle"]

  connect() {
    this.open = false
    this.handleKeydown = this.handleKeydown.bind(this)
    this.close()
  }

  toggle(event) {
    event.preventDefault()
    this.open ? this.close() : this.expand()
  }

  expand() {
    this.open = true
    this.element.dataset.open = "true"
    this.menuTarget.hidden = false
    this.menuTarget.setAttribute("aria-hidden", "false")
    this.toggleTarget.setAttribute("aria-expanded", "true")
    this.toggleTarget.setAttribute("aria-label", "Close quick actions")
    document.addEventListener("keydown", this.handleKeydown)
  }

  close(event) {
    event?.preventDefault?.()
    this.open = false
    this.element.dataset.open = "false"
    this.menuTarget.hidden = true
    this.menuTarget.setAttribute("aria-hidden", "true")
    this.toggleTarget.setAttribute("aria-expanded", "false")
    this.toggleTarget.setAttribute("aria-label", "Open quick actions")
    document.removeEventListener("keydown", this.handleKeydown)
  }

  closeAndNavigate() {
    this.close()
  }

  closeOnOutsideClick(event) {
    return unless this.open
    return if this.element.contains(event.target)

    this.close()
  }

  disconnect() {
    document.removeEventListener("keydown", this.handleKeydown)
  }

  handleKeydown(event) {
    this.close(event) if event.key === "Escape"
  }
}
