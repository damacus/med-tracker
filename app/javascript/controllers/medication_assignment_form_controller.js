import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static values = {
    currentStep: Number,
    options: Object,
    translations: Object,
    startDate: String,
    endDate: String
  }

  static targets = [
    "medicationSelect", "doseOptionInput", "sourceDosageOptionIdInput",
    "doseAmountInput", "doseUnitInput", "stepPanel", "stepIndicator",
    "prevButton", "nextButton", "submitButton", "selectedMedicationName",
    "selectedDoseName", "reviewFrequency", "reviewMaxDoses", "reviewMinHours",
    "reviewDoseCycle", "reviewPlanType", "reviewActiveDates", "reviewActiveDatesItem"
  ]

  connect() {
    if (!this.hasCurrentStepValue) this.currentStepValue = 1
    this.updateMedication()
    this.#refreshWorkflow()
  }

  updateMedication(event) {
    const preserveDose = !event
    const medication = this.#selectedMedicationPayload()

    if (!medication) {
      this.#renderDoseOptions([])
      this.#clearDose()
      this.#syncMedicationSummary()
      this.#syncDoseSummary(null)
      this.#syncReview(null, null)
      this.#refreshWorkflow()
      return
    }

    if (!preserveDose) this.#clearDose()
    this.#renderDoseOptions(medication.dose_options || [])

    const currentValue = preserveDose ? this.#currentDoseOptionValue() : ""
    const currentOption = this.#doseOptions().find((dose) => this.#optionValue(dose) === currentValue)
    if (currentOption) {
      this.doseOptionInputTarget.value = this.#optionValue(currentOption)
      this.#applyDose(currentOption)
    }

    if (event && this.currentStepValue === 1) this.currentStepValue = 2
    this.#syncMedicationSummary()
    this.#syncDoseSummary(currentOption)
    this.#syncReview(medication, currentOption)
    this.#refreshWorkflow()
  }

  selectDose() {
    const medication = this.#selectedMedicationPayload()
    const option = this.#selectedDoseOption()

    if (!option) {
      this.#clearDose()
      this.#syncDoseSummary(null)
      this.#syncReview(medication, null)
      this.#refreshWorkflow()
      return
    }

    this.#applyDose(option)
    this.#syncDoseSummary(option)
    this.#syncReview(medication, option)
    this.#refreshWorkflow()
  }

  nextStep() {
    if (!this.#currentStepValid()) return
    if (this.currentStepValue >= 3) return

    this.currentStepValue += 1
    this.#refreshWorkflow()
  }

  prevStep() {
    if (this.currentStepValue <= 1) return

    this.currentStepValue -= 1
    this.#refreshWorkflow()
  }

  cancel(event) {
    event.preventDefault()
    const frame = this.element.closest("turbo-frame")
    if (frame) {
      frame.removeAttribute("src")
      frame.innerHTML = ""
    }
  }

  #selectedMedication() {
    return this.medicationSelectTargets.find((target) => target.checked)
  }

  #selectedMedicationPayload() {
    const selected = this.#selectedMedication()
    if (!selected) return null

    return this.optionsValue?.[selected.value] || null
  }

  #doseOptions() {
    return this.#selectedMedicationPayload()?.dose_options || []
  }

  #selectedDoseOption() {
    if (!this.hasDoseOptionInputTarget || !this.doseOptionInputTarget.value) return null

    return this.#doseOptions().find((dose) => this.#optionValue(dose) === this.doseOptionInputTarget.value) || null
  }

  #renderDoseOptions(options) {
    if (!this.hasDoseOptionInputTarget) return

    this.doseOptionInputTarget.innerHTML = ""

    const placeholder = document.createElement("option")
    placeholder.value = ""
    placeholder.textContent = options.length > 0 ? this.t("selectDose") : this.t("noDosesAvailable")
    this.doseOptionInputTarget.appendChild(placeholder)
    this.doseOptionInputTarget.disabled = options.length === 0

    options.forEach((dose) => {
      const option = document.createElement("option")
      option.value = this.#optionValue(dose)
      option.textContent = this.#doseLabel(dose)
      if (dose.id) option.dataset.id = dose.id
      option.dataset.amount = dose.amount
      option.dataset.unit = dose.unit
      this.doseOptionInputTarget.appendChild(option)
    })
  }

  #applyDose(dose) {
    if (this.hasSourceDosageOptionIdInputTarget) {
      this.sourceDosageOptionIdInputTarget.value = dose.id || ""
    }
    if (this.hasDoseAmountInputTarget) {
      this.doseAmountInputTarget.value = dose.amount || ""
    }
    if (this.hasDoseUnitInputTarget) {
      this.doseUnitInputTarget.value = dose.unit || ""
    }
  }

  #clearDose() {
    if (this.hasSourceDosageOptionIdInputTarget) this.sourceDosageOptionIdInputTarget.value = ""
    if (this.hasDoseAmountInputTarget) this.doseAmountInputTarget.value = ""
    if (this.hasDoseUnitInputTarget) this.doseUnitInputTarget.value = ""
    if (this.hasDoseOptionInputTarget) this.doseOptionInputTarget.value = ""
  }

  #currentDoseOptionValue() {
    if (this.hasSourceDosageOptionIdInputTarget && this.sourceDosageOptionIdInputTarget.value !== "") {
      return this.sourceDosageOptionIdInputTarget.value
    }

    if (this.hasDoseAmountInputTarget && this.hasDoseUnitInputTarget &&
      this.doseAmountInputTarget.value !== "" && this.doseUnitInputTarget.value !== "") {
      return `${this.doseAmountInputTarget.value}|${this.doseUnitInputTarget.value}`
    }

    return null
  }

  #optionValue(dose) {
    if (dose.option_value) return String(dose.option_value)
    if (dose.id) return String(dose.id)

    return `${dose.amount}|${dose.unit}`
  }

  #doseLabel(dose) {
    const amount = this.#formatAmount(dose.amount)
    const description = dose.description ? ` - ${dose.description}` : ""
    return `${amount} ${dose.unit}${description}`
  }

  #formatAmount(amount) {
    const numericAmount = Number.parseFloat(amount)
    if (Number.isNaN(numericAmount)) return amount

    return Number.isInteger(numericAmount) ? numericAmount.toString() : numericAmount.toString()
  }

  #refreshWorkflow() {
    if (!this.hasStepPanelTarget) return

    this.stepPanelTargets.forEach((panel) => {
      const step = Number.parseInt(panel.dataset.step, 10)
      panel.classList.toggle("hidden", step !== this.currentStepValue)
    })

    if (this.hasStepIndicatorTarget) {
      this.stepIndicatorTargets.forEach((indicator, index) => {
        const active = index + 1 <= this.currentStepValue
        indicator.classList.toggle("bg-foreground", active)
        indicator.classList.toggle("bg-primary/15", !active)
      })
    }

    if (this.hasPrevButtonTarget) {
      const firstStep = this.currentStepValue === 1
      this.prevButtonTarget.classList.toggle("hidden", firstStep)
      this.prevButtonTarget.hidden = firstStep
    }

    if (this.hasNextButtonTarget) {
      const finalStep = this.currentStepValue === 3
      this.nextButtonTarget.classList.toggle("hidden", finalStep)
      this.nextButtonTarget.hidden = finalStep
      this.nextButtonTarget.disabled = !this.#currentStepValid()
    }

    if (this.hasSubmitButtonTarget) {
      const finalStep = this.currentStepValue === 3
      this.submitButtonTarget.classList.toggle("hidden", !finalStep)
      this.submitButtonTarget.hidden = !finalStep
    }
  }

  #currentStepValid() {
    if (this.currentStepValue === 1) return !!this.#selectedMedication()

    if (this.currentStepValue === 2) {
      return this.hasDoseAmountInputTarget && this.hasDoseUnitInputTarget &&
        this.doseAmountInputTarget.value !== "" && this.doseUnitInputTarget.value !== ""
    }

    return true
  }

  #syncMedicationSummary() {
    if (!this.hasSelectedMedicationNameTarget) return

    const selected = this.#selectedMedication()
    const label = selected?.dataset.text || this.t("chooseMedication")
    this.selectedMedicationNameTargets.forEach((target) => { target.textContent = label })
  }

  #syncDoseSummary(dose) {
    if (!this.hasSelectedDoseNameTarget) return

    const label = dose ? `${this.#formatAmount(dose.amount)} ${dose.unit}` : this.t("chooseDose")
    this.selectedDoseNameTargets.forEach((target) => { target.textContent = label })
  }

  #syncReview(medication, dose) {
    this.#setReviewText("reviewFrequency", dose?.frequency ?? this.t("notSet"))
    this.#setReviewText("reviewMaxDoses", dose?.default_max_daily_doses ?? this.t("notSet"))
    this.#setReviewText("reviewMinHours", dose?.default_min_hours_between_doses ?? this.t("notSet"))
    this.#setReviewText("reviewDoseCycle", dose?.default_dose_cycle ?? this.t("notSet"))
    this.#setReviewText("reviewPlanType", medication?.plan_type_label ?? this.t("notSet"))
    this.#setReviewText("reviewActiveDates", `${this.startDateValue} to ${this.endDateValue}`)
    this.#toggleReviewActiveDates(medication)
  }

  #setReviewText(targetName, value) {
    const targets = this[`${targetName}Targets`]
    if (!targets) return

    targets.forEach((target) => { target.textContent = value })
  }

  #toggleReviewActiveDates(medication) {
    if (!this.hasReviewActiveDatesItemTarget) return

    const hidden = medication?.direct_plan === true
    this.reviewActiveDatesItemTargets.forEach((target) => {
      target.classList.toggle("hidden", hidden)
    })
  }

  t(key) {
    return this.translationsValue?.[key] || ""
  }
}
