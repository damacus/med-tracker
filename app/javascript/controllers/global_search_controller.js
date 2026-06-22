import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["panel", "input", "results", "status", "result", "trigger"]

  connect() {
    this.shortcutHandler = this.shortcut.bind(this)
    this.pointerDownHandler = this.closeFromOutside.bind(this)
    window.addEventListener("keydown", this.shortcutHandler)
    document.addEventListener("pointerdown", this.pointerDownHandler)
    this.activeIndex = -1
    this.isOpen = false
    this.panelAnimation = null
    this.translations = JSON.parse(this.panelTarget.dataset.translations || "{}")
    this.element.dataset.globalSearchConnected = "true"
  }

  disconnect() {
    window.removeEventListener("keydown", this.shortcutHandler)
    document.removeEventListener("pointerdown", this.pointerDownHandler)
    this.abortCurrentSearch()
    this.cancelPanelAnimation()
    clearTimeout(this.searchTimer)
    delete this.element.dataset.globalSearchConnected
  }

  shortcut(event) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
      event.preventDefault()
      this.open(event)
    }
  }

  open(event) {
    if (event) event.preventDefault()
    const opener = event?.currentTarget instanceof Element ? event.currentTarget : this.visibleTrigger
    this.previouslyFocused = event?.currentTarget instanceof Element ? opener : document.activeElement
    this.isOpen = true
    this.positionPanel(opener)
    this.panelTarget.hidden = false
    this.panelTarget.dataset.open = "true"
    this.panelTarget.setAttribute("aria-hidden", "false")
    this.updateTriggerState(true)

    this.inputTarget.value = ""
    this.resultsTarget.innerHTML = ""
    this.renderResults([])
    this.animatePanelOpen()
    requestAnimationFrame(() => this.inputTarget.focus({ preventScroll: true }))
  }

  close(event) {
    if (event) event.preventDefault()
    if (!this.isOpen) return

    this.isOpen = false
    this.abortCurrentSearch()
    clearTimeout(this.searchTimer)
    this.updateTriggerState(false)
    this.animatePanelClosed()
    this.previouslyFocused?.focus?.({ preventScroll: true })
  }

  cancel(event) {
    event.preventDefault()
    this.close()
  }

  search() {
    clearTimeout(this.searchTimer)
    this.searchTimer = setTimeout(() => {
      this.fetchResults(this.inputTarget.value.trim())
    }, 150)
  }

  submit(event) {
    event.preventDefault()
    const result = this.activeResult || this.resultTargets[0]
    if (result) window.location.href = result.href
  }

  handleKeydown(event) {
    if (event.key === "ArrowDown") {
      event.preventDefault()
      this.activate(this.activeIndex + 1)
    } else if (event.key === "ArrowUp") {
      event.preventDefault()
      this.activate(this.activeIndex - 1)
    } else if (event.key === "Enter") {
      this.submit(event)
    } else if (event.key === "Escape") {
      this.close(event)
    }
  }

  activatePointer(event) {
    const index = Number.parseInt(event.currentTarget.dataset.index, 10)
    if (!Number.isNaN(index)) this.activate(index)
  }

  async fetchResults(query) {
    this.abortCurrentSearch()
    this.showLoading()

    this.abortController = new AbortController()
    const url = new URL(this.panelTarget.dataset.searchUrl, window.location.origin)
    url.searchParams.set("q", query)

    try {
      const response = await fetch(url.toString(), {
        headers: { "Accept": "application/json" },
        signal: this.abortController.signal
      })
      const data = await response.json()
      this.renderResults(data.results || [])
    } catch (error) {
      if (error.name !== "AbortError") this.renderResults([])
    }
  }

  abortCurrentSearch() {
    if (this.abortController) {
      this.abortController.abort()
      this.abortController = null
    }
  }

  showLoading() {
    this.activeIndex = -1
    this.resultsTarget.innerHTML = `
      <div class="px-3 py-8 text-center text-sm text-on-surface-variant">
        ${this.escapeHtml(this.t("loading"))}
      </div>
    `
    this.setStatus(this.t("loading"))
  }

  renderResults(results) {
    this.activeIndex = -1

    if (results.length === 0) {
      this.resultsTarget.innerHTML = `
        <div class="px-3 py-8 text-center text-sm text-on-surface-variant">
          ${this.escapeHtml(this.t("no_results"))}
        </div>
      `
      this.setStatus(this.t("no_results"))
      return
    }

    this.resultsTarget.innerHTML = this.groupedResultsHtml(results)
    this.animateResultRows()
    this.setStatus(this.resultCountText(results.length))
  }

  groupedResultsHtml(results) {
    const groups = []

    results.forEach((result, index) => {
      const group = groups.find((entry) => entry.type === result.type)
      const item = this.resultHtml(result, index)

      if (group) {
        group.items.push(item)
      } else {
        groups.push({ type: result.type, items: [item] })
      }
    })

    return groups.map((group) => `
      <section class="space-y-1 pb-3">
        <h2 class="px-2 py-1 text-[10px] font-black uppercase tracking-widest text-on-surface-variant">
          ${this.escapeHtml(this.typeLabel(group.type))}
        </h2>
        <div class="space-y-1">${group.items.join("")}</div>
      </section>
    `).join("")
  }

  resultHtml(result, index) {
    return `
      <a
        href="${this.hrefAttribute(result.path)}"
        class="block rounded-md border border-transparent px-3 py-3 no-underline outline-none transition-colors
               hover:border-primary hover:bg-surface-container-low
               focus-visible:border-primary focus-visible:bg-surface-container-low focus-visible:ring-2 focus-visible:ring-primary
               data-[global-search-active=true]:border-primary data-[global-search-active=true]:bg-surface-container-low"
        data-global-search-target="result"
        data-global-search-active="false"
        data-index="${index}"
        data-action="mouseenter->global-search#activatePointer"
      >
        <span class="block text-sm font-bold text-foreground">${this.escapeHtml(result.title)}</span>
        <span class="block text-xs text-on-surface-variant">${this.escapeHtml(result.subtitle || "")}</span>
      </a>
    `
  }

  activate(index) {
    if (this.resultTargets.length === 0) return

    this.activeIndex = (index + this.resultTargets.length) % this.resultTargets.length

    this.resultTargets.forEach((result, resultIndex) => {
      const active = resultIndex === this.activeIndex
      result.dataset.globalSearchActive = active ? "true" : "false"
      if (active) result.scrollIntoView({ block: "nearest" })
    })
  }

  get activeResult() {
    return this.resultTargets[this.activeIndex]
  }

  setStatus(text) {
    this.statusTarget.textContent = text
  }

  closeFromOutside(event) {
    if (!this.isOpen) return
    if (this.panelTarget.contains(event.target)) return
    if (this.triggerTargets.some((trigger) => trigger.contains(event.target))) return

    this.close()
  }

  animatePanelOpen() {
    this.cancelPanelAnimation()

    if (this.prefersReducedMotion) return

    const animation = this.panelTarget.animate(
      [
        { opacity: 0, transform: "translateY(-10px) scaleY(0.96)" },
        { opacity: 1, transform: "translateY(0) scaleY(1.01)", offset: 0.72 },
        { opacity: 1, transform: "translateY(0) scaleY(1)" }
      ],
      { duration: 240, easing: "cubic-bezier(0.22, 1, 0.36, 1)" }
    )
    this.panelAnimation = animation
    animation.finished.then(() => {
      if (this.panelAnimation === animation) this.panelAnimation = null
    }).catch(() => {})
  }

  animatePanelClosed() {
    const hide = () => {
      this.panelTarget.hidden = true
      this.panelTarget.dataset.open = "false"
      this.panelTarget.setAttribute("aria-hidden", "true")
    }

    this.cancelPanelAnimation()

    if (this.prefersReducedMotion) {
      hide()
      return
    }

    const animation = this.panelTarget.animate(
      [
        { opacity: 1, transform: "translateY(0) scaleY(1)" },
        { opacity: 0, transform: "translateY(-6px) scaleY(0.98)" }
      ],
      { duration: 120, easing: "ease-out" }
    )
    this.panelAnimation = animation
    animation.finished.then(() => {
      if (this.panelAnimation !== animation) return

      this.panelAnimation = null
      if (!this.isOpen) hide()
    }).catch(() => {})
  }

  animateResultRows() {
    if (this.prefersReducedMotion) return

    this.resultTargets.forEach((result, index) => {
      result.animate(
        [
          { opacity: 0, transform: "translateY(-6px)" },
          { opacity: 1, transform: "translateY(0)" }
        ],
        { duration: 190, delay: index * 22, easing: "cubic-bezier(0.22, 1, 0.36, 1)" }
      )
    })
  }

  cancelPanelAnimation() {
    if (!this.panelAnimation) return

    this.panelAnimation.cancel()
    this.panelAnimation = null
  }

  updateTriggerState(expanded) {
    this.triggerTargets.forEach((trigger) => {
      trigger.setAttribute("aria-expanded", expanded ? "true" : "false")
    })
  }

  positionPanel(trigger) {
    const margin = 16

    if (!trigger || window.innerWidth < 640) {
      this.panelTarget.style.left = `${margin}px`
      this.panelTarget.style.top = "62px"
      this.panelTarget.style.width = `${window.innerWidth - margin * 2}px`
      return
    }

    const rect = trigger.getBoundingClientRect()
    const left = Math.max(margin, rect.left)
    const availableWidth = window.innerWidth - left - margin
    const width = Math.min(Math.max(rect.width, 320), availableWidth)

    this.panelTarget.style.left = `${left}px`
    this.panelTarget.style.top = `${rect.bottom + 12}px`
    this.panelTarget.style.width = `${width}px`
  }

  get visibleTrigger() {
    return this.triggerTargets.find((trigger) => trigger.offsetParent !== null) || this.triggerTargets[0]
  }

  get prefersReducedMotion() {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches
  }

  typeLabel(type) {
    return this.translations.type_labels?.[type] || type
  }

  resultCountText(count) {
    const template = count === 1 ? this.t("result_one") : this.t("result_other")
    return template.replace("%{count}", count)
  }

  t(key) {
    return this.translations[key] || ""
  }

  escapeHtml(value) {
    const div = document.createElement("div")
    div.appendChild(document.createTextNode(String(value || "")))
    return div.innerHTML
  }

  hrefAttribute(url) {
    const stringUrl = String(url || "").trim()
    if (!stringUrl) return ""

    try {
      const parsedUrl = new URL(stringUrl, window.location.origin)
      const protocol = parsedUrl.protocol.toLowerCase()

      if (!["http:", "https:", "mailto:", "tel:"].includes(protocol)) {
        return "#"
      }

      const link = document.createElement("a")
      link.setAttribute("href", stringUrl)
      return link.getAttribute("href") || ""
    } catch (error) {
      return "#"
    }
  }
}
