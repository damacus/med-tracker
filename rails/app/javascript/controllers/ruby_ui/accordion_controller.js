import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["icon", "content", "trigger"]
  static values = {
    open: { type: Boolean, default: false },
    animationDuration: { type: Number, default: 0.15 },
    animationEasing: { type: String, default: "ease-in-out" },
    rotateIcon: { type: Number, default: 180 }
  }

  connect() {
    if (this.openValue) {
      this.contentTarget.removeAttribute("hidden")
      this.contentTarget.dataset.state = "open"
      this.contentTarget.style.height = "auto"
      this.setExpanded(true)
      this.rotateIcon(true)
    } else {
      this.contentTarget.setAttribute("hidden", "")
      this.contentTarget.dataset.state = "closed"
      this.contentTarget.style.height = "0px"
      this.setExpanded(false)
      this.rotateIcon(false)
    }
  }

  toggle(event) {
    event.currentTarget.getAttribute("aria-expanded") === "true" ? this.close() : this.open()
  }

  open() {
    if (!this.hasContentTarget) return

    this.contentTarget.removeAttribute("hidden")
    this.contentTarget.dataset.state = "open"
    this.setExpanded(true)
    this.rotateIcon(true)
    const height = this.contentTarget.scrollHeight
    const animation = this.contentTarget.animate(
      [{ height: "0px" }, { height: `${height}px` }],
      this.animationOptions()
    )
    animation.finished
      .then(() => {
        if (this.contentTarget.dataset.state === "open") this.contentTarget.style.height = "auto"
        animation.cancel()
      })
      .catch(() => {})
    this.openValue = true
  }

  close() {
    if (!this.hasContentTarget) return

    const content = this.contentTarget
    content.dataset.state = "closed"
    this.setExpanded(false)
    this.rotateIcon(false)
    const height = content.getBoundingClientRect().height
    const animation = content.animate(
      [{ height: `${height}px` }, { height: "0px" }],
      this.animationOptions()
    )
    animation.finished
      .then(() => {
        if (content.dataset.state === "closed") {
          content.style.height = "0px"
          content.setAttribute("hidden", "")
        }
        animation.cancel()
      })
      .catch(() => {})
    this.openValue = false
  }

  setExpanded(expanded) {
    if (this.hasTriggerTarget) this.triggerTarget.setAttribute("aria-expanded", expanded.toString())
  }

  rotateIcon(open) {
    if (!this.hasIconTarget) return

    this.iconTarget.animate(
      [{ rotate: open ? "0deg" : `${this.rotateIconValue}deg` },
       { rotate: open ? `${this.rotateIconValue}deg` : "0deg" }],
      { ...this.animationOptions(), fill: "forwards" }
    )
  }

  animationOptions() {
    return { duration: this.animationDurationValue * 1000, easing: this.animationEasingValue }
  }
}
