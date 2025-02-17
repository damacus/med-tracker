import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  submit(event) {
    // Don't submit if it's a keypress event and the key is not Enter
    if (event.type === "keypress" && event.key !== "Enter") {
      return
    }
    
    // Submit the form
    this.element.requestSubmit()
  }
}
