import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["content"]

  open() {
    if (this.wrapper && this.wrapper.isConnected) return

    const trigger = this.element.querySelector('.hamburger')
    if (trigger) {
      trigger.classList.add('is-active')
      trigger.setAttribute('aria-expanded', 'true')
    }

    const wrapper = document.createElement("div")
    wrapper.setAttribute("data-controller", "ruby-ui--sheet-content")
    wrapper.setAttribute("data-ruby-ui--sheet-content-sheet-id", this.element.id || "")
    wrapper.style.cssText = "position:fixed;inset:0;z-index:50;pointer-events:none;"
    wrapper.innerHTML = this.contentTarget.innerHTML
    document.body.appendChild(wrapper)
    this.wrapper = wrapper

    const backdrop = wrapper.querySelector('[data-testid="drawer-backdrop"]')
    const panel = wrapper.querySelector('[role="dialog"]')

    if (backdrop) {
      backdrop.setAttribute('data-state', 'closed')
      backdrop.offsetHeight
    }

    if (panel) {
      panel.setAttribute('data-state', 'closed')
      panel.offsetHeight
    }

    requestAnimationFrame(() => {
      if (backdrop) backdrop.setAttribute('data-state', 'open')
      if (panel) {
        panel.setAttribute('data-state', 'open')
        panel.focus()
      }
    })
  }

  close() {
    this.wrapper = null

    const trigger = this.element.querySelector('.hamburger')
    if (trigger) {
      trigger.classList.remove('is-active')
      trigger.setAttribute('aria-expanded', 'false')
    }
  }
}
