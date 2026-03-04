import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["singleDoseRadio", "singleDoseContainer", "multiDoseContainer", "presetChip", "otherInput", "hiddenOutput"]

  connect() {
    this.updateHidden()
  }

  toggle() {
    if (!this.hasSingleDoseRadioTarget) return

    const isSingle = this.singleDoseRadioTarget.checked

    if (this.hasSingleDoseContainerTarget) {
      this.singleDoseContainerTarget.classList.toggle("hidden", !isSingle)
    }

    if (this.hasMultiDoseContainerTarget) {
      this.multiDoseContainerTarget.classList.toggle("hidden", isSingle)
    }
  }

  updateHidden() {
    if (!this.hasSingleDoseRadioTarget || !this.hasHiddenOutputTarget) return

    const isSingle = this.singleDoseRadioTarget.checked
    if (isSingle) {
      this.hiddenOutputTarget.value = ""
      return
    }

    const selectedPresets = this.hasPresetChipTarget
      ? this.presetChipTargets.filter(chip => chip.checked).map(chip => chip.value)
      : []

    const otherValues = this.hasOtherInputTarget && this.otherInputTarget.value
      ? this.otherInputTarget.value.split(',').map(v => v.trim()).filter(v => v !== "" && !isNaN(v))
      : []

    const allValues = [...new Set([...selectedPresets, ...otherValues])]
      .map(v => parseFloat(v))
      .sort((a, b) => a - b)

    this.hiddenOutputTarget.value = allValues.join(',')
  }
}
