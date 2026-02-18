import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static values = {
    current: String
  }

  connect() {
    const savedTheme = localStorage.getItem("med-tracker-theme")
    if (savedTheme) {
      this.applyTheme(savedTheme)
    }
  }

  switch(event) {
    const theme = event.currentTarget.dataset.theme
    this.applyTheme(theme)
    localStorage.setItem("med-tracker-theme", theme)
  }

  applyTheme(theme) {
    const root = document.documentElement
    
    // Remove all theme classes
    const classes = Array.from(root.classList)
    classes.forEach(c => {
      if (c.startsWith("theme-")) {
        root.classList.remove(c)
      }
    })

    // Add new theme class
    if (theme && theme !== "default") {
      root.classList.add(`theme-${theme}`)
    }
    
    // Mark active in UI - look for the circle div inside the button
    this.element.querySelectorAll("[data-theme]").forEach(btn => {
      const circle = btn.querySelector("div")
      if (btn.dataset.theme === theme) {
        circle.classList.add("ring-2", "ring-offset-2", "ring-primary")
        circle.classList.remove("border-2") // Remove border when ring is active for cleaner look
      } else {
        circle.classList.remove("ring-2", "ring-offset-2", "ring-primary")
        circle.classList.add("border-2")
      }
    })
  }
}
