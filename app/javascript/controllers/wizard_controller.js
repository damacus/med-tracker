import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["step", "indicator", "prevButton", "nextButton", "submitButton"]
  static values = { current: { type: Number, default: 0 } }

  connect() {
    this.showStep()
  }

  next() {
    if (!this.validateCurrentStep()) return
    if (this.currentValue < this.stepTargets.length - 1) {
      this.currentValue++
    }
  }

  prev() {
    if (this.currentValue > 0) {
      this.currentValue--
    }
  }

  goToStep(event) {
    const index = parseInt(event.currentTarget.dataset.wizardStep, 10)
    if (index < this.currentValue) {
      this.currentValue = index
    }
  }

  currentValueChanged() {
    this.showStep()
  }

  // Private

  showStep() {
    this.stepTargets.forEach((step, index) => {
      step.classList.toggle("hidden", index !== this.currentValue)
    })

    this.indicatorTargets.forEach((indicator, index) => {
      const circle = indicator.querySelector("[data-indicator-circle]")
      const label = indicator.querySelector("[data-indicator-label]")
      const line = indicator.querySelector("[data-indicator-line]")

      if (index < this.currentValue) {
        // Completed
        circle?.classList.remove("bg-slate-100", "text-slate-400", "bg-primary", "ring-4", "ring-primary/20", "scale-110")
        circle?.classList.add("bg-primary", "text-white")
        label?.classList.remove("text-slate-400", "text-primary", "font-bold")
        label?.classList.add("text-primary", "font-medium")
        if (line) {
          line.classList.remove("bg-slate-200")
          line.classList.add("bg-primary")
        }
      } else if (index === this.currentValue) {
        // Current
        circle?.classList.remove("bg-slate-100", "text-slate-400", "text-white")
        circle?.classList.add("bg-primary", "text-white", "ring-4", "ring-primary/20", "scale-110")
        label?.classList.remove("text-slate-400", "font-medium")
        label?.classList.add("text-primary", "font-bold")
      } else {
        // Future
        circle?.classList.remove("bg-primary", "text-white", "ring-4", "ring-primary/20", "scale-110", "font-bold")
        circle?.classList.add("bg-slate-100", "text-slate-400")
        label?.classList.remove("text-primary", "font-bold", "font-medium")
        label?.classList.add("text-slate-400")
        if (line) {
          line.classList.remove("bg-primary")
          line.classList.add("bg-slate-200")
        }
      }
    })

    // Toggle button visibility
    if (this.hasPrevButtonTarget) {
      this.prevButtonTarget.classList.toggle("invisible", this.currentValue === 0)
    }
    if (this.hasNextButtonTarget && this.hasSubmitButtonTarget) {
      const isLastStep = this.currentValue === this.stepTargets.length - 1
      this.nextButtonTarget.classList.toggle("hidden", isLastStep)
      this.submitButtonTarget.classList.toggle("hidden", !isLastStep)
    }
  }

  validateCurrentStep() {
    const currentStepEl = this.stepTargets[this.currentValue]
    if (!currentStepEl) return true

    let valid = true

    // Validate visible text/number/etc inputs (not radios — they live inside combobox popovers)
    currentStepEl.querySelectorAll(
      "input[required]:not([type=radio]):not([type=hidden]), select[required], textarea[required]"
    ).forEach((input) => {
      if (!input.checkValidity()) {
        input.reportValidity()
        valid = false
      }
    })

    // Validate radio groups as a whole — a group is valid if any radio is checked
    const radioGroups = new Map()
    currentStepEl.querySelectorAll("input[type=radio][required]").forEach((radio) => {
      if (!radioGroups.has(radio.name)) {
        radioGroups.set(radio.name, false)
      }
      if (radio.checked) {
        radioGroups.set(radio.name, true)
      }
    })

    radioGroups.forEach((isChecked) => {
      if (!isChecked) valid = false
    })

    return valid
  }
}
