import { Controller } from "@hotwired/stimulus"

const BACK_CAMERA_PATTERN = /(rear|back|environment)/i
const SCANNING_STATE = 2
const PAUSED_STATE = 3

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
		if (this.stateValue === "requesting" || this.stateValue === "scanning") return

		this.transition("requesting")

		try {
			await this.stopScanning()
			const library = await this.loadLibrary()
			const camera = await this.selectCamera(library.Html5Qrcode)

			if (!camera) {
				this.transition("unavailable")
				this.dispatch("unavailable")
				return
			}

			await this.startScanner(library, camera.id)

			this.transition("scanning")
		} catch (error) {
			await this.stopScanning()
			this.handleStartError(error)
		}
	}

	async loadLibrary() {
		return window.__barcodeScannerTestLibrary || import("html5-qrcode")
	}

	async selectCamera(Html5Qrcode) {
		const cameras = await Html5Qrcode.getCameras()

		if (!Array.isArray(cameras) || cameras.length === 0) return null

		return cameras.find((camera) => BACK_CAMERA_PATTERN.test(camera.label || "")) || cameras[0]
	}

	async startScanner(library, cameraId) {
		const scannerId = this.scannerRegionTarget.id
		const formatsToSupport = this.formatsToSupport(library.Html5QrcodeSupportedFormats)
		const constructorConfig = formatsToSupport.length > 0 ? { formatsToSupport } : {}
		this.scanner = new library.Html5Qrcode(scannerId, constructorConfig)

		await this.scanner.start(
			cameraId,
			{ fps: 10, qrbox: { width: 250, height: 150 }, aspectRatio: 1.777 },
			(decodedText, decodedResult) => this.onDecodeSuccess(decodedText, decodedResult),
			(_errorMessage) => { }
		)
	}

	scannerConfig(_supportedFormats) {
		return {
			fps: 10,
			qrbox: { width: 250, height: 150 },
			aspectRatio: 1.777
		}
	}

	scannerConfig(supportedFormats) {
		const config = {
			fps: 10,
			qrbox: { width: 250, height: 150 },
			aspectRatio: 1.777
		}
		const formatsToSupport = this.formatsToSupport(supportedFormats)

		if (formatsToSupport.length > 0) {
			config.formatsToSupport = formatsToSupport
		}

		return config
	}

	formatsToSupport(supportedFormats) {
		if (!supportedFormats) return []

		return this.formatsValue
			.map((format) => supportedFormats[format])
			.filter((format) => format !== undefined)
	}

	handleStartError(error) {
		const msg = error.message || String(error)

		if (this.isDeniedError(error)) {
			this.transition("denied")
			this.dispatch("denied", { detail: { error: msg } })
			return
		}

		this.transition("error")
		this.dispatch("error", { detail: { error: msg } })
	}

	isDeniedError(error) {
		const msg = (error.message || String(error)).toLowerCase()

		return error.name === "NotAllowedError" ||
			error.name === "SecurityError" ||
			msg.includes("permission") ||
			msg.includes("denied")
	}

	async stop() {
		await this.stopScanning()
		this.transition("idle")
	}

	async stopScanning() {
		if (!this.scanner) return

		const scanner = this.scanner
		this.scanner = null

		try {
			const state = scanner.getState()
			if (state === SCANNING_STATE || state === PAUSED_STATE) {
				await scanner.stop()
			}
		} catch (_error) {
			// Scanner may already be stopped
		}

		if (typeof scanner.clear === "function") {
			try {
				await scanner.clear()
			} catch (_error) {
			}
		}
	}

	async onDecodeSuccess(decodedText, _decodedResult) {
		await this.stopScanning()
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
			unavailable: "No camera was found. Please use manual entry below.",
			error: "Scanner error. Please use manual entry below."
		}

		this.statusTarget.textContent = messages[this.stateValue] || ""

		this.statusTarget.classList.toggle("text-destructive",
			this.stateValue === "denied" || this.stateValue === "unavailable" || this.stateValue === "error")
		this.statusTarget.classList.toggle("text-green-600",
			this.stateValue === "decoded")
	}
}
