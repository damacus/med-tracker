import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    // Show the modal
    this.element.showModal()
    
    // Prevent body scrolling when modal is open
    document.body.style.overflow = "hidden"
    
    // Focus the first focusable element
    const focusable = this.element.querySelector('button, [href], input, select, textarea')
    if (focusable) focusable.focus()
    
    // Handle escape key
    this.boundKeyHandler = this.handleKeyUp.bind(this)
    document.addEventListener("keyup", this.boundKeyHandler)
  }

  disconnect() {
    // Close the modal
    this.element.close()
    
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

  closeOnBackdropClick(event) {
    if (event.target === this.element) {
      this.close()
    }
  }

  handleKeyUp(event) {
    if (event.key === "Escape") {
      this.close()
    }
  }
}
