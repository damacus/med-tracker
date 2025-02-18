import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["template", "container"]

  add(event) {
    event.preventDefault()
    const newId = new Date().getTime()

    // Create a temporary container
    const temp = document.createElement('div')
    temp.innerHTML = this.templateTarget.innerHTML.replace(/NEW_RECORD/g, newId)

    // Get the option element
    const option = temp.querySelector('[data-dosage-options-target="option"]')
    if (option) {
      // Make sure it's visible
      option.style.cssText = 'display: flex !important'

      // Add it to the container
      this.containerTarget.appendChild(option)

      // Focus the input
      const input = option.querySelector('input')
      if (input) input.focus()
    }
  }

  remove(event) {
    event.preventDefault()
    const option = event.target.closest('[data-dosage-options-target="option"]')
    if (option) {
      const destroyField = option.querySelector('input[name*="_destroy"]')
      if (destroyField) {
        destroyField.value = '1'
        option.style.display = 'none'
      } else {
        option.remove()
      }
    }
  }
}
