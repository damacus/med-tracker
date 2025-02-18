import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    // Prevent body scrolling when modal is open
    document.body.style.overflow = "hidden"
    
    // Handle escape key
    this.boundKeyHandler = this.handleKeyUp.bind(this)
    document.addEventListener("keyup", this.boundKeyHandler)
    
    // Show animation
    this.element.dataset.show = "true"
  }

  disconnect() {
    // Restore body scrolling
    document.body.style.overflow = ""
    document.removeEventListener("keyup", this.boundKeyHandler)
  }

  close() {
    // Play close animation then remove
    this.element.dataset.show = "false"
    setTimeout(() => {
      this.element.remove()
    }, 200)
  }

  handleKeyUp(event) {
    if (event.key === "Escape") {
      this.close()
    }
  }
}
