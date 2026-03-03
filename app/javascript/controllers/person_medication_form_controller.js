import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["medicationSelect", "maxDosesInput", "minHoursInput", "doseCycleInput"]

  async updateDefaults() {
    const medicationId = this.medicationSelectTarget.value
    const personType = this.element.dataset.personType || 'adult'

    if (!medicationId) return

    try {
      const response = await fetch(`/medications/${medicationId}/dosages.json`)
      const dosages = await response.json()

      if (dosages.length === 0) return

      // Identify the default dosage for this person's type
      const isChild = personType === 'minor' || personType === 'dependent_adult'
      const defaultDosage = dosages.find(d => isChild ? d.default_for_children : d.default_for_adults)
        || dosages[0]

      if (defaultDosage) {
        if (this.hasMaxDosesInputTarget && !this.maxDosesInputTarget.value) {
          this.maxDosesInputTarget.value = defaultDosage.default_max_daily_doses || ''
        }
        if (this.hasMinHoursInputTarget && !this.minHoursInputTarget.value) {
          this.minHoursInputTarget.value = defaultDosage.default_min_hours_between_doses || ''
        }
        if (this.hasDoseCycleInputTarget && !this.doseCycleInputTarget.value && defaultDosage.default_dose_cycle !== null) {
          const cycleMap = { 0: 'daily', 1: 'weekly', 2: 'monthly' }
          const cycleValue = typeof defaultDosage.default_dose_cycle === 'number'
            ? cycleMap[defaultDosage.default_dose_cycle]
            : defaultDosage.default_dose_cycle
          if (cycleValue) this.doseCycleInputTarget.value = cycleValue
        }
      }
    } catch (error) {
      console.error('Error fetching dosages:', error)
    }
  }
}
