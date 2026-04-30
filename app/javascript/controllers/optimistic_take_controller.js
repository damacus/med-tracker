import { Controller } from "@hotwired/stimulus";
import { queueTake } from "controllers/offline_store"

export default class extends Controller {
    static targets = ["button"];
    static values = { loadingLabel: String, queuedLabel: String }

    async submit(event) {
        if (!navigator.onLine) {
            event.preventDefault();
            await this.queueOfflineTake();
            this.markQueued();
            return;
        }

        if (this.hasButtonTarget) {
            this.buttonTarget.disabled = true;
            this.buttonTarget.textContent = this.loadingLabelValue || "Taking…";
            this.buttonTarget.classList.add("opacity-50", "cursor-not-allowed");
        }
    }

    async queueOfflineTake() {
        const formData = new FormData(this.element);
        const take = await queueTake({
            source_type: this.element.dataset.offlineSourceType,
            source_id: this.element.dataset.offlineSourceId,
            amount_ml: formData.get("amount_ml"),
            taken_at: this.takenAtValue(formData),
            taken_from_medication_id: formData.get("medication_take[taken_from_medication_id]")
        });

        window.dispatchEvent(new CustomEvent("medtracker:offline-take-queued", { detail: { take } }));
    }

    takenAtValue(formData) {
        const raw = formData.get("medication_take[taken_at]");
        return raw ? new Date(raw).toISOString() : new Date().toISOString();
    }

    markQueued() {
        if (!this.hasButtonTarget) return;

        this.buttonTarget.disabled = true;
        this.buttonTarget.textContent = this.queuedLabelValue || "Queued";
        this.buttonTarget.classList.add("opacity-50", "cursor-not-allowed");
    }
}
