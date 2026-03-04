import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["singleDoseRadio", "singleDoseContainer", "multiDoseContainer", "presetChip", "otherInput", "hiddenOutput"]

  connect() {
    this.updateHidden()
  }

  toggle() {
    const isSingle = this.singleDoseRadioTarget.checked
    this.singleDoseContainerTarget.classList.toggle("hidden", !isSingle)
    this.multiDoseContainerTarget.classList.toggle("hidden", isSingle)
  }

  updateHidden() {
    const isSingle = this.singleDoseRadioTarget.checked
    if (isSingle) {
      this.hiddenOutputTarget.value = ""
      return
    }

    const selectedPresets = this.presetChipTargets
      .filter(chip => chip.checked)
      .map(chip => chip.value)

    const otherValues = this.otherInputTarget.value
      .split(',')
      .map(v => v.trim())
      .filter(v => v !== "" && !isNaN(v))

    const allValues = [...new Set([...selectedPresets, ...otherValues])]
      .map(v => parseFloat(v))
      .sort((a, b) => a - b)

    this.hiddenOutputTarget.value = allValues.join(',')
  }
}
