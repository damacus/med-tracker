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
      const { Html5Qrcode } = await import("html5-qrcode")
      const scannerId = this.scannerRegionTarget.id

      this.scanner = new Html5Qrcode(scannerId)

      const config = {
        fps: 10,
        qrbox: { width: 250, height: 150 },
        aspectRatio: 1.777
      }

      await this.startScanner(Html5Qrcode, config)

      this.transition("scanning")
    } catch (error) {
      const msg = error.message || String(error)
      const isDenied = error.name === "NotAllowedError" ||
        error.name === "SecurityError" ||
        msg.toLowerCase().includes("permission") ||
        msg.toLowerCase().includes("denied")

      if (isDenied) {
        this.transition("denied")
        this.dispatch("denied", { detail: { error: msg } })
      } else {
        this.transition("error")
        this.dispatch("error", { detail: { error: msg } })
      }
    }
  }

  async startScanner(Html5Qrcode, config) {
    try {
      await this.scanner.start(
        { facingMode: "environment" },
        config,
        (decodedText, decodedResult) => this.onDecodeSuccess(decodedText, decodedResult),
        (_errorMessage) => { }
      )
    } catch (error) {
      if (!this.shouldRetryWithCameraList(error)) throw error

      const cameras = await Html5Qrcode.getCameras()
      if (!cameras || cameras.length === 0) throw error

      await this.scanner.start(
        cameras[0].id,
        config,
        (decodedText, decodedResult) => this.onDecodeSuccess(decodedText, decodedResult),
        (_errorMessage) => { }
      )
    }
  }

  shouldRetryWithCameraList(error) {
    const msg = (error.message || String(error)).toLowerCase()

    return error.name === "NotFoundError" ||
      error.name === "OverconstrainedError" ||
      msg.includes("facingmode") ||
      msg.includes("environment") ||
      msg.includes("rear camera") ||
      msg.includes("back camera")
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
