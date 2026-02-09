import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static outlets = ["ruby-ui--sheet"]

  connect() {
    this.handleKeydown = this.handleKeydown.bind(this)
    document.addEventListener('keydown', this.handleKeydown)
  }

  disconnect() {
    document.removeEventListener('keydown', this.handleKeydown)
  }

  handleKeydown(event) {
    if (event.key === 'Escape') {
      this.close()
    }
  }

  close() {
    const backdrop = this.element.querySelector('[data-testid="drawer-backdrop"]')
    const panel = this.element.querySelector('[role="dialog"]')

    if (backdrop) backdrop.setAttribute('data-state', 'closed')
    if (panel) panel.setAttribute('data-state', 'closed')

    if (this.hasRubyUiSheetOutlet) {
      this.rubyUiSheetOutlet.close()
    }

    const duration = 300
    setTimeout(() => {
      this.element.remove()
    }, duration)
  }
}
