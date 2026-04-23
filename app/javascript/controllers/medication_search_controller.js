import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["input", "results", "idle", "submitButton"]
  static values = { translations: Object }

  barcodeDecoded(event) {
    const barcode = event.detail.barcode
    if (!barcode) return

    this.inputTarget.value = barcode
    this.search(event)
  }

  async search(event) {
    event.preventDefault()

    const query = this.inputTarget.value.trim()

    if (!query) {
      this.showIdle()
      return
    }

    this.showLoading()

    try {
      const url = new URL(this.searchUrl, window.location.origin)
      url.searchParams.set("q", query)

      const response = await fetch(url.toString(), {
        headers: { "Accept": "application/json" }
      })

      const data = await response.json()

      if (data.error) {
        this.showError(data.error)
      } else {
        const barcode = data.barcode || this.barcodeQuery(query)
        const displayQuery = barcode || query
        this.showResults(displayQuery, data.results, barcode)
      }
    } catch (error) {
      this.showError(this.t("unavailableMessage"))
    }
  }

  get searchUrl() {
    return "/medication-finder/search"
  }

  showLoading() {
    this.resultsTarget.innerHTML = `
      <div class="text-center py-12 text-on-surface-variant" data-medication-search-target="loading">
        <div class="inline-flex items-center gap-2">
          <svg class="animate-spin h-4 w-4 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <span class="text-sm">${this.escapeHtml(this.t("loading"))}</span>
        </div>
      </div>
    `
  }

  showIdle() {
    this.resultsTarget.innerHTML = `
      <div class="text-center py-12 text-on-surface-variant">
        <p class="text-sm">${this.escapeHtml(this.t("idleText"))}</p>
      </div>
    `
  }

  showError(message) {
    this.resultsTarget.innerHTML = `
      <div class="rounded-lg border border-destructive/50 bg-destructive/10 p-4" role="alert">
        <p class="text-sm font-medium text-destructive">${this.escapeHtml(this.t("unavailableTitle"))}</p>
        <p class="text-sm text-destructive/80 mt-1">${this.escapeHtml(message)}</p>
      </div>
    `
  }

  showResults(query, results, barcode) {
    if (results.length === 0) {
      this.resultsTarget.innerHTML = `
        <div class="text-center py-12 text-on-surface-variant" data-testid="no-results">
          <p class="text-sm font-medium">${this.escapeHtml(this.t("noResultsTitle"))}</p>
          <p class="text-sm mt-1">${this.escapeHtml(this.t("noResultsMessage"))}</p>
        </div>
      `
      return
    }

    const sources = [...new Set(results.map((result) => result.source_label).filter(Boolean))]
    const sourceText = sources.length > 0
      ? `${this.escapeHtml(this.t("source"))}: ${this.escapeHtml(sources.join(", "))}`
      : this.escapeHtml(this.t("source"))

    const header = `
      <div class="flex items-center justify-between mb-3">
        <p class="text-sm font-medium text-foreground" data-testid="search-results-header">
          ${this.escapeHtml(this.t("resultsTitle"))}
          <span class="text-on-surface-variant font-normal">— ${this.escapeHtml(this.resultCountText(results.length, query))}</span>
        </p>
        <p class="text-xs text-on-surface-variant">${sourceText}</p>
      </div>
    `

    const items = results.map(result => this.renderResultCard(result, barcode)).join('')

    this.resultsTarget.innerHTML = header + `<div class="space-y-2" data-testid="results-list">${items}</div>`
  }

  renderResultCard(result, barcode) {
    const matchReasonBadge = result.match_reason_label
      ? `<span class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-tertiary-container text-on-tertiary-container">${this.escapeHtml(result.match_reason_label)}</span>`
      : ''

    const sourceBadge = result.source_label
      ? `<span class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-secondary-container text-on-secondary-container">${this.escapeHtml(result.source_label)}</span>`
      : ''

    const badge = result.concept_class
      ? `<span class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-primary-container text-on-primary-container">${this.escapeHtml(result.concept_class)}</span>`
      : ''

    const label = result.concept_class_label && result.concept_class_label !== result.concept_class
      ? `<span class="text-xs text-on-surface-variant">${this.escapeHtml(result.concept_class_label)}</span>`
      : ''

    const addAction = (result.name || result.display)
      ? `
        <div class="mt-4 flex justify-end">
          <a
            href="${this.hrefAttribute(this.addMedicationUrl(result, barcode || result.barcode))}"
            class="inline-flex items-center rounded-full bg-primary px-4 py-2 text-sm font-medium text-on-primary shadow-sm transition-all hover:shadow-md"
            data-testid="add-medication-link"
          >${this.escapeHtml(this.t("addMedication"))}</a>
        </div>
      `
      : ''

    const identifier = this.renderIdentifier(result)
    const title = result.name || result.display
    const packageSize = result.package_size
      ? `<p class="text-xs text-on-surface-variant mt-0.5">Pack size: ${this.escapeHtml(result.package_size)}</p>`
      : ''

    return `
      <div class="rounded-lg border border-border bg-surface-container-lowest p-4 hover:border-primary hover:shadow-sm transition-all" data-testid="result-card">
        <div class="flex items-start justify-between gap-3">
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-foreground truncate">${this.escapeHtml(title)}</p>
            ${packageSize}
            ${identifier}
          </div>
          <div class="flex flex-col items-end gap-1 shrink-0">
            ${matchReasonBadge}
            ${sourceBadge}
            ${badge}
            ${label}
          </div>
        </div>
        ${addAction}
      </div>
    `
  }

  addMedicationUrl(result, barcode) {
    const url = new URL("/medications/new", window.location.origin)
    url.searchParams.set("name", result.name || result.display || "")

    if (result.category) {
      url.searchParams.set("category", result.category)
    }

    if (barcode) {
      url.searchParams.set("barcode", barcode)
    }

    if (result.package_quantity !== null && result.package_quantity !== undefined) {
      url.searchParams.set("package_quantity", result.package_quantity)
    }

    if (result.package_unit) {
      url.searchParams.set("package_unit", result.package_unit)
    }

    if (result.code && result.system === 'https://dmd.nhs.uk') {
      url.searchParams.set("dmd_code", result.code)
      url.searchParams.set("dmd_system", result.system || "")
      url.searchParams.set("dmd_concept_class", result.concept_class || "")
    }

    return url.toString()
  }

  renderIdentifier(result) {
    if (result.code) {
      return `<p class="text-xs text-on-surface-variant mt-0.5">${this.escapeHtml(this.t("dmdCode"))}: ${this.escapeHtml(result.code)}</p>`
    }

    if (result.barcode) {
      return `<p class="text-xs text-on-surface-variant mt-0.5">${this.escapeHtml(this.t("barcode"))}: ${this.escapeHtml(result.barcode)}</p>`
    }

    return ''
  }

  barcodeQuery(query) {
    const normalized = String(query || "").trim().replace(/\D/g, "")
    return /^\d{13,14}$/.test(normalized) ? normalized : null
  }

  escapeHtml(str) {
    const div = document.createElement('div')
    div.appendChild(document.createTextNode(String(str || '')))
    return div.innerHTML
  }

  hrefAttribute(url) {
    const link = document.createElement('a')
    link.setAttribute('href', String(url || ''))
    return link.getAttribute('href') || ''
  }

  t(key) {
    return this.translationsValue?.[key] || ""
  }

  resultCountText(count, query) {
    const rules = new Intl.PluralRules(document.documentElement.lang || "en")
    const category = rules.select(count)
    const templates = this.translationsValue?.resultCount || {}
    const template = templates[category] || templates.other || templates.one || ""
    return template
      .replace("%{count}", count)
      .replace("%{query}", query)
  }
}
