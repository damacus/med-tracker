import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
    static values = {
        dismissAfter: { type: Number, default: 5000 },
    };

    connect() {
        if (this.dismissAfterValue > 0) {
            this.timeout = setTimeout(() => {
                this.dismiss();
            }, this.dismissAfterValue);
        }
    }

    disconnect() {
        if (this.timeout) {
            clearTimeout(this.timeout);
        }
    }

    dismiss() {
        this.element.classList.add("opacity-0", "transition-opacity", "duration-300");
        setTimeout(() => {
            this.element.remove();
        }, 300);
    }
}
