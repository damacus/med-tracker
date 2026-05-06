import { Controller } from "@hotwired/stimulus"
import {
  getFailedTakes,
  getQueuedTakes,
  getSnapshot,
  queueTake,
  refreshSnapshot,
  syncQueuedTakes
} from "controllers/offline_store"

export default class extends Controller {
  static targets = ["connection", "snapshotAge", "pendingCount", "failedCount", "today", "people", "failures"]
  static values = { snapshotUrl: String, syncUrl: String }

  async connect() {
    this.renderConnection()
    window.addEventListener("online", this.online)
    window.addEventListener("offline", this.offline)
    window.addEventListener("medtracker:offline-take-queued", this.refresh)

    if (navigator.onLine) await this.refreshSnapshot()
    await this.sync()
    await this.render()
  }

  disconnect() {
    window.removeEventListener("online", this.online)
    window.removeEventListener("offline", this.offline)
    window.removeEventListener("medtracker:offline-take-queued", this.refresh)
  }

  online = async () => {
    this.renderConnection()
    await this.refreshSnapshot()
    await this.sync()
    await this.render()
  }

  offline = async () => {
    this.renderConnection()
    await this.render()
  }

  refresh = async () => {
    await this.render()
  }

  async refreshSnapshot() {
    if (!navigator.onLine) return

    try {
      await refreshSnapshot(this.snapshotUrlValue)
    } catch (_) {
      await this.render()
    }
  }

  async sync() {
    if (!navigator.onLine) return

    const result = await syncQueuedTakes(this.syncUrlValue)
    if (result.synced.length > 0) await this.refreshSnapshot()
  }

  async queue(event) {
    const button = event.currentTarget
    const sourceType = button.dataset.sourceType
    const sourceId = Number(button.dataset.sourceId)
    const doseAmount = button.dataset.doseAmount
    const doseUnit = button.dataset.doseUnit
    const snapshot = await getSnapshot()
    const data = snapshot?.payload?.data || {}
    const source = this.sourceFor(data, sourceType, sourceId)
    const medication = this.medicationForSource(data, source)

    if (!source || !medication) return

    const inventory = this.inventoryFor(data, medication, [], source)
    const take = await queueTake({
      source_type: sourceType,
      source_id: sourceId,
      dose_amount: doseAmount,
      dose_unit: doseUnit,
      taken_at: new Date().toISOString(),
      taken_from_medication_id: inventory?.id || medication.id
    })

    window.dispatchEvent(new CustomEvent("medtracker:offline-take-queued", { detail: { take } }))
  }

  async render() {
    const snapshot = await getSnapshot()
    const queued = await getQueuedTakes()
    const failed = await getFailedTakes()
    const data = snapshot?.payload?.data || {}

    this.snapshotAgeTarget.textContent = snapshot ? this.relativeAge(snapshot.cached_at) : "Not cached"
    this.pendingCountTarget.textContent = String(queued.length)
    this.failedCountTarget.textContent = String(failed.length)
    this.renderToday(data, queued)
    this.renderPeople(data, queued)
    this.renderFailures(failed)
  }

  renderConnection() {
    this.connectionTarget.textContent = navigator.onLine ? "Online" : "Offline"
  }

  renderToday(data, queued) {
    const schedules = data.schedules || []
    const personMedications = data.person_medications || []
    const items = [
      ...schedules.map((source) => ({ source, sourceType: "schedule" })),
      ...personMedications.map((source) => ({ source, sourceType: "person_medication" }))
    ]

    if (items.length === 0) {
      this.todayTarget.innerHTML = this.empty("No cached medication schedule is available.")
      return
    }

    this.todayTarget.innerHTML = items.map(({ source, sourceType }) => {
      const person = this.byId(data.people, source.person_id)
      const medication = this.byId(data.medications, source.medication_id)
      const pending = queued.filter((take) => take.source_type === sourceType && Number(take.source_id) === source.id)
      const inventory = medication ? this.inventoryFor(data, medication, queued, source) : null
      const stockMedication = inventory || medication
      const disabled = !medication || this.locallyOutOfStock(stockMedication, queued, source)
      const label = pending.length > 0 ? `${pending.length} pending` : "Take now"

      return `
        <article class="rounded-lg border border-border bg-surface-container-low p-4" data-testid="offline-dose-card">
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0">
              <p class="font-bold text-foreground">${this.escape(medication?.name || "Medication")}</p>
              <p class="mt-1 text-sm text-on-surface-variant">${this.escape(person?.name || "Person")} · ${this.escape(this.doseLabel(source))}</p>
              ${pending.length > 0 ? `<p class="mt-2 text-xs font-bold uppercase tracking-widest text-primary">Queued locally</p>` : ""}
            </div>
            <button
              type="button"
              class="rounded-lg px-4 py-2 text-sm font-bold ${disabled ? "bg-surface-container text-on-surface-variant" : "bg-primary text-on-primary"}"
              data-action="offline-shell#queue"
              data-source-type="${this.escape(sourceType)}"
              data-source-id="${source.id}"
              data-dose-amount="${this.escape(source.dose_amount || medication?.dosage_amount || "")}"
              data-dose-unit="${this.escape(source.dose_unit || medication?.dosage_unit || "")}"
              ${disabled ? "disabled" : ""}
            >${this.escape(disabled ? "Out of stock" : label)}</button>
          </div>
        </article>
      `
    }).join("")
  }

