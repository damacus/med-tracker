import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["input", "formFilter", "strengthFilter", "results", "idle", "submitButton"]
  static values = { translations: Object, searchUrl: String, newMedicationUrl: String }

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
      if (this.selectedForm) url.searchParams.set("form", this.selectedForm)
      if (this.selectedStrength) url.searchParams.set("strength", this.selectedStrength)

      const response = await fetch(url.toString(), {
        headers: { "Accept": "application/json" }
      })

      const data = await response.json()

      if (data.error) {
        this.showError(data.error)
      } else {
        const barcode = data.barcode || this.barcodeQuery(query)
        const displayQuery = barcode || query
        this.showResults(displayQuery, data.results, barcode, data.permissions || {})
      }
    } catch (error) {
      this.showError(this.t("unavailableMessage"))
    }
  }

  get searchUrl() {
    return this.hasSearchUrlValue ? this.searchUrlValue : "/medication-finder/search"
  }

  get newMedicationUrl() {
    return this.hasNewMedicationUrlValue ? this.newMedicationUrlValue : "/medications/new"
  }

  get selectedForm() {
    return this.hasFormFilterTarget ? this.formFilterTarget.value : ""
  }

  get selectedStrength() {
    return this.hasStrengthFilterTarget ? this.strengthFilterTarget.value.trim() : ""
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

  showResults(query, results, barcode, permissions = {}) {
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

    const items = results.map(result => this.renderResultCard(result, barcode, permissions)).join('')

    this.resultsTarget.innerHTML = header + `<div class="space-y-2" data-testid="results-list">${items}</div>`
  }

  renderResultCard(result, barcode, permissions = {}) {
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

    const existingMedication = result.existing_medication
    const action = this.renderResultAction(result, barcode, permissions)

    const identifier = this.renderIdentifier(result)
    const pilLink = this.renderPilLink(result)
    const spcLink = this.renderSpcLink(result)
    const medicineDetails = this.renderMedicineDetails(result)
    const reviewPrompts = this.renderReviewPrompts(result)
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
            ${pilLink}
            ${spcLink}
            ${medicineDetails}
            ${reviewPrompts}
          </div>
          <div class="flex flex-col items-end gap-1 shrink-0">
            ${matchReasonBadge}
            ${sourceBadge}
            ${badge}
            ${label}
          </div>
        </div>
        ${action}
      </div>
    `
  }

  renderMedicineDetails(result) {
    const detailRows = this.medicineDetailRows(result)
    if (detailRows.length === 0) return ''

    const detailsId = `medicine-details-${Math.random().toString(36).slice(2)}`
    const rows = detailRows.map(([label, value]) => `
      <div>
        <dt class="text-xs font-bold text-on-surface-variant">${this.escapeHtml(label)}</dt>
        <dd class="mt-1 text-xs text-foreground">${this.escapeHtml(value)}</dd>
      </div>
    `).join('')

    return `
      <div class="mt-3" data-testid="medicine-details-panel">
        <button
          type="button"
          class="text-xs font-bold text-primary underline underline-offset-2"
          aria-expanded="false"
          aria-controls="${this.escapeHtml(detailsId)}"
          data-action="medication-search#toggleMedicineDetails"
          data-details-id="${this.escapeHtml(detailsId)}"
        >${this.escapeHtml(this.t("medicineDetailsButton"))}</button>
        <dl id="${this.escapeHtml(detailsId)}" class="hidden mt-3 space-y-3 rounded-lg border border-border bg-surface-container-low p-3" data-testid="medicine-details">
          ${rows}
        </dl>
      </div>
    `
  }

  medicineDetailRows(result) {
    return [
      [this.t("detailsDescription"), result.description],
      [this.t("detailsCategory"), result.category],
      [this.t("detailsPackage"), result.package_size],
      [this.t("detailsDirections"), result.directions],
      [this.t("detailsWarnings"), result.warnings]
    ].filter(([, value]) => value)
  }

  toggleMedicineDetails(event) {
    const details = document.getElementById(event.currentTarget.dataset.detailsId)
    if (!details) return

    const expanded = details.classList.toggle("hidden") === false
    event.currentTarget.setAttribute("aria-expanded", String(expanded))
  }

  renderReviewPrompts(result) {
    const reviewPrompts = Array.isArray(result.review_prompts) ? result.review_prompts : []
    const hiddenCount = Number(result.review_prompt_filter?.hidden_count || 0)
    if (reviewPrompts.length === 0 && hiddenCount === 0) return ''

    const hiddenNotice = hiddenCount > 0 ? `
      <p class="text-xs font-medium text-on-surface-variant" data-testid="filtered-review-prompts">
        ${this.escapeHtml(this.reviewPromptFilteredText(hiddenCount))}
      </p>
    ` : ''

    if (reviewPrompts.length === 0) {
      return `<div class="mt-3 border-y border-border py-3">${hiddenNotice}</div>`
    }

    const detailsId = `review-prompt-details-${Math.random().toString(36).slice(2)}`
    const highestRiskLevel = reviewPrompts[0]?.risk_level_label || ''
    const summary = this.reviewPromptSummaryText(reviewPrompts.length, highestRiskLevel)

    const items = reviewPrompts.map((reviewPrompt) => `
      <li class="rounded-md border border-warning/40 bg-warning-container/10 p-3">
        <p class="text-xs font-bold text-on-warning-container">
          ${this.escapeHtml(this.t("reviewPromptRiskLevel"))}: ${this.escapeHtml(reviewPrompt.risk_level_label || reviewPrompt.risk_level)}
        </p>
        <p class="mt-1 text-xs text-on-warning-container">
          ${this.escapeHtml(reviewPrompt.interacting_medication_name || '')}
        </p>
        <p class="mt-2 text-xs text-on-warning-container">
          <span class="font-bold">${this.escapeHtml(this.t("reviewPromptSource"))}:</span>
          ${this.escapeHtml(reviewPrompt.source_name || '')}
        </p>
        <p class="mt-1 text-xs text-on-warning-container">
          <span class="font-bold">${this.escapeHtml(this.t("reviewPromptCheckedOn"))}:</span>
          ${this.escapeHtml(reviewPrompt.source_checked_on || '')}
        </p>
        <p class="mt-2 text-xs text-on-warning-container">
          <span class="font-bold">${this.escapeHtml(this.t("reviewPromptDescription"))}:</span>
          ${this.escapeHtml(reviewPrompt.description || '')}
        </p>
      </li>
    `).join('')

    return `
      <div class="mt-3 rounded-lg border border-warning/50 bg-warning-container/20 p-3" data-testid="medication-review-prompt">
        <p class="text-xs font-bold text-on-warning-container">${this.escapeHtml(summary)}</p>
        <div class="mt-2">${hiddenNotice}</div>
        <button
          type="button"
          class="mt-2 text-xs font-bold text-on-warning-container underline underline-offset-2"
          aria-expanded="false"
          aria-controls="${this.escapeHtml(detailsId)}"
          data-action="medication-search#toggleReviewPromptDetails"
          data-details-id="${this.escapeHtml(detailsId)}"
        >${this.escapeHtml(this.t("reviewPromptDetailsButton"))}</button>
        <div id="${this.escapeHtml(detailsId)}" class="hidden pt-3">
          <h3 class="text-sm font-bold text-on-warning-container">${this.escapeHtml(this.t("reviewPromptDetails"))}</h3>
          <ul class="mt-2 space-y-2">${items}</ul>
        </div>
      </div>
    `
  }

  toggleReviewPromptDetails(event) {
    const details = document.getElementById(event.currentTarget.dataset.detailsId)
    if (!details) return

    const expanded = details.classList.toggle("hidden") === false
    event.currentTarget.setAttribute("aria-expanded", String(expanded))
  }

  renderResultAction(result, barcode, permissions = {}) {
    const existingMedication = result.existing_medication

    if (existingMedication && permissions.can_restock) {
      return this.renderRestockAction(result, existingMedication)
    }

    if (!existingMedication && permissions.can_create && (result.name || result.display)) {
      const actionUrl = this.addMedicationUrl(result, barcode || result.barcode)

      return `
        <div class="mt-4 flex justify-end">
          <a
            href="${this.hrefAttribute(actionUrl)}"
            class="inline-flex items-center rounded-full bg-primary px-4 py-2 text-sm font-medium text-on-primary shadow-sm transition-all hover:shadow-md"
            data-testid="add-medication-link"
          >${this.escapeHtml(this.t("addMedication"))}</a>
        </div>
      `
    }

    return ''
  }

  renderRestockAction(result, medication) {
    const modalId = `restock-modal-${medication.id}-${Math.random().toString(36).slice(2)}`

    return `
      <div class="mt-4 flex justify-end">
        <button
          type="button"
          class="inline-flex items-center rounded-full bg-primary px-4 py-2 text-sm font-medium text-on-primary shadow-sm transition-all hover:shadow-md"
          data-action="medication-search#openRestockModal"
          data-modal-id="${this.escapeHtml(modalId)}"
          data-testid="update-stock-button"
        >${this.escapeHtml(this.t("updateStock"))}</button>
      </div>
      ${this.renderRestockModal(result, medication, modalId)}
    `
  }

  renderRestockModal(result, medication, modalId) {
    const quantity = this.packageQuantity(result)
    const quantityField = quantity
      ? `<input type="hidden" name="refill[quantity]" value="${this.escapeHtml(quantity)}">`
      : `
        <label class="block space-y-2 text-sm font-medium">
          <span>${this.escapeHtml(this.t("restockQuantity"))}</span>
          <input
            type="number"
            name="refill[quantity]"
            required
            min="0.01"
            step="0.01"
            class="w-full rounded-shape-sm border border-outline bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary/20 transition-all"
          >
        </label>
      `

    return `
      <div
        id="${this.escapeHtml(modalId)}"
        class="fixed inset-0 z-50 hidden items-center justify-center bg-foreground/40 p-4"
        data-testid="finder-restock-modal"
      >
        <div class="w-full max-w-md rounded-shape-xl bg-background p-6 shadow-2xl">
          <div class="space-y-2">
            <h2 class="text-lg font-bold text-foreground">${this.escapeHtml(this.t("updateStock"))}</h2>
            <p class="text-sm text-on-surface-variant">${this.escapeHtml(this.restockConfirmationText(result, medication, quantity))}</p>
            ${this.currentSupplyText(medication)}
          </div>
          <form action="${this.hrefAttribute(medication.refill_path)}" method="post" class="mt-6 space-y-4" data-turbo="false">
            <input type="hidden" name="authenticity_token" value="${this.escapeHtml(this.csrfToken)}">
            <input type="hidden" name="_method" value="patch">
            <input type="hidden" name="refill[restock_date]" value="${this.escapeHtml(this.today)}">
            ${quantityField}
            <div class="flex justify-end gap-3 pt-2">
              <button
                type="button"
                class="rounded-full px-4 py-2 text-sm font-medium text-on-surface-variant hover:bg-surface-container"
                data-action="medication-search#closeRestockModal"
                data-modal-id="${this.escapeHtml(modalId)}"
              >${this.escapeHtml(this.t("restockCancel"))}</button>
              <button
                type="submit"
                class="rounded-full bg-primary px-4 py-2 text-sm font-medium text-on-primary shadow-sm"
              >${this.escapeHtml(this.t("restockSubmit"))}</button>
            </div>
          </form>
        </div>
      </div>
    `
  }

  openRestockModal(event) {
    const modal = document.getElementById(event.currentTarget.dataset.modalId)
    if (!modal) return

    modal.classList.remove("hidden")
    modal.classList.add("flex")
  }

  closeRestockModal(event) {
    const modal = document.getElementById(event.currentTarget.dataset.modalId)
    if (!modal) return

    modal.classList.add("hidden")
    modal.classList.remove("flex")
  }

  restockConfirmationText(result, medication, quantity) {
    const key = quantity ? "confirmRestock" : "confirmRestockWithoutQuantity"

    return this.t(key)
      .replace("%{quantity}", quantity || "")
      .replace("%{medication}", medication.name || result.name || result.display || "")
  }

  currentSupplyText(medication) {
    if (!medication.current_supply) return ''

    return `<p class="text-xs text-on-surface-variant">${this.escapeHtml(this.t("currentSupply"))}: ${this.escapeHtml(medication.current_supply)}</p>`
  }

  packageQuantity(result) {
    const quantity = result.package_quantity
    if (quantity === null || quantity === undefined || quantity === "") return null

    return String(quantity)
  }

  addMedicationUrl(result, barcode) {
    const url = new URL(this.newMedicationUrl, window.location.origin)
    url.searchParams.set("name", result.name || result.display || "")

    if (result.category) {
      url.searchParams.set("category", result.category)
    }

    if (result.description) {
      url.searchParams.set("description", result.description)
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

  renderPilLink(result) {
    const url = this.safeExternalUrl(result.pil_url)
    if (!url) return ''

    return `
      <a
        href="${this.hrefAttribute(url)}"
        target="_blank"
        rel="noopener noreferrer"
        class="mt-2 inline-flex text-xs font-medium text-primary underline-offset-2 hover:underline"
        data-testid="pil-link"
      >${this.escapeHtml(this.t("pilLink"))}</a>
    `
  }

  renderSpcLink(result) {
    const url = this.safeExternalUrl(result.spc_url)
    if (!url) return ''

    return `
      <a
        href="${this.hrefAttribute(url)}"
        target="_blank"
        rel="noopener noreferrer"
        class="ml-3 mt-2 inline-flex text-xs font-medium text-primary underline-offset-2 hover:underline"
        data-testid="spc-link"
      >${this.escapeHtml(this.t("spcLink"))}</a>
    `
  }

  barcodeQuery(query) {
    const normalized = String(query || "").trim().replace(/\D/g, "")
    return /^\d{13,14}$/.test(normalized) ? normalized : null
  }

  safeExternalUrl(url) {
    try {
      const parsed = new URL(String(url || ""))
      return parsed.protocol === "https:" ? parsed.toString() : null
    } catch (error) {
      return null
    }
  }

  escapeHtml(str) {
  return String(str || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;')
}

  hrefAttribute(url) {
    const link = document.createElement('a')
    link.setAttribute('href', String(url || ''))
    return link.getAttribute('href') || ''
  }

  get csrfToken() {
    return document.querySelector('meta[name="csrf-token"]')?.content || ''
  }

  get today() {
    return new Date().toISOString().slice(0, 10)
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

  reviewPromptSummaryText(count, riskLevel) {
    const rules = new Intl.PluralRules(document.documentElement.lang || "en")
    const category = rules.select(count)
    const templates = this.translationsValue?.reviewPromptSummary || {}
    const template = templates[category] || templates.other || templates.one || ""
    return template
      .replace("%{count}", count)
      .replace("%{risk_level}", riskLevel)
  }

  reviewPromptFilteredText(count) {
    const rules = new Intl.PluralRules(document.documentElement.lang || "en")
    const category = rules.select(count)
    const templates = this.translationsValue?.reviewPromptFiltered || {}
    const template = templates[category] || templates.other || templates.one || ""
    return template.replace("%{count}", count)
  }
}
