import { Controller } from "@hotwired/stimulus";

let formFieldErrorId = 0;

export default class extends Controller {
  static targets = ["input", "error"];
  static values = { shouldValidate: false };

  connect() {
    if (this.hasErrorTarget) {
      if (!this.errorTarget.id) {
        formFieldErrorId += 1;
        this.errorTarget.id = `${this.inputTarget.id || "ruby-ui-form-field"}-error-${formFieldErrorId}`;
      }

      if (this.errorTarget.textContent) {
        this.shouldValidateValue = true;
      } else {
        this.errorTarget.classList.add("hidden");
      }
    }
  }

  onInvalid(error) {
    if (!this.hasErrorTarget) return;

    error.preventDefault();
    this.shouldValidateValue = true;
    this.#setErrorMessage();
  }

  onInput() {
    this.#setErrorMessage();
  }

  onChange() {
    this.#setErrorMessage();
  }

  #setErrorMessage() {
    if (!this.shouldValidateValue || !this.hasErrorTarget) return;

    if (this.inputTarget.validity.valid) {
      this.errorTarget.textContent = "";
      this.errorTarget.classList.add("hidden");
      this.inputTarget.removeAttribute("aria-invalid");
      if (this.inputTarget.getAttribute("aria-describedby") === this.errorTarget.id) {
        this.inputTarget.removeAttribute("aria-describedby");
      }
    } else {
      this.errorTarget.textContent = this.#getValidationMessage();
      this.errorTarget.classList.remove("hidden");
      this.inputTarget.setAttribute("aria-invalid", "true");
      this.inputTarget.setAttribute("aria-describedby", this.errorTarget.id);
    }
  }

  #getValidationMessage() {
    let errorMessage;

    const { validity, dataset, validationMessage } = this.inputTarget;

    if (validity.tooLong) errorMessage = dataset.tooLong;
    if (validity.tooShort) errorMessage = dataset.tooShort;
    if (validity.badInput) errorMessage = dataset.badInput;
    if (validity.typeMismatch) errorMessage = dataset.typeMismatch;
    if (validity.stepMismatch) errorMessage = dataset.stepMismatch;
    if (validity.valueMissing) errorMessage = dataset.valueMissing;
    if (validity.rangeOverflow) errorMessage = dataset.rangeOverflow;
    if (validity.rangeUnderflow) errorMessage = dataset.rangeUnderflow;
    if (validity.patternMismatch) errorMessage = dataset.patternMismatch;

    return errorMessage || validationMessage;
  }
}
