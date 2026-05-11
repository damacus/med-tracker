import { Controller } from "@hotwired/stimulus"

const DESKTOP_QUERY = "(min-width: 768px)"

export default class extends Controller {
  connect() {
    this.mediaQuery = window.matchMedia(DESKTOP_QUERY)
    this.sync = this.sync.bind(this)
    this.mediaQuery.addEventListener?.("change", this.sync)
    this.mediaQuery.addListener?.(this.sync)
    window.addEventListener("resize", this.sync)
    this.sync()
  }

  disconnect() {
    this.mediaQuery?.removeEventListener?.("change", this.sync)
    this.mediaQuery?.removeListener?.(this.sync)
    window.removeEventListener("resize", this.sync)
  }

  sync() {
    const isDesktop = this.mediaQuery.matches

    this.toggleElement("sidebar", isDesktop, "flex")
    this.toggleElement("mobile-top-bar", !isDesktop, "block")
    this.toggleElement("mobile-rail", !isDesktop, "flex")
    this.toggleFloatingActionMenu(!isDesktop)
  }

  toggleElement(role, visible, display) {
    const element = this.element.querySelector(`[data-responsive-shell-role="${role}"]`)
    if (!element) return

    element.style.display = visible ? display : "none"
  }

  toggleFloatingActionMenu(visible) {
    const element = this.element.querySelector('[data-responsive-shell-role="floating-action-menu"]')
    const shell = this.element.querySelector(".floating-action-menu-shell")
    if (!element) return

    if (visible) {
      element.style.display = "block"
      shell.style.position = "fixed"
      shell.style.right = "calc(1rem + env(safe-area-inset-right))"
      shell.style.bottom = "calc(1.5rem + env(safe-area-inset-bottom))"
      shell.style.zIndex = "60"
      shell.style.display = "flex"
      shell.style.flexDirection = "column"
      shell.style.alignItems = "flex-end"
      shell.style.gap = "0.75rem"
    } else {
      element.style.display = "none"
    }
  }
}
