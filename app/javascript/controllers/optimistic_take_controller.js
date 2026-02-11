import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
    static targets = ["button"];

    submit() {
        if (this.hasButtonTarget) {
            this.buttonTarget.disabled = true;
            this.buttonTarget.textContent = "Taking…";
            this.buttonTarget.classList.add("opacity-50", "cursor-not-allowed");
        }
    }
}
