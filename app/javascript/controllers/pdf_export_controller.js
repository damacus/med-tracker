import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["label"]
  static values = {
    fallbackLocation: String,
    readyLabel: String,
    preparingLabel: String
  }

  initialize() {
    this.cancel = this.cancel.bind(this)
  }

  connect() {
    document.addEventListener("turbo:before-cache", this.cancel)
  }

  disconnect() {
    this.cancel()
    document.removeEventListener("turbo:before-cache", this.cancel)
  }

  async prepare(event) {
    if (this.busy) {
      event.preventDefault()
      return
    }

    event.preventDefault()
    this.busy = true
    this.element.setAttribute("aria-busy", "true")
    this.element.setAttribute("aria-disabled", "true")
    this.labelTarget.textContent = this.preparingLabelValue
    const request = new AbortController()
    this.request = request

    try {
      const response = await fetch(this.element.href, {
        credentials: "same-origin",
        redirect: "follow",
        signal: request.signal
      })
      if (!this.activeRequest(request)) return

      if (response.redirected) {
        this.followFallbackPath()
        return
      }

      if (!this.pdfResponse(response)) {
        this.followExportPath()
        return
      }

      const pdf = await response.blob()
      if (!this.activeRequest(request)) return
      if (!await this.validPdf(pdf)) {
        if (!this.activeRequest(request)) return
        this.followExportPath()
        return
      }

      if (!this.activeRequest(request)) return

      this.download(pdf, response.headers.get("Content-Disposition"))
    } catch (_) {
      if (!this.activeRequest(request)) return

      this.followExportPath()
    } finally {
      this.complete(request)
    }
  }

  reset() {
    this.busy = false
    this.element.setAttribute("aria-busy", "false")
    this.element.setAttribute("aria-disabled", "false")
    this.labelTarget.textContent = this.readyLabelValue
  }

  cancel() {
    const request = this.request
    this.request = undefined
    request?.abort()
    this.reset()
  }

  complete(request) {
    if (!this.activeRequest(request)) return

    this.request = undefined
    this.reset()
  }

  activeRequest(request) {
    return this.request === request && !request.signal.aborted
  }

  pdfResponse(response) {
    return response.ok && response.headers.get("Content-Type")?.startsWith("application/pdf")
  }

  async validPdf(pdf) {
    return (await pdf.slice(0, 4).text()) === "%PDF"
  }

  download(pdf, contentDisposition) {
    const download = document.createElement("a")
    const objectUrl = URL.createObjectURL(pdf)
    const focusedElement = document.activeElement

    download.href = objectUrl
    download.download = this.filename(contentDisposition)
    download.tabIndex = -1
    download.setAttribute("aria-hidden", "true")
    document.body.append(download)
    download.click()
    download.remove()
    if (document.activeElement !== focusedElement) focusedElement?.focus({ preventScroll: true })
    window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0)
  }

  filename(contentDisposition) {
    const filename = contentDisposition?.match(/filename="?([^";]+)"?/i)?.[1]
    return filename || "medtracker-report.pdf"
  }

  followExportPath() {
    window.location.assign(this.element.href)
  }

  followFallbackPath() {
    window.location.assign(this.fallbackLocationValue)
  }
}
