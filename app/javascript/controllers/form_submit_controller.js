import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
	submitForm(e) {
		const form = e.target.closest('form')
		if (form) {
			form.requestSubmit()
		}
	}
}
