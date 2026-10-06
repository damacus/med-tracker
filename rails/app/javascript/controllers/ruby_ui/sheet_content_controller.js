import { Controller } from "@hotwired/stimulus"

let sheetContentId = 0

export default class extends Controller {
  connect() {
    this.handleKeydown = this.handleKeydown.bind(this)
    document.addEventListener('keydown', this.handleKeydown)
    this.connectNaming()
  }

  disconnect() {
    document.removeEventListener('keydown', this.handleKeydown)
    this.updateBodyScrollLock()
  }

  handleKeydown(event) {
    if (event.key === 'Escape') {
      this.close()
    } else if (event.key === 'Tab') {
      this.trapFocus(event)
    }
  }

  close() {
    const backdrop = this.element.querySelector('[data-testid="drawer-backdrop"]')
    const panel = this.element.querySelector('[role="dialog"]')

    if (backdrop) backdrop.setAttribute('data-state', 'closed')
    if (panel) panel.setAttribute('data-state', 'closed')

    const sheetId = this.element.getAttribute('data-ruby-ui--sheet-content-sheet-id')
    const sheet = sheetId ? document.getElementById(sheetId) : null
    const sheetController = sheet && this.application.getControllerForElementAndIdentifier(sheet, 'ruby-ui--sheet')
    if (sheetController) sheetController.close()

    const duration = 300
    setTimeout(() => {
      this.element.remove()
      this.updateBodyScrollLock()
    }, duration)
  }

  connectNaming() {
    const panel = this.element.querySelector('[role="dialog"]')
    if (!panel) return

    sheetContentId += 1
    const idPrefix = `ruby-ui-sheet-content-${sheetContentId}`
    this.connectRelationship(
      panel,
      'aria-labelledby',
      panel.querySelector('[data-ruby-ui-sheet-title]'),
      `${idPrefix}-title`
    )
    this.connectRelationship(
      panel,
      'aria-describedby',
      panel.querySelector('[data-ruby-ui-sheet-description]'),
      `${idPrefix}-description`
    )
  }

  connectRelationship(panel, attribute, element, id) {
    if (!element || (attribute === 'aria-labelledby' && panel.hasAttribute('aria-label'))) {
      panel.removeAttribute(attribute)
      return
    }

    element.id = id
    panel.setAttribute(attribute, id)
  }

  trapFocus(event) {
    const panel = this.element.querySelector('[role="dialog"]')
    if (!panel) return

    const tabbableElements = Array.from(
      panel.querySelectorAll(
        'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), ' +
        'select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
      )
    ).filter((element) => this.usableTabbable(element, panel))
    const firstElement = tabbableElements[0]
    const lastElement = tabbableElements.at(-1)

    if (!firstElement || !lastElement) return

    if (event.shiftKey && document.activeElement === firstElement) {
      event.preventDefault()
      lastElement.focus({ preventScroll: true })
    } else if (!event.shiftKey && document.activeElement === lastElement) {
      event.preventDefault()
      firstElement.focus({ preventScroll: true })
    }
  }

  usableTabbable(element, panel) {
    if (element.tabIndex < 0 || element.matches(':disabled') || element.closest('[aria-hidden="true"], [hidden], [inert]')) {
      return false
    }

    for (let ancestor = element; ancestor && ancestor !== panel.parentElement; ancestor = ancestor.parentElement) {
      const style = window.getComputedStyle(ancestor)
      if (style.display === 'none' || style.visibility === 'hidden' || style.visibility === 'collapse') return false
    }

    return element.getClientRects().length > 0
  }

  updateBodyScrollLock() {
    document.body.classList.toggle(
      'overflow-hidden',
      Boolean(document.querySelector('[data-controller="ruby-ui--sheet-content"]'))
    )
  }
}
