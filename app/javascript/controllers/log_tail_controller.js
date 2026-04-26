import { Controller } from "@hotwired/stimulus"

// Keeps a scrollable log element pinned to its bottom so the most recent
// lines stay visible across Turbo renders and periodic reloads.
export default class extends Controller {
  connect() {
    this.pin()
    requestAnimationFrame(() => this.pin())
  }

  pin() {
    this.element.scrollTop = this.element.scrollHeight
  }
}
