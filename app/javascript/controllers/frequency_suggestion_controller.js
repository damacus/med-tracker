import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["input"]

  suggest(event) {
    event.preventDefault()
    const suggestion = event.params.suggestion
    if (this.hasInputTarget && suggestion) {
      this.inputTarget.value = suggestion
      this.inputTarget.focus()
    }
  }
}
