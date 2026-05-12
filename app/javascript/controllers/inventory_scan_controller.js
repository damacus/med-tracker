import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
	static targets = ["barcode"]

	barcodeDecoded(event) {
		if (!this.hasBarcodeTarget) return

		this.barcodeTarget.value = event.detail.barcode || ""
	}
}
