import { Controller } from "@hotwired/stimulus";

// Connects to data-controller="ruby-ui--tabs"
export default class extends Controller {
  static targets = ["trigger", "content"];
  static values = { active: String };

  connect() {
    if (!this.hasActiveValue && this.triggerTargets.length > 0) {
      this.activeValue = this.triggerTargets[0].dataset.value;
    }
  }

  show(e) {
    this.activeValue = e.currentTarget.dataset.value;
  }

  navigate(e) {
    const moves = { ArrowLeft: -1, ArrowRight: 1, Home: 0, End: -1 };
    if (!(e.key in moves)) return;

    const availableTriggers = this.triggerTargets.filter(
      (trigger) => !trigger.disabled && trigger.getAttribute("aria-disabled") !== "true",
    );
    const currentIndex = availableTriggers.indexOf(e.currentTarget);
    if (currentIndex < 0 || availableTriggers.length === 0) return;

    e.preventDefault();
    const nextIndex = e.key === "Home"
      ? 0
      : e.key === "End"
        ? availableTriggers.length - 1
        : (currentIndex + moves[e.key] + availableTriggers.length) % availableTriggers.length;
    const nextTrigger = availableTriggers[nextIndex];

    this.activeValue = nextTrigger.dataset.value;
    nextTrigger.focus();
  }

  activeValueChanged(currentValue, previousValue) {
    if (currentValue == "" || currentValue == previousValue) return;

    this.contentTargets.forEach((el) => {
      el.classList.add("hidden");
      el.hidden = true;
    });

    this.triggerTargets.forEach((el) => {
      el.dataset.state = "inactive";
      el.setAttribute("aria-selected", "false");
    });

    const activeContent = this.activeContentTarget();
    const activeTrigger = this.activeTriggerTarget();

    if (activeContent) {
      activeContent.classList.remove("hidden");
      activeContent.hidden = false;
    }

    if (activeTrigger) {
      activeTrigger.dataset.state = "active";
      activeTrigger.setAttribute("aria-selected", "true");
    }
  }

  activeTriggerTarget() {
    return this.triggerTargets.find(
      (el) => el.dataset.value == this.activeValue,
    );
  }

  activeContentTarget() {
    return this.contentTargets.find(
      (el) => el.dataset.value == this.activeValue,
    );
  }
}
