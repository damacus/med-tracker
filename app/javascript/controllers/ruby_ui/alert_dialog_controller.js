import { Controller } from "@hotwired/stimulus";

// Connects to data-controller="ruby-ui--alert-dialog"
export default class extends Controller {
  static targets = ["content"];
  static values = {
    open: {
      type: Boolean,
      default: false,
    },
  };

  connect() {
    this.portalElement = null;
    if (this.openValue) {
      this.open();
    }
  }

  disconnect() {
    this.cleanup();
  }

  open() {
    if (this.portalElement) return;

    document.body.insertAdjacentHTML("beforeend", this.contentTarget.innerHTML);
    this.portalElement = document.body.lastElementChild;
    // prevent scroll on body
    document.body.classList.add("overflow-hidden");
  }

  dismiss(e) {
    this.cleanup();
  }

  cleanup() {
    // allow scroll on body
    document.body.classList.remove("overflow-hidden");
    
    if (this.portalElement) {
      this.portalElement.remove();
      this.portalElement = null;
    }
    
    // Also remove this element if it's not the one we just removed
    // (In case this was called from a child action)
    if (this.element.isConnected && this.element !== document.body) {
      // Don't remove the trigger container usually, but RubyUI's original code did it
      // Actually, if we're in a Turbo Stream replace, the element will be removed anyway.
    }
  }
}
