import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
    static targets = ["button"];
    static values = { loadingLabel: String }

    submit() {
        if (this.hasButtonTarget) {
            this.buttonTarget.disabled = true;
            this.buttonTarget.textContent = this.loadingLabelValue || "Taking…";
            this.buttonTarget.classList.add("opacity-50", "cursor-not-allowed");
        }
    }
}
