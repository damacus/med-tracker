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
    this.updateScheduleDefaults(template)
  }

  updateScheduleDefaults(template) {
    const scheduleTypeField = document.querySelector("input[name='medication[default_schedule_type]']")
    const scheduleConfigField = document.querySelector("input[name='medication[default_schedule_config]']")
    if (!scheduleTypeField || !scheduleConfigField) return

    const doseCycle = template.frequencySuggestionsDoseCycleValue || "daily"
    const times = String(template.times || "").split(",").map((time) => time.trim()).filter(Boolean)

    scheduleTypeField.value = doseCycle === "weekly" ? "weekly" : "multiple_daily"
    scheduleConfigField.value = JSON.stringify({
      schedule_type: scheduleTypeField.value,
      frequency: template.frequencySuggestionsFrequencyValue || "",
      times
    })
  }
}
