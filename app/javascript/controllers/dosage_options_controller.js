import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["template", "container"]

  add(event) {
    event.preventDefault()
    const newId = new Date().getTime()

    // Create a temporary container
    const temp = document.createElement('div')
    temp.innerHTML = this.templateTarget.innerHTML.replace(/NEW_RECORD/g, newId)

    // Get the option element
    const option = temp.querySelector('[data-dosage-options-target="option"]')
    if (option) {
      // Make sure it's visible
      option.style.cssText = 'display: flex !important'

      // Add it to the container
      this.containerTarget.appendChild(option)

      // Focus the input
      const input = option.querySelector('input')
      if (input) input.focus()
    }
  }

  remove(event) {
    event.preventDefault()
    const option = event.target.closest('[data-dosage-options-target="option"]')
    if (option) {
      this.toggleOption(option, true)
    }
  }

  undo(event) {
    event.preventDefault()
    const option = event.target.closest('[data-dosage-options-target="option"]')
    if (option) {
      this.toggleOption(option, false)
    }
  }

  toggleOption(option, removed) {
    const destroyField = option.querySelector('[data-dosage-options-target="destroyField"]')
    const editor = option.querySelector('[data-dosage-options-target="editor"]')
    const removedState = option.querySelector('[data-dosage-options-target="removedState"]')

    if (destroyField) destroyField.value = removed ? '1' : '0'

    this.toggleEditorControls(editor, removed)

    if (editor) editor.classList.toggle('hidden', removed)
    if (removedState) {
      removedState.classList.toggle('hidden', !removed)
      removedState.classList.toggle('flex', removed)
    }
  }

  toggleEditorControls(editor, disabled) {
    if (!editor) return

    editor.querySelectorAll('input, select, textarea, button').forEach((element) => {
      if (element.dataset.action?.includes('dosage-options#remove')) return
      element.disabled = disabled
    })
  }
}
