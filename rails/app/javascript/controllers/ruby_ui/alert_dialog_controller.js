import { Controller } from "@hotwired/stimulus"

let alertDialogId = 0

// Connects to data-controller="ruby-ui--alert-dialog"
export default class extends Controller {
  static targets = ["content"]
  static values = {
    open: {
      type: Boolean,
      default: false,
    },
  }

  connect() {
    this.sourceFrame = this.element.closest("turbo-frame")
    this.parentDropdownContent = this.element.closest("[data-ruby-ui--dropdown-menu-target='content']")

    if (this.openValue) {
      this.open()
    }
  }

  disconnect() {
    this.restoreParentDropdown()

    if (this.contentTarget.open) {
      this.contentTarget.close()
    }

    this.updateBodyScrollLock()
  }

  open(e) {
    e?.preventDefault()

    if (this.contentTarget.open) return

    this.connectNaming()
    this.contentTarget.hidden = false
    this.contentTarget.showModal()
    this.hideParentDropdown()
    this.updateBodyScrollLock()
  }

  dismiss(e) {
    e?.preventDefault()
    this.restoreParentDropdown()

    if (this.contentTarget.open) {
      this.contentTarget.close()
    }
    this.contentTarget.hidden = true
    this.updateBodyScrollLock()

    if (this.sourceFrame?.id === "modal") {
      this.sourceFrame.removeAttribute("src")
      this.sourceFrame.innerHTML = ""
    }

  }

  trapFocus(e) {
    if (e.key !== "Tab") return

    const tabbableElements = Array.from(
      this.contentTarget.querySelectorAll(
        'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
      )
    ).filter((element) => this.usableTabbable(element))
    const firstElement = tabbableElements[0]
    const lastElement = tabbableElements.at(-1)

    if (!firstElement || !lastElement) return

    if (e.shiftKey && document.activeElement === firstElement) {
      e.preventDefault()
      lastElement.focus({ preventScroll: true })
    } else if (!e.shiftKey && document.activeElement === lastElement) {
      e.preventDefault()
      firstElement.focus({ preventScroll: true })
    }
  }

  connectNaming() {
    alertDialogId += 1
    const idPrefix = `ruby-ui-alert-dialog-${alertDialogId}`

    this.connectRelationship(
      "aria-labelledby",
      this.contentTarget.querySelector("[data-ruby-ui-alert-dialog-title]"),
      `${idPrefix}-title`
    )
    this.connectRelationship(
      "aria-describedby",
      this.contentTarget.querySelector("[data-ruby-ui-alert-dialog-description]"),
      `${idPrefix}-description`
    )
  }

  updateBodyScrollLock() {
    document.body.classList.toggle("overflow-hidden", Boolean(document.querySelector("dialog:modal")))
  }

  hideParentDropdown() {
    if (!this.parentDropdownContent) return

    this.parentDropdownContent.style.visibility = "hidden"
    this.contentTarget.style.visibility = "visible"
  }

  restoreParentDropdown() {
    this.parentDropdownContent?.style.removeProperty("visibility")
    this.contentTarget.style.removeProperty("visibility")
  }

  usableTabbable(element) {
    if (element.tabIndex < 0 || element.matches(":disabled") || element.closest('[aria-hidden="true"], [hidden], [inert]')) {
      return false
    }

    for (let ancestor = element; ancestor && ancestor !== this.contentTarget.parentElement; ancestor = ancestor.parentElement) {
      const style = window.getComputedStyle(ancestor)

      if (style.display === "none" || style.visibility === "hidden" || style.visibility === "collapse") {
        return false
      }
    }

    return element.getClientRects().length > 0
  }

  connectRelationship(attribute, element, id) {
    if (!element) {
      this.contentTarget.removeAttribute(attribute)
      return
    }

    element.id = id
    this.contentTarget.setAttribute(attribute, id)
  }
}