  renderPeople(data, queued) {
    const people = data.people || []
    const medications = data.medications || []

    this.peopleTarget.innerHTML = `
      <div class="rounded-lg border border-border bg-surface-container-low p-4">
        <p class="text-xs font-bold uppercase tracking-widest text-on-surface-variant">People</p>
        <div class="mt-3 space-y-2">${people.map((person) => `<p class="text-sm font-semibold">${this.escape(person.name)}</p>`).join("") || this.empty("No people cached.")}</div>
      </div>
      <div class="rounded-lg border border-border bg-surface-container-low p-4">
        <p class="text-xs font-bold uppercase tracking-widest text-on-surface-variant">Inventory</p>
        <div class="mt-3 space-y-2">${medications.map((medication) => `
          <div class="flex justify-between gap-3 text-sm">
            <span class="font-semibold">${this.escape(medication.name)}</span>
            <span class="text-on-surface-variant">${this.escape(this.localSupplyLabel(medication, queued))}</span>
          </div>
        `).join("") || this.empty("No inventory cached.")}</div>
      </div>
    `
  }

  renderFailures(failed) {
    if (failed.length === 0) {
      this.failuresTarget.innerHTML = ""
      return
    }

    this.failuresTarget.innerHTML = `
      <div class="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
        <p class="text-sm font-bold text-destructive">Sync needs attention</p>
        <div class="mt-3 space-y-2">${failed.map((failure) => `
          <p class="text-xs text-destructive/90">${this.escape(failure.failure_message)}</p>
        `).join("")}</div>
      </div>
    `
  }

  sourceFor(data, sourceType, sourceId) {
    const collection = sourceType === "schedule" ? data.schedules : data.person_medications
    return this.byId(collection, sourceId)
  }

  medicationForSource(data, source) {
    return source ? this.byId(data.medications, source.medication_id) : null
  }

  inventoryFor(data, medication, queued = [], source = null) {
    return (data.medications || []).find((candidate) =>
      candidate.name === medication.name &&
      String(candidate.dosage_amount) === String(medication.dosage_amount) &&
      candidate.dosage_unit === medication.dosage_unit &&
      !this.locallyOutOfStock(candidate, queued, source)
    )
  }

  locallyOutOfStock(medication, queued, source = null) {
    if (!medication) return true

    const supply = this.localSupply(medication, queued)
    const consumption = this.stockConsumptionFor({
      dose_amount: source?.dose_amount || medication.dosage_amount,
      dose_unit: source?.dose_unit || medication.dosage_unit
    })

    return supply === null ? medication.out_of_stock : supply < consumption
  }

  localSupplyLabel(medication, queued) {
    const supply = this.localSupply(medication, queued)
    if (supply === null) return "Untracked"

    return medication.dosage_unit === "ml" ? `${this.formatQuantity(supply)} ml` : this.formatQuantity(supply)
  }

  localSupply(medication, queued) {
    if (medication.current_supply === null || medication.current_supply === undefined) return null

    const pending = queued
      .filter((take) => Number(take.taken_from_medication_id) === Number(medication.id))
      .reduce((total, take) => total + this.stockConsumptionFor(take), 0)
    return Math.max(Number(medication.current_supply) - pending, 0)
  }

  stockConsumptionFor(take) {
    return take.dose_unit === "ml" ? Number(take.dose_amount || 0) : 1
  }

  formatQuantity(quantity) {
    return Number(quantity).toLocaleString(undefined, { maximumFractionDigits: 2 })
  }

  byId(collection, id) {
    return (collection || []).find((item) => Number(item.id) === Number(id))
  }

  doseLabel(source) {
    return [source.dose_amount, source.dose_unit].filter(Boolean).join(" ")
  }

  relativeAge(timestamp) {
    const seconds = Math.max(0, Math.round((Date.now() - Date.parse(timestamp)) / 1000))
    if (seconds < 60) return "Just now"
    const minutes = Math.round(seconds / 60)
    if (minutes < 60) return `${minutes}m ago`
    return `${Math.round(minutes / 60)}h ago`
  }

  empty(message) {
    return `<p class="text-sm text-on-surface-variant">${this.escape(message)}</p>`
  }

  escape(value) {
    const div = document.createElement("div")
    div.appendChild(document.createTextNode(String(value ?? "")))
    return div.innerHTML
  }
}
