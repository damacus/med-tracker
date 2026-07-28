import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
	static targets = [
		"selection",
		"medicineRow",
		"batchRow",
		"quantity",
		"difference",
		"emptyState",
		"selectedCount",
		"summaryCount",
		"netChange",
		"submitButton",
		"submitLabel",
		"search",
	]
	static values = {
		applyLabel: String,
		selectedLabel: String,
		medicinesLabel: String,
		units: String,
	}

	connect() {
		this.refresh()
	}

	toggleMedication({ params: { id } }) {
		const selection = this.selectionFor(id)
		if (selection) {
			if (selection.checked) this.moveBatchRowToEnd(id)
			this.setSelected(id, selection.checked)
		}
		this.refresh()
	}

	removeFromBatch({ params: { id } }) {
		const selection = this.selectionFor(id)
		if (selection) selection.checked = false
		this.setSelected(id, false)
		this.refresh()
	}

	clearSelection() {
		this.selectionTargets.forEach((selection) => {
			selection.checked = false
			this.setSelected(this.medicationId(selection), false)
		})
		this.refresh()
	}

	filterMedicines() {
		const query = this.searchTarget.value.trim().toLowerCase()
		this.medicineRowTargets.forEach((row) => {
			row.hidden = !row.dataset.medicationName.includes(query)
		})
	}

	changeLocation(event) {
		window.location.assign(event.currentTarget.value)
	}

	updateQuantity() {
		this.refresh()
	}

	setToZero({ params: { id } }) {
		const quantity = this.quantityFor(id)
		if (!quantity) return

		quantity.value = "0"
		this.refresh()
	}

	refresh() {
		this.selectionTargets.forEach((selection) => {
			this.setSelected(this.medicationId(selection), selection.checked)
		})

		const selectedIds = this.selectedIds
		const validQuantities = selectedIds.every((id) => this.validQuantity(this.quantityFor(id)))
		const count = selectedIds.length

		this.emptyStateTarget.hidden = count > 0
		this.emptyStateTarget.classList.toggle("hidden", count > 0)
		this.selectedCountTarget.textContent = this.withCount(this.selectedLabelValue, count)
		this.summaryCountTarget.textContent = this.withCount(this.medicinesLabelValue, count)
		this.submitLabelTarget.textContent = this.withCount(this.applyLabelValue, count)
		this.submitButtonTarget.disabled = count === 0 || !validQuantities

		const netChange = selectedIds.reduce((total, id) => total + this.renderDifference(id), 0)
		this.netChangeTarget.textContent = this.signedQuantity(netChange)
	}

	setSelected(id, selected) {
		const batchRow = this.batchRowFor(id)
		const quantity = this.quantityFor(id)
		const medicineRow = this.medicineRowFor(id)

		if (batchRow) {
			batchRow.hidden = !selected
			batchRow.classList.toggle("hidden", !selected)
		}
		if (quantity) quantity.disabled = !selected
		if (medicineRow) medicineRow.classList.toggle("bg-primary-container/25", selected)
	}

	moveBatchRowToEnd(id) {
		const batchRow = this.batchRowFor(id)
		if (batchRow) batchRow.parentElement.append(batchRow)
	}

	renderDifference(id) {
		const row = this.batchRowFor(id)
		const quantity = this.quantityFor(id)
		const output = this.differenceFor(id)
		if (!row || !quantity || !output || !this.validQuantity(quantity)) {
			if (output) output.textContent = "—"
			return 0
		}

		const nextSupply = Number(quantity.value)
		const difference = this.round(nextSupply - Number(row.dataset.currentSupply))
		const label = this.differenceLabel(row, difference, nextSupply)
		output.textContent = difference === 0 ? label : `${this.signedQuantity(difference)} · ${label}`
		this.applyDifferenceColour(output, difference, nextSupply)
		return difference
	}

	differenceLabel(row, difference, nextSupply) {
		if (difference === 0) return row.dataset.noChangeLabel
		if (nextSupply === 0) return row.dataset.outOfStockLabel
		return difference > 0 ? row.dataset.increaseLabel : row.dataset.decreaseLabel
	}

	applyDifferenceColour(output, difference, nextSupply) {
		output.classList.remove("text-success", "text-error", "text-primary", "text-on-surface-variant")
		if (difference > 0) output.classList.add("text-success")
		else if (difference < 0 && nextSupply === 0) output.classList.add("text-error")
		else if (difference < 0) output.classList.add("text-primary")
		else output.classList.add("text-on-surface-variant")
	}

	validQuantity(quantity) {
		if (!quantity || quantity.value.trim() === "") return false

		const value = Number(quantity.value)
		return Number.isFinite(value) && value >= 0
	}

	withCount(label, count) {
		return label.replace("__COUNT__", count)
	}

	signedQuantity(value) {
		const rounded = this.round(value)
		const sign = rounded > 0 ? "+" : ""
		return `${sign}${rounded} ${this.unitsValue}`
	}

	round(value) {
		return Math.round((value + Number.EPSILON) * 100) / 100
	}

	medicationId(element) {
		return element.closest("[data-medication-id]").dataset.medicationId
	}

	get selectedIds() {
		return this.selectionTargets
			.filter((selection) => selection.checked)
			.map((selection) => this.medicationId(selection))
	}

	selectionFor(id) {
		return this.selectionTargets.find((selection) => this.medicationId(selection) === String(id))
	}

	medicineRowFor(id) {
		return this.medicineRowTargets.find((row) => row.dataset.medicationId === String(id))
	}

	batchRowFor(id) {
		return this.batchRowTargets.find((row) => row.dataset.medicationId === String(id))
	}

	quantityFor(id) {
		return this.quantityTargets.find((quantity) => quantity.dataset.medicationId === String(id))
	}

	differenceFor(id) {
		return this.differenceTargets.find((output) => output.dataset.medicationId === String(id))
	}
}
