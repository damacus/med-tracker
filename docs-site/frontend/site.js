import "@awesome.me/webawesome/dist/styles/webawesome.css"
import "willamette/frontend/styles/theme.css"
import "./site.css"
import Willamette from "willamette"

Willamette.init()

const modeButton = document.querySelector("#colour-mode")
const prefersDark = window.matchMedia("(prefers-color-scheme: dark)")
const modes = ["system", "light", "dark"]
let mode = localStorage.getItem("medtracker-colour-mode")
if (!modes.includes(mode)) mode = "system"

function applyColourMode() {
  const dark = mode === "dark" || (mode === "system" && prefersDark.matches)
  document.documentElement.classList.toggle("wa-dark", dark)
  if (modeButton) {
    modeButton.textContent = `${mode} theme`
    modeButton.setAttribute("aria-label", `Colour theme: ${mode}. Change colour theme.`)
  }
}

applyColourMode()
prefersDark.addEventListener("change", applyColourMode)
modeButton?.addEventListener("click", () => {
  mode = modes[(modes.indexOf(mode) + 1) % modes.length]
  localStorage.setItem("medtracker-colour-mode", mode)
  applyColourMode()
})

const diagrams = document.querySelectorAll("pre.mermaid")
if (diagrams.length) {
  const { default: mermaid } = await import("mermaid")
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: document.documentElement.classList.contains("wa-dark") ? "dark" : "default",
  })
  // Mermaid adds measurement SVGs directly to the body. Willamette's shadow
  // layout needs a default slot for those temporary, unnamed children.
  const measurementArea = document.createElement("div")
  measurementArea.style.cssText = "position:absolute; width:0; height:0; overflow:hidden"
  measurementArea.setAttribute("aria-hidden", "true")
  measurementArea.append(document.createElement("slot"))
  document.body.shadowRoot?.append(measurementArea)
  try {
    await mermaid.run({ nodes: diagrams })
  } finally {
    measurementArea.remove()
  }
}
