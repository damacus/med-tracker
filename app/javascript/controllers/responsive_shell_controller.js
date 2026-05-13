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
  }

  toggleElement(role, visible, display) {
    const element = this.element.querySelector(`[data-responsive-shell-role="${role}"]`)
    if (!element) return

    element.style.display = visible ? display : "none"
  }

}
