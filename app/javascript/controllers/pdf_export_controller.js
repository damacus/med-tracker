import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["label"]
  static values = {
    readyLabel: String,
    preparingLabel: String
  }

  connect() {
    this.reset = this.reset.bind(this)
    document.addEventListener("turbo:before-cache", this.reset)
  }

  disconnect() {
    document.removeEventListener("turbo:before-cache", this.reset)
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

    try {
      const response = await fetch(this.element.href, {
        credentials: "same-origin",
        redirect: "follow"
      })

      if (!this.pdfResponse(response)) {
        this.followExportPath()
        return
      }

      const pdf = await response.blob()
      if (!await this.validPdf(pdf)) {
        this.followExportPath()
        return
      }

      this.download(pdf, response.headers.get("Content-Disposition"))
    } catch (_) {
      this.followExportPath()
    } finally {
      this.reset()
    }
  }

  reset() {
    this.busy = false
    this.element.setAttribute("aria-busy", "false")
    this.element.setAttribute("aria-disabled", "false")
    this.labelTarget.textContent = this.readyLabelValue
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
}
