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

      // Build RubyUI SelectItem markup
      const items = dosages.map((dosage) => {
        const text = `${dosage.amount} ${dosage.unit} - ${dosage.description}`
        return `
          <div
            role="option"
            tabindex="0"
            data-value="${dosage.id}"
            aria-selected="false"
            data-orientation="vertical"
            data-controller="ruby-ui--select-item"
            data-action="click->ruby-ui--select#selectItem keydown.enter->ruby-ui--select#selectItem keydown.down->ruby-ui--select#handleKeyDown keydown.up->ruby-ui--select#handleKeyUp keydown.esc->ruby-ui--select#handleEsc"
            data-ruby-ui__select-target="item"
            class="item group relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors focus:bg-accent focus:text-accent-foreground hover:bg-accent hover:text-accent-foreground"
          >
            ${text}
          </div>
        `
      }).join('')

      this.dosageSelectTarget.innerHTML = items
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
