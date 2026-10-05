import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
    error() {
        this.element.classList.add("hidden");
    }
}
