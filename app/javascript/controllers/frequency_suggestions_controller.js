import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["input", "maxDoses", "minHours", "doseCycle"]

  suggest(event) {
    event.preventDefault()

    const template = event.currentTarget.dataset

    if (this.hasInputTarget) {
      this.inputTarget.value = template.frequencySuggestionsFrequencyValue || template.suggestion || ''
      this.inputTarget.focus()
    }
    if (this.hasMaxDosesTarget) this.maxDosesTarget.value = template.frequencySuggestionsMaxDosesValue || ''
    if (this.hasMinHoursTarget) this.minHoursTarget.value = template.frequencySuggestionsMinHoursValue || ''
    if (this.hasDoseCycleTarget) this.doseCycleTarget.value = template.frequencySuggestionsDoseCycleValue || ''
  }
}
