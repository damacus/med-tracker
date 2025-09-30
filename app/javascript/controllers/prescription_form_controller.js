import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["submit", "dosageSelect"]

  connect() {
	this.validate()
  }

  validate() {
	const requiredFields = [
	  'medicine_id',
	  'dosage_id',
	  'frequency',
	  'start_date'
	]

	const isValid = requiredFields.every(field => {
	  const input = this.element.querySelector(`[name*='[${field}]']`)
	  return input && input.value.trim() !== ''
	})

	if (this.hasSubmitTarget) {
	  this.submitTarget.disabled = !isValid
	}
  }

  async updateDosages(event) {
	const medicineId = event.target.value
	
	if (!medicineId || !this.hasDosageSelectTarget) {
	  return
	}

	try {
	  const response = await fetch(`/medicines/${medicineId}/dosages.json`)
	  const dosages = await response.json()
	  
	  // Clear existing options
	  this.dosageSelectTarget.innerHTML = '<option value="">Select a dosage</option>'
	  
	  // Add new options
	  dosages.forEach(dosage => {
		const option = document.createElement('option')
		option.value = dosage.id
		option.text = `${dosage.amount} ${dosage.unit} - ${dosage.description}`
		this.dosageSelectTarget.appendChild(option)
	  })
	  
	  this.validate()
	} catch (error) {
	  console.error('Error fetching dosages:', error)
	}
  }

  cancel(event) {
	event.preventDefault()
	this.element.closest('turbo-frame').src = null
  }
}
