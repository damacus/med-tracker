import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static values = { nextUrl: String }
  static targets = [
    "submit", "dosageSelect", "medicationSelect", "dosageContent",
    "dosageValue", "dosageTrigger", "frequencyInput",
    "maxDosesInput", "minHoursInput", "doseCycleInput"
  ]

  connect() {
    this.dosageData = {}
    this.validate()
  }

  validate() {
    const medicationSelected = this.#fieldPresent('schedule[medication_id]')
    const dosageSelected = this.#fieldPresent('schedule[dosage_id]')
    const frequencyPresent = this.#fieldPresent('schedule[frequency]')
    const startDatePresent = this.#fieldPresent('schedule[start_date]')
    const isValid = medicationSelected && dosageSelected && frequencyPresent && startDatePresent

    if (this.hasSubmitTarget) {
      this.submitTarget.disabled = !isValid
    }
  }

  advanceToDetails() {
    const medicationInput = this.element.querySelector('[name="schedule[medication_id]"]')
    const medicationId = medicationInput?.value

    if (!medicationId || !this.hasNextUrlValue) return

    const frame = this.element.closest('turbo-frame')
    const url = new URL(this.nextUrlValue, window.location.origin)
    url.searchParams.set('medication_id', medicationId)
    if (frame) {
      frame.src = url.toString()
    } else {
      window.location.assign(url.toString())
    }
  }

  onDosageChange() {
    const selectedRadio = this.element.querySelector('[name="schedule[dosage_id]"]:checked')
    if (selectedRadio) {
      const dosage = {
        frequency: selectedRadio.dataset.frequency,
        default_max_daily_doses: selectedRadio.dataset.defaultMaxDailyDoses,
        default_min_hours_between_doses: selectedRadio.dataset.defaultMinHoursBetweenDoses,
        default_dose_cycle: selectedRadio.dataset.defaultDoseCycle
      }
      if (dosage.frequency && this.hasFrequencyInputTarget && !this.frequencyInputTarget.value) {
        this.frequencyInputTarget.value = dosage.frequency
      }
      this.#fillSchedulingDefaults(dosage)
      this.validate()
      return
    }

    const dosageInput = this.element.querySelector('[name="schedule[dosage_id]"]')
    const dosageId = dosageInput?.value
    if (dosageId && this.dosageData[dosageId]) {
      const dosage = this.dosageData[dosageId]
      if (dosage.frequency && this.hasFrequencyInputTarget && !this.frequencyInputTarget.value) {
        this.frequencyInputTarget.value = dosage.frequency
      }
      this.#fillSchedulingDefaults(dosage)
    }
    this.validate()
  }

  generateFrequency() {
    const max = parseInt(this.hasMaxDosesInputTarget ? this.maxDosesInputTarget.value : '') || null
    const hours = parseFloat(this.hasMinHoursInputTarget ? this.minHoursInputTarget.value : '') || null
    const cycle = this.hasDoseCycleInputTarget ? this.doseCycleInputTarget.value : ''

    if (!max && !hours) {
      this.validate()
      return
    }

    const parts = []
    if (max && cycle) {
      parts.push(max === 1 ? `Once ${cycle}` : `Up to ${max} times ${cycle}`)
    } else if (max) {
      parts.push(max === 1 ? 'Once per cycle' : `Up to ${max} times per cycle`)
    }
    if (hours) parts.push(`at least ${hours}h apart`)

    if (parts.length && this.hasFrequencyInputTarget) {
      this.frequencyInputTarget.value = parts.join(', ')
    }
    this.validate()
  }

  async updateDosages(event) {
    // Get the medication ID from the hidden input within the RubyUI Select
    const medicationInput = this.element.querySelector('[name="schedule[medication_id]"]')
    const medicationId = medicationInput?.value
    const personType = this.element.dataset.personType || 'adult'

    if (!this.hasDosageContentTarget) {
      return
    }

    // Reset the dosage select value
    const dosageInput = this.element.querySelector('[name="schedule[dosage_id]"]')
    if (dosageInput) {
      dosageInput.value = ''
    }

    // Reset the displayed value and disable trigger if no medication
    if (this.hasDosageValueTarget) {
      this.dosageValueTarget.textContent = medicationId ? 'Select a dosage' : 'Select a medication first'
    }

    // Enable/disable the dosage trigger based on medication selection
    if (this.hasDosageTriggerTarget) {
      this.dosageTriggerTarget.disabled = !medicationId
      this.dosageTriggerTarget.setAttribute('aria-disabled', !medicationId)
    }

    if (!medicationId) {
      // Clear dosage options and cached data
      const innerDiv = this.dosageContentTarget.querySelector('div')
      if (innerDiv) {
        innerDiv.innerHTML = ''
      }
      this.dosageData = {}
      this.validate()
      return
    }

    try {
      const response = await fetch(`/medications/${medicationId}/dosages.json`)
      const dosages = await response.json()

      // Cache dosage data for use in onDosageChange
      this.dosageData = {}
      dosages.forEach(d => { this.dosageData[String(d.id)] = d })

      // Identify the default dosage for this person's type
      const isChild = personType === 'minor' || personType === 'dependent_adult'
      const defaultDosage = dosages.find(d => isChild ? d.default_for_children : d.default_for_adults)
        || (dosages.length === 1 ? dosages[0] : null)

      // Auto-select default dosage if none already selected
      if (defaultDosage && dosageInput && !dosageInput.value) {
        dosageInput.value = String(defaultDosage.id)
        if (this.hasDosageValueTarget) {
          this.dosageValueTarget.textContent = `${defaultDosage.amount} ${defaultDosage.unit} - ${defaultDosage.description}`
        }
        this.#fillSchedulingDefaults(defaultDosage)
      }

      // Build RubyUI SelectItem markup
      const items = dosages.map((dosage) => {
        const text = `${dosage.amount} ${dosage.unit} - ${dosage.description}`
        const isSelected = dosageInput && dosageInput.value === String(dosage.id)
        return `
          <div
            role="option"
            tabindex="0"
            data-value="${dosage.id}"
            aria-selected="${isSelected}"
            data-orientation="vertical"
            data-controller="ruby-ui--select-item"
            data-action="click->ruby-ui--select#selectItem keydown.enter->ruby-ui--select#selectItem keydown.down->ruby-ui--select#handleKeyDown keydown.up->ruby-ui--select#handleKeyUp keydown.esc->ruby-ui--select#handleEsc"
            data-ruby-ui__select-target="item"
            class="item group relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors focus:bg-accent focus:text-accent-foreground hover:bg-accent hover:text-accent-foreground"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" class="${isSelected ? 'visible' : 'invisible'} group-aria-selected:visible mr-2 h-4 w-4 flex-none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"></path></svg>
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

  #fillSchedulingDefaults(dosage) {
    if (this.hasMaxDosesInputTarget && !this.maxDosesInputTarget.value && dosage.default_max_daily_doses) {
      this.maxDosesInputTarget.value = dosage.default_max_daily_doses
    }
    if (this.hasMinHoursInputTarget && !this.minHoursInputTarget.value && dosage.default_min_hours_between_doses) {
      this.minHoursInputTarget.value = dosage.default_min_hours_between_doses
    }
    if (this.hasDoseCycleInputTarget && !this.doseCycleInputTarget.value && dosage.default_dose_cycle !== null) {
      const cycleMap = { 0: 'daily', 1: 'weekly', 2: 'monthly' }
      const cycleValue = typeof dosage.default_dose_cycle === 'number'
        ? cycleMap[dosage.default_dose_cycle]
        : dosage.default_dose_cycle
      if (cycleValue) this.doseCycleInputTarget.value = cycleValue
    }
  }

  #fieldPresent(fieldName) {
    const checked = this.element.querySelector(`[name="${fieldName}"]:checked`)
    if (checked) return checked.value.trim() !== ''

    const input = this.element.querySelector(`[name="${fieldName}"]`)
    return !!(input && input.value.trim() !== '')
  }
}
