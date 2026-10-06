import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["checkbox", "hint"]

  toggleHint() {
    if (this.checkboxTarget.checked) {
      this.hintTarget.classList.add("hidden")
    } else {
      this.hintTarget.classList.remove("hidden")
    }
  }
}
