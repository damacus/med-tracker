import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["dialog", "input", "results", "status", "result"]

  connect() {
    this.shortcutHandler = this.shortcut.bind(this)
    window.addEventListener("keydown", this.shortcutHandler)
    this.activeIndex = -1
    this.translations = JSON.parse(this.dialogTarget.dataset.translations || "{}")
  }

  disconnect() {
    window.removeEventListener("keydown", this.shortcutHandler)
    this.abortCurrentSearch()
    clearTimeout(this.searchTimer)
  }

  shortcut(event) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
      event.preventDefault()
      this.open(event)
    }
  }

  open(event) {
    const opener = event?.currentTarget
    this.previouslyFocused = opener instanceof Element ? opener : document.activeElement

    if (!this.dialogTarget.open) {
      this.dialogTarget.showModal()
    }

    this.inputTarget.value = ""
    this.resultsTarget.innerHTML = ""
    this.setStatus("")
    this.fetchResults("")
    requestAnimationFrame(() => this.inputTarget.focus())
  }

  close(event) {
    if (event) event.preventDefault()
    if (this.dialogTarget.open) this.dialogTarget.close()
  }

  cancel(event) {
    event.preventDefault()
    this.close()
  }

  closed() {
    this.abortCurrentSearch()
    clearTimeout(this.searchTimer)
    this.previouslyFocused?.focus?.()
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
    const url = new URL(this.dialogTarget.dataset.searchUrl, window.location.origin)
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
    const link = document.createElement("a")
    link.setAttribute("href", String(url || ""))
    return link.getAttribute("href") || ""
  }
}
