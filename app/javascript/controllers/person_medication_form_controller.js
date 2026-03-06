import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static values = { currentStep: Number }
  static targets = [
    "medicationSelect", "doseOptionInput", "doseAmountInput", "doseUnitInput",
    "maxDosesInput", "minHoursInput", "doseCycleInput", "stepPanel",
    "stepIndicator", "prevButton", "nextButton", "submitButton",
    "selectedMedicationName", "selectedDoseName"
  ]

  connect() {
    if (!this.hasCurrentStepValue) {
      this.currentStepValue = 1
    }
    if (this.#selectedMedication()) {
      this.updateDefaults()
    }
    this.#syncMedicationSummary()
    this.#syncDoseSummary()
    this.#refreshWorkflow()
  }

  async updateDefaults() {
    const checkedRadio = this.#selectedMedication()
    const medicationId = checkedRadio ? checkedRadio.value : null
    const personType = this.element.dataset.personType || 'adult'

    if (!medicationId) {
      this.#clearDose()
      this.#syncMedicationSummary()
      this.#syncDoseSummary()
      this.#refreshWorkflow()
      return
    }

    const fallbackDose = this.#fallbackDoseFor(checkedRadio)
    if (fallbackDose) {
      this.#renderDoseOptions([fallbackDose])
      if (this.hasDoseOptionInputTarget) {
        this.doseOptionInputTarget.value = `${fallbackDose.amount}|${fallbackDose.unit}`
      }
      this.#applyDose(fallbackDose)
    }
    if (this.currentStepValue === 1) {
      this.currentStepValue = 2
    }
    this.#syncMedicationSummary()
    this.#syncDoseSummary()
    this.#refreshWorkflow()

    try {
      const response = await fetch(`/medications/${medicationId}/dosages.json`)
      const dosages = await response.json()
      const options = dosages.length > 0 ? dosages : (fallbackDose ? [fallbackDose] : [])

      this.#renderDoseOptions(options)
      if (options.length === 0) {
        this.#clearDose()
        this.#syncDoseSummary()
        this.#refreshWorkflow()
        return
      }

      const isChild = personType === 'minor' || personType === 'dependent_adult'
      const currentValue = this.hasDoseAmountInputTarget && this.hasDoseUnitInputTarget
        ? `${this.doseAmountInputTarget.value}|${this.doseUnitInputTarget.value}`
        : null
      const defaultDosage = options.find(d => `${d.amount}|${d.unit}` === currentValue)
        || options.find(d => isChild ? d.default_for_children : d.default_for_adults)
        || options[0]

      if (defaultDosage) {
        if (this.hasDoseOptionInputTarget) {
          this.doseOptionInputTarget.value = `${defaultDosage.amount}|${defaultDosage.unit}`
        }
        this.#applyDose(defaultDosage)
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
      this.#syncDoseSummary()
      this.#refreshWorkflow()
    } catch (error) {
      console.error('Error fetching dosages:', error)
    }
  }

  selectDose() {
    if (!this.hasDoseOptionInputTarget) return

    const selected = this.doseOptionInputTarget.selectedOptions[0]
    if (!selected || !selected.value) return

    this.#applyDose({
      amount: selected.dataset.amount,
      unit: selected.dataset.unit
    })
    this.#syncDoseSummary()
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

  #selectedMedication() {
    return this.medicationSelectTargets.find(target => target.checked)
  }

  #fallbackDoseFor(checkedRadio) {
    if (!checkedRadio?.dataset.doseAmount || !checkedRadio?.dataset.doseUnit) return null

    return {
      id: 'standard',
      amount: checkedRadio.dataset.doseAmount,
      unit: checkedRadio.dataset.doseUnit
    }
  }

  #renderDoseOptions(options) {
    if (!this.hasDoseOptionInputTarget) return

    this.doseOptionInputTarget.innerHTML = ''

    const placeholder = document.createElement('option')
    placeholder.value = ''
    placeholder.textContent = options.length > 0 ? 'Select a dose' : 'No doses available'
    this.doseOptionInputTarget.appendChild(placeholder)
    this.doseOptionInputTarget.disabled = options.length === 0

    options.forEach((dose) => {
      const option = document.createElement('option')
      option.value = `${dose.amount}|${dose.unit}`
      const amount = this.#formatAmount(dose.amount)
      const description = this.#displayDoseDescription(dose.description)
      option.textContent = description ? `${amount} ${dose.unit} - ${description}` : `${amount} ${dose.unit}`
      option.dataset.amount = dose.amount
      option.dataset.unit = dose.unit
      this.doseOptionInputTarget.appendChild(option)
    })
  }

  #applyDose(dose) {
    if (this.hasDoseAmountInputTarget) {
      this.doseAmountInputTarget.value = dose.amount
    }
    if (this.hasDoseUnitInputTarget) {
      this.doseUnitInputTarget.value = dose.unit
    }
    this.#syncDoseSummary()
  }

  #clearDose() {
    if (this.hasDoseAmountInputTarget) {
      this.doseAmountInputTarget.value = ''
    }
    if (this.hasDoseUnitInputTarget) {
      this.doseUnitInputTarget.value = ''
    }
    this.#syncDoseSummary()
  }

  #formatAmount(amount) {
    const numericAmount = Number.parseFloat(amount)
    if (Number.isNaN(numericAmount)) return amount

    return Number.isInteger(numericAmount) ? numericAmount.toString() : numericAmount.toString()
  }

  #displayDoseDescription(description) {
    if (!description) return null
    if (/^standard\b.*dose$/i.test(description)) return null

    return description
  }

  #refreshWorkflow() {
    if (!this.hasStepPanelTarget) return

    this.stepPanelTargets.forEach((panel) => {
      const step = Number.parseInt(panel.dataset.step, 10)
      panel.classList.toggle('hidden', step !== this.currentStepValue)
    })

    if (this.hasStepIndicatorTarget) {
      this.stepIndicatorTargets.forEach((indicator, index) => {
        const active = index + 1 <= this.currentStepValue
        indicator.classList.toggle('bg-slate-900', active)
        indicator.classList.toggle('bg-slate-200', !active)
      })
    }

    if (this.hasPrevButtonTarget) {
      this.prevButtonTarget.classList.toggle('hidden', this.currentStepValue === 1)
      this.prevButtonTarget.hidden = this.currentStepValue === 1
    }

    if (this.hasNextButtonTarget) {
      const finalStep = this.currentStepValue === 3
      this.nextButtonTarget.classList.toggle('hidden', finalStep)
      this.nextButtonTarget.hidden = finalStep
      this.nextButtonTarget.disabled = !this.#currentStepValid()
    }

    if (this.hasSubmitButtonTarget) {
      const finalStep = this.currentStepValue === 3
      this.submitButtonTarget.classList.toggle('hidden', !finalStep)
      this.submitButtonTarget.hidden = !finalStep
    }
  }

  #currentStepValid() {
    if (this.currentStepValue === 1) {
      return !!this.#selectedMedication()
    }

    if (this.currentStepValue === 2) {
      return this.hasDoseAmountInputTarget && this.hasDoseUnitInputTarget &&
        this.doseAmountInputTarget.value !== '' && this.doseUnitInputTarget.value !== ''
    }

    return true
  }

  #syncMedicationSummary() {
    if (!this.hasSelectedMedicationNameTarget) return

    const selectedMedication = this.#selectedMedication()
    const label = selectedMedication?.dataset.text || 'Choose a medication'
    this.selectedMedicationNameTargets.forEach((target) => { target.textContent = label })
  }

  #syncDoseSummary() {
    if (!this.hasSelectedDoseNameTarget) return

    let label = 'Choose a dose'
    if (this.hasDoseAmountInputTarget && this.hasDoseUnitInputTarget &&
      this.doseAmountInputTarget.value !== '' && this.doseUnitInputTarget.value !== '') {
      label = `${this.#formatAmount(this.doseAmountInputTarget.value)} ${this.doseUnitInputTarget.value}`
    }
    this.selectedDoseNameTargets.forEach((target) => { target.textContent = label })
  }
}
