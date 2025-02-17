import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["submit"]

  connect() {
	this.validate()
  }

  validate() {
	const requiredFields = [
	  'medicine_id',
	  'dosage',
	  'frequency',
	  'start_date'
	]

	const isValid = requiredFields.every(field => {
	  const input = this.element.querySelector(`[name*='[${field}]']`)
	  return input && input.value.trim() !== ''
	})

	this.submitTarget.disabled = !isValid
  }

  cancel(event) {
	event.preventDefault()
	this.element.closest('turbo-frame').src = null
  }
}
