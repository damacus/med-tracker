import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
	static targets = ["barcode", "matchPanel", "noMatchPanel", "matchName", "matchLocation", "matchSupply"]
	static values = { matchUrl: String }

	barcodeDecoded(event) {
		if (!this.hasBarcodeTarget) return

		this.barcodeTarget.value = event.detail.barcode || ""
		this.lookupStockMatch()
	}

	barcodeChanged() {
		this.lookupStockMatch()
	}

	async lookupStockMatch() {
		const barcode = this.barcodeTarget.value.trim()
		this.lookupSequence = (this.lookupSequence || 0) + 1
		const sequence = this.lookupSequence

		if (!barcode) {
			this.hideFeedback()
			return
		}

		try {
			const url = new URL(this.matchUrlValue, window.location.origin)
			url.searchParams.set("q", barcode)

			const response = await fetch(url.toString(), {
				credentials: "same-origin",
				headers: { Accept: "application/json" },
			})

			if (sequence !== this.lookupSequence) return
			if (!response.ok) {
				this.hideFeedback()
				return
			}

			this.renderMatch(await response.json())
		} catch (_error) {
			if (sequence === this.lookupSequence) this.hideFeedback()
		}
	}

	renderMatch(payload) {
		if (payload.matched && payload.medication) {
			this.showMatchedMedication(payload.medication)
			return
		}

		if (payload.matched === false) {
			this.hideMatchPanel()
			this.showNoMatchPanel()
			return
		}

		this.hideFeedback()
	}

	showMatchedMedication(medication) {
		this.matchNameTarget.textContent = medication.name || medication.display_name || ""
		this.matchLocationTarget.textContent = medication.location || ""
		this.matchSupplyTarget.textContent = medication.current_supply || ""
		this.showMatchPanel()
		this.hideNoMatchPanel()
	}

	showMatchPanel() {
		if (this.hasMatchPanelTarget) this.matchPanelTarget.hidden = false
	}

	hideMatchPanel() {
		if (this.hasMatchPanelTarget) this.matchPanelTarget.hidden = true
	}

	showNoMatchPanel() {
		if (this.hasNoMatchPanelTarget) this.noMatchPanelTarget.hidden = false
	}

	hideNoMatchPanel() {
		if (this.hasNoMatchPanelTarget) this.noMatchPanelTarget.hidden = true
	}

	hideFeedback() {
		this.hideMatchPanel()
		this.hideNoMatchPanel()
	}
}
