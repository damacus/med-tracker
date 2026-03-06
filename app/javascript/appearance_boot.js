(() => {
  const root = document.documentElement
  const appearance = localStorage.getItem("med-tracker-appearance") || "system"
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches
  const allowPalette = root.dataset.allowPalette === "true"
  const themeColorMeta = document.querySelector('meta[name="theme-color"]')
  const palette = localStorage.getItem("med-tracker-theme")

  root.dataset.appearance = appearance
  root.classList.toggle("dark", appearance === "dark" || (appearance === "system" && prefersDark))

  Array.from(root.classList).forEach((className) => {
    if (className.startsWith("theme-")) {
      root.classList.remove(className)
    }
  })

  if (allowPalette && palette && palette !== "default") {
    root.classList.add(`theme-${palette}`)
  }

  if (themeColorMeta) {
    themeColorMeta.setAttribute("content", root.classList.contains("dark") ? "#111827" : "#f8fafc")
  }
})()
