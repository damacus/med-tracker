import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  navigateToType(event) {
    const frame = document.getElementById('modal')
    if (frame) frame.src = event.target.dataset.url
  }
}
