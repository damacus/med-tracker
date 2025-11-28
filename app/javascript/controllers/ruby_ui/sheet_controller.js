import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["content"]

  open() {
    // Insert the sheet content into the body
    document.body.insertAdjacentHTML("beforeend", this.contentTarget.innerHTML)

    // Get the newly inserted sheet content
    const sheetContent = document.body.lastElementChild
    const sheetPanel = sheetContent.querySelector('[data-state]')

    if (sheetPanel) {
      // Start with closed state
      sheetPanel.setAttribute('data-state', 'closed')

      // Force a reflow to ensure the closed state is applied
      sheetPanel.offsetHeight

      // Trigger the animation by changing to open state
      requestAnimationFrame(() => {
        sheetPanel.setAttribute('data-state', 'open')
      })
    }
  }
}
