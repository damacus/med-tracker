import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["video", "status", "result", "manualInput", "startButton", "stopButton", "scannerRegion"]
  static values = {
    formats: { type: Array, default: ["EAN_13", "EAN_8", "CODE_128", "CODE_39", "QR_CODE"] },
    state: { type: String, default: "idle" }
  }

  connect() {
    this.scanner = null
    this.transition("idle")
  }

  disconnect() {
    this.stopScanning()
  }

  async start() {
    if (this.stateValue === "scanning") return

    this.transition("requesting")

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "environment" } })
      stream.getTracks().forEach(track => track.stop())
    } catch (error) {
      this.transition("denied")
      this.dispatch("denied", { detail: { error: error.message } })
      return
    }

    try {
      const { Html5Qrcode } = await import("html5-qrcode")
      const scannerId = this.scannerRegionTarget.id

      this.scanner = new Html5Qrcode(scannerId)

      const config = {
        fps: 10,
        qrbox: { width: 250, height: 150 },
        formatsToSupport: this.formatsValue.map(f => Html5Qrcode.getSupportedFormats
          ? f : f),
        aspectRatio: 1.777
      }

      await this.scanner.start(
        { facingMode: "environment" },
        config,
        (decodedText, decodedResult) => this.onDecodeSuccess(decodedText, decodedResult),
        (_errorMessage) => {}
      )

      this.transition("scanning")
    } catch (error) {
      this.transition("error")
      this.dispatch("error", { detail: { error: error.message } })
    }
  }

  async stop() {
    await this.stopScanning()
    this.transition("idle")
  }

  async stopScanning() {
    if (this.scanner) {
      try {
        const state = this.scanner.getState()
        if (state === 2 || state === 3) {
          await this.scanner.stop()
        }
      } catch (_error) {
        // Scanner may already be stopped
      }
      this.scanner = null
    }
  }

  onDecodeSuccess(decodedText, _decodedResult) {
    this.stopScanning()
    this.transition("decoded")

    if (this.hasResultTarget) {
      this.resultTarget.textContent = decodedText
    }

    this.dispatch("decoded", { detail: { barcode: decodedText } })
  }

  submitManual() {
    if (!this.hasManualInputTarget) return

    const value = this.manualInputTarget.value.trim()
    if (!value) return

    this.dispatch("decoded", { detail: { barcode: value } })
  }

  transition(newState) {
    this.stateValue = newState
    this.element.dataset.scannerState = newState

    this.updateVisibility()
    this.updateStatus()
  }

  updateVisibility() {
    const state = this.stateValue

    if (this.hasStartButtonTarget) {
      this.startButtonTarget.hidden = (state === "scanning" || state === "requesting")
    }

    if (this.hasStopButtonTarget) {
      this.stopButtonTarget.hidden = (state !== "scanning")
    }

    if (this.hasScannerRegionTarget) {
      this.scannerRegionTarget.hidden = (state !== "scanning" && state !== "requesting")
    }
  }

  updateStatus() {
    if (!this.hasStatusTarget) return

    const messages = {
      idle: "",
      requesting: "Requesting camera access…",
      scanning: "Point your camera at a barcode",
      decoded: "Barcode scanned successfully!",
      denied: "Camera access was denied. Please use manual entry below.",
      error: "Scanner error. Please use manual entry below."
    }

    this.statusTarget.textContent = messages[this.stateValue] || ""

    this.statusTarget.classList.toggle("text-destructive",
      this.stateValue === "denied" || this.stateValue === "error")
    this.statusTarget.classList.toggle("text-green-600",
      this.stateValue === "decoded")
  }
}
