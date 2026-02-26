import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["input", "results", "idle", "submitButton"]

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

      if (data.error === "not_configured") {
        this.showNotConfigured()
      } else if (data.error) {
        this.showError(data.error)
      } else {
        this.showResults(query, data.results)
      }
    } catch (error) {
      this.showError("Unable to connect to the medication database. Please try again.")
    }
  }

  get searchUrl() {
    return "/medication-finder/search"
  }

  showLoading() {
    this.resultsTarget.innerHTML = `
      <div class="text-center py-12 text-slate-500" data-medication-search-target="loading">
        <div class="inline-flex items-center gap-2">
          <svg class="animate-spin h-4 w-4 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <span class="text-sm">Searching NHS dm+d database...</span>
        </div>
      </div>
    `
  }

  showIdle() {
    this.resultsTarget.innerHTML = `
      <div class="text-center py-12 text-slate-500">
        <p class="text-sm">Enter a medication name above to search the NHS dm+d database.</p>
      </div>
    `
  }

  showNotConfigured() {
    this.resultsTarget.innerHTML = `
      <div class="rounded-lg border border-amber-200 bg-amber-50 p-4" role="alert">
        <p class="text-sm font-medium text-amber-800">Medication search not available</p>
        <p class="text-sm text-amber-700 mt-1">NHS dm+d credentials are not configured. Ask your administrator to set <code>NHS_DMD_CLIENT_ID</code> and <code>NHS_DMD_CLIENT_SECRET</code>.</p>
      </div>
    `
  }

  showError(message) {
    this.resultsTarget.innerHTML = `
      <div class="rounded-lg border border-destructive/50 bg-destructive/10 p-4" role="alert">
        <p class="text-sm font-medium text-destructive">Search unavailable</p>
        <p class="text-sm text-destructive/80 mt-1">${this.escapeHtml(message)}</p>
      </div>
    `
  }

  showResults(query, results) {
    if (results.length === 0) {
      this.resultsTarget.innerHTML = `
        <div class="text-center py-12 text-slate-500" data-testid="no-results">
          <p class="text-sm font-medium">No medications found</p>
          <p class="text-sm mt-1">Try searching with different terms or check the spelling.</p>
        </div>
      `
      return
    }

    const header = `
      <div class="flex items-center justify-between mb-3">
        <p class="text-sm font-medium text-slate-700" data-testid="search-results-header">
          Search Results
          <span class="text-slate-500 font-normal">— ${results.length} result${results.length === 1 ? '' : 's'} for &ldquo;${this.escapeHtml(query)}&rdquo;</span>
        </p>
        <p class="text-xs text-slate-400">Source: NHS dm+d</p>
      </div>
    `

    const items = results.map(result => this.renderResultCard(result)).join('')

    this.resultsTarget.innerHTML = header + `<div class="space-y-2" data-testid="results-list">${items}</div>`
  }

  renderResultCard(result) {
    const badge = result.concept_class
      ? `<span class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-blue-100 text-blue-700">${this.escapeHtml(result.concept_class)}</span>`
      : ''

    const label = result.concept_class_label && result.concept_class_label !== result.concept_class
      ? `<span class="text-xs text-slate-400">${this.escapeHtml(result.concept_class_label)}</span>`
      : ''

    return `
      <div class="rounded-lg border border-slate-200 bg-white p-4 hover:border-slate-300 hover:shadow-sm transition-all" data-testid="result-card">
        <div class="flex items-start justify-between gap-3">
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-slate-900 truncate">${this.escapeHtml(result.display)}</p>
            <p class="text-xs text-slate-500 mt-0.5">dm+d code: ${this.escapeHtml(result.code)}</p>
          </div>
          <div class="flex flex-col items-end gap-1 shrink-0">
            ${badge}
            ${label}
          </div>
        </div>
      </div>
    `
  }

  escapeHtml(str) {
    const div = document.createElement('div')
    div.appendChild(document.createTextNode(String(str || '')))
    return div.innerHTML
  }
}
