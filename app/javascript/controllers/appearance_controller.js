import { Controller } from "@hotwired/stimulus"

const APPEARANCE_STORAGE_KEY = "med-tracker-appearance"
const THEME_STORAGE_KEY = "med-tracker-theme"
const DARK_THEME_COLOR = "#111827"
const LIGHT_THEME_COLOR = "#f8fafc"

export default class extends Controller {
  connect() {
    this.mediaQuery = window.matchMedia("(prefers-color-scheme: dark)")
    this.handleSystemChange = this.handleSystemChange.bind(this)
    if (this.mediaQuery.addEventListener) {
      this.mediaQuery.addEventListener("change", this.handleSystemChange)
    } else {
      this.mediaQuery.addListener(this.handleSystemChange)
    }
    this.sync()
  }

  disconnect() {
    if (!this.mediaQuery) {
      return
    }

    if (this.mediaQuery.removeEventListener) {
      this.mediaQuery.removeEventListener("change", this.handleSystemChange)
    } else {
      this.mediaQuery.removeListener(this.handleSystemChange)
    }
  }

  switchAppearance(event) {
    const appearance = event.currentTarget.dataset.appearance || "system"
    localStorage.setItem(APPEARANCE_STORAGE_KEY, appearance)
    this.applyAppearance(appearance)
  }

  switchTheme(event) {
    const theme = event.currentTarget.dataset.theme || "default"
    localStorage.setItem(THEME_STORAGE_KEY, theme)
    this.applyTheme(theme)
  }

  handleSystemChange() {
    if ((localStorage.getItem(APPEARANCE_STORAGE_KEY) || "system") === "system") {
      this.applyAppearance("system")
    }
  }

  sync() {
    this.applyAppearance(localStorage.getItem(APPEARANCE_STORAGE_KEY) || "system")
    this.applyTheme(localStorage.getItem(THEME_STORAGE_KEY) || "default")
  }

  applyAppearance(appearance) {
    const root = document.documentElement
    const isDark = appearance === "dark" || (appearance === "system" && this.mediaQuery.matches)

    root.dataset.appearance = appearance
    root.classList.toggle("dark", isDark)

    this.element.querySelectorAll("[data-appearance]").forEach((button) => {
      const isActive = button.dataset.appearance === appearance
      button.setAttribute("aria-pressed", isActive ? "true" : "false")
      button.dataset.active = isActive ? "true" : "false"
    })

    const metaThemeColor = document.querySelector('meta[name="theme-color"]')
    if (metaThemeColor) {
      metaThemeColor.setAttribute("content", isDark ? DARK_THEME_COLOR : LIGHT_THEME_COLOR)
    }
  }

  applyTheme(theme) {
    const root = document.documentElement
    const activeTheme = theme || "default"

    Array.from(root.classList).forEach((className) => {
      if (className.startsWith("theme-")) {
        root.classList.remove(className)
      }
    })

    if (activeTheme !== "default") {
      root.classList.add(`theme-${activeTheme}`)
    }

    this.element.querySelectorAll("[data-theme]").forEach((button) => {
      const swatch = button.querySelector("[data-theme-swatch]")
      const isActive = button.dataset.theme === activeTheme

      button.setAttribute("aria-pressed", isActive ? "true" : "false")
      button.dataset.active = isActive ? "true" : "false"

      if (swatch) {
        swatch.dataset.active = isActive ? "true" : "false"
      }
    })
  }
}
