import { Controller } from "@hotwired/stimulus"

let sheetId = 0

export default class extends Controller {
  static targets = ["content"]

  disconnect() {
    this.wrapper?.remove()
    document.body.classList.toggle(
      "overflow-hidden",
      Boolean(document.querySelector('[data-controller="ruby-ui--sheet-content"]'))
    )
  }

  open(event) {
    if (this.wrapper && this.wrapper.isConnected) return

    this.trigger = event?.currentTarget?.querySelector("button, a, [tabindex]") || event?.currentTarget
    if (!this.element.id) {
      sheetId += 1
      this.element.id = `ruby-ui-sheet-${sheetId}`
    }

    if (this.trigger) {
      this.trigger.classList.toggle('is-active', this.trigger.classList.contains('hamburger'))
      this.trigger.setAttribute('aria-expanded', 'true')
    }

    const wrapper = document.createElement("div")
    wrapper.setAttribute("data-controller", "ruby-ui--sheet-content")
    wrapper.setAttribute("data-ruby-ui--sheet-content-sheet-id", this.element.id || "")
    wrapper.style.cssText = "position:fixed;inset:0;z-index:50;pointer-events:none;"
    wrapper.innerHTML = this.contentTarget.innerHTML
    document.body.appendChild(wrapper)
    this.wrapper = wrapper
    document.body.classList.add("overflow-hidden")

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

    if (this.trigger) {
      this.trigger.classList.remove('is-active')
      this.trigger.setAttribute('aria-expanded', 'false')
      this.trigger.focus({ preventScroll: true })
    }
  }
}
