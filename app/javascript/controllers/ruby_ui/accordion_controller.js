import { Controller } from "@hotwired/stimulus"
import { animate } from "motion"

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
    animate(this.contentTarget, { height: `${height}px` }, this.animationOptions())
      .then(() => {
        if (this.contentTarget.dataset.state === "open") this.contentTarget.style.height = "auto"
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
    animate(content, { height: [`${height}px`, "0px"] }, this.animationOptions())
      .then(() => {
        if (content.dataset.state === "closed") content.setAttribute("hidden", "")
      })
      .catch(() => {})
    this.openValue = false
  }

  setExpanded(expanded) {
    if (this.hasTriggerTarget) this.triggerTarget.setAttribute("aria-expanded", expanded.toString())
  }

  rotateIcon(open) {
    if (!this.hasIconTarget) return

    animate(this.iconTarget, { rotate: `${open ? this.rotateIconValue : 0}deg` })
  }

  animationOptions() {
    return { duration: this.animationDurationValue, easing: this.animationEasingValue }
  }
}
