import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = [
    "button",
    "status",
    "appliedField",
    "confirmationPanel",
    "confirmationInput",
    "sourceList"
  ]

  static values = { url: String }

  async suggest(event) {
    event.preventDefault()

    this.setBusy(true)
    this.showStatus("Looking for trusted source guidance...")

    try {
      const response = await fetch(this.urlValue, {
        method: "POST",
        headers: {
          "Accept": "application/json",
          "Content-Type": "application/json",
          "X-CSRF-Token": this.csrfToken()
        },
        body: JSON.stringify({ medication: this.medicationIdentity() })
      })

      if (!response.ok) throw new Error("request_failed")

      const suggestion = await response.json()
      if (!this.applySuggestion(suggestion)) {
        this.showStatus("No trusted source guidance was found for this medication.")
      }
    } catch (_error) {
      this.showStatus("Medication help is temporarily unavailable.")
    } finally {
      this.setBusy(false)
    }
  }

  applySuggestion(suggestion) {
    const medication = suggestion.medication || {}
    const doses = suggestion.doses || []
    const sources = suggestion.sources || []

    if (Object.keys(medication).length === 0 && doses.length === 0) return false

    this.applyMedicationFields(medication)
    this.applyPrimaryDose(doses[0])
    this.showSources(sources)
    this.markApplied()
    this.showStatus("Suggested fields were added. Check them before saving.")
    return true
  }

  applyMedicationFields(medication) {
    this.setFieldValue("medication_name", medication.name, { onlyIfBlank: true })
    this.setFieldValue("medication_description", medication.description, { onlyIfBlank: true })
    this.setFieldValue("medication_warnings", medication.warnings, { onlyIfBlank: true })
  }

  applyPrimaryDose(dose) {
    if (!dose) return

    this.setFieldValue("wizard_dose_amount", dose.amount)
    this.setFieldValue("wizard_dose_unit", dose.unit)

    const event = new Event("input", { bubbles: true })
    this.element.querySelector("#wizard_dose_amount")?.dispatchEvent(event)
    this.element.querySelector("#wizard_dose_unit")?.dispatchEvent(new Event("change", { bubbles: true }))
  }

  showSources(sources) {
    if (!this.hasSourceListTarget || sources.length === 0) return

    this.sourceListTarget.replaceChildren()
    sources.forEach((source) => {
      const link = document.createElement("a")
      link.href = source.url
      link.target = "_blank"
      link.rel = "noopener noreferrer"
      link.className = "block rounded-2xl border border-outline-variant/60 bg-surface px-3 py-2 text-sm font-semibold text-primary"
      link.textContent = source.title || source.url
      this.sourceListTarget.append(link)
    })
  }

  markApplied() {
    if (this.hasAppliedFieldTarget) this.appliedFieldTarget.value = "1"
    if (this.hasConfirmationPanelTarget) this.confirmationPanelTarget.classList.remove("hidden")
    if (this.hasConfirmationInputTarget) this.confirmationInputTarget.required = true
  }

  setFieldValue(id, value, options = {}) {
    if (value === undefined || value === null || value === "") return

    const field = this.element.querySelector(`#${id}`)
    if (!field) return
    if (options.onlyIfBlank && field.value) return

    field.value = value
    field.dispatchEvent(new Event("input", { bubbles: true }))
    field.dispatchEvent(new Event("change", { bubbles: true }))
  }

  medicationIdentity() {
    return {
      name: this.valueOf("medication_name"),
      barcode: this.valueOfName("medication[barcode]"),
      dmd_code: this.valueOfName("medication[dmd_code]"),
      dmd_system: this.valueOfName("medication[dmd_system]"),
      dmd_concept_class: this.valueOfName("medication[dmd_concept_class]"),
      description: this.valueOf("medication_description")
    }
  }

  valueOf(id) {
    return this.element.querySelector(`#${id}`)?.value || ""
  }

  valueOfName(name) {
    return this.element.querySelector(`[name="${name}"]`)?.value || ""
  }

  showStatus(message) {
    if (!this.hasStatusTarget) return

    this.statusTarget.textContent = message
    this.statusTarget.classList.remove("hidden")
  }

  setBusy(busy) {
    if (!this.hasButtonTarget) return

    this.buttonTarget.disabled = busy
  }

  csrfToken() {
    return document.querySelector("meta[name='csrf-token']")?.content || ""
  }
}
