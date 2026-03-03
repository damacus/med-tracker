import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["button", "label", "input", "panel", "search", "option", "empty"]

  connect() {
    this.applySelectedState()
  }

  toggle(event) {
    event.preventDefault()
    if (this.panelTarget.classList.contains("hidden")) {
      this.open()
    } else {
      this.close()
    }
  }

  open() {
    this.panelTarget.classList.remove("hidden")
    this.buttonTarget.setAttribute("aria-expanded", "true")
    this.searchTarget.value = ""
    this.filter()
    this.searchTarget.focus()
  }

  close() {
    this.panelTarget.classList.add("hidden")
    this.buttonTarget.setAttribute("aria-expanded", "false")
  }

  handleOutsideClick(event) {
    if (this.panelTarget.classList.contains("hidden")) return
    if (this.element.contains(event.target)) return

    this.close()
  }

  filter() {
    const query = this.searchTarget.value.trim().toLowerCase()
    let visibleCount = 0

    this.optionTargets.forEach((option) => {
      const text = option.textContent.trim().toLowerCase()
      const visible = query.length === 0 || text.includes(query)
      option.classList.toggle("hidden", !visible)
      if (visible) visibleCount += 1
    })

    this.emptyTarget.classList.toggle("hidden", visibleCount > 0)
  }

  select(event) {
    const value = event.currentTarget.dataset.value || ""
    this.inputTarget.value = value
    const placeholder = this.buttonTarget.dataset.placeholder || "Select a category"
    this.labelTarget.textContent = value || placeholder
    this.applySelectedState()
    this.close()
    this.buttonTarget.focus()
  }

  applySelectedState() {
    const selected = this.inputTarget.value || ""
    this.optionTargets.forEach((option) => {
      const isSelected = (option.dataset.value || "") === selected
      option.setAttribute("aria-selected", isSelected ? "true" : "false")
      option.classList.toggle("bg-accent", isSelected)
      option.classList.toggle("text-accent-foreground", isSelected)
    })
  }
}
