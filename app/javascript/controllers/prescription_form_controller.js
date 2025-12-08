import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["submit", "dosageSelect", "medicineSelect", "dosageContent", "dosageValue", "dosageTrigger"]

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
    // Get the medicine ID from the hidden input within the RubyUI Select
    const medicineInput = this.element.querySelector('[name="prescription[medicine_id]"]')
    const medicineId = medicineInput?.value

    if (!this.hasDosageContentTarget) {
      return
    }

    // Reset the dosage select value
    const dosageInput = this.element.querySelector('[name="prescription[dosage_id]"]')
    if (dosageInput) {
      dosageInput.value = ''
    }

    // Reset the displayed value and disable trigger if no medicine
    if (this.hasDosageValueTarget) {
      this.dosageValueTarget.textContent = medicineId ? 'Select a dosage' : 'Select a medicine first'
    }

    // Enable/disable the dosage trigger based on medicine selection
    if (this.hasDosageTriggerTarget) {
      this.dosageTriggerTarget.disabled = !medicineId
      this.dosageTriggerTarget.setAttribute('aria-disabled', !medicineId)
    }

    if (!medicineId) {
      // Clear dosage options
      const innerDiv = this.dosageContentTarget.querySelector('div')
      if (innerDiv) {
        innerDiv.innerHTML = ''
      }
      this.validate()
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
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" class="invisible group-aria-selected:visible mr-2 h-4 w-4 flex-none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"></path></svg>
            ${text}
          </div>
        `
      }).join('')

      // Update the inner div of SelectContent
      const innerDiv = this.dosageContentTarget.querySelector('div')
      if (innerDiv) {
        innerDiv.innerHTML = items
      }

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
