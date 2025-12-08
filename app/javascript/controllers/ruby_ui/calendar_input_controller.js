import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  setValue(value) {
    this.element.value = value
  }
}
