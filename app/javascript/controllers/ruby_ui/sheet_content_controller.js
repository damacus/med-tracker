import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
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

    const trigger = document.querySelector('.hamburger.is-active')
    if (trigger) {
      trigger.classList.remove('is-active')
      trigger.setAttribute('aria-expanded', 'false')
    }

    const sheetController = this.application.getControllerForElementAndIdentifier(
      document.querySelector('[data-controller="ruby-ui--sheet"]'),
      'ruby-ui--sheet'
    )
    if (sheetController) sheetController.close()

    const duration = 300
    setTimeout(() => {
      this.element.remove()
    }, duration)
  }
}
