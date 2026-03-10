import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["input"]

  suggest(event) {
    event.preventDefault()
    if (this.hasInputTarget) {
      this.inputTarget.value = event.currentTarget.dataset.suggestion
      this.inputTarget.focus()
    }
  }
}
