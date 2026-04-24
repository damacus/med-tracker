import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = [
    "personCard",
    "scheduleTypeCard",
    "schedulePanel",
    "amountInput",
    "unitInput",
    "dosesPerDayInput",
    "hoursApartInput",
    "firstTimeInput",
    "secondTimeInput",
    "dailyTimeInput",
    "weeklyDayInput",
    "weeklyTimeInput",
    "specificDatesInput",
    "prnMaxDailyInput",
    "prnHoursApartInput",
    "taperingPlanInput",
    "startDateInput",
    "endDateInput",
    "amountField",
    "unitField",
    "dosageFrequencyField",
    "defaultMaxDailyDosesField",
    "defaultMinHoursBetweenDosesField",
    "defaultDoseCycleField",
    "defaultForAdultsField",
    "defaultForChildrenField",
    "personIdField",
    "scheduleTypeField",
    "frequencyField",
    "startDateField",
    "endDateField",
    "maxDailyDosesField",
    "minHoursBetweenDosesField",
    "doseCycleField",
    "scheduleConfigField",
    "reviewText",
    "reviewCompleteInput"
  ]

  static values = { scheduleType: { type: String, default: "multiple_daily" } }

  connect() {
    this.selectedPersonCard = this.personCardTargets.find((card) => card.dataset.personId === this.personIdFieldTarget.value)
    this.updateHiddenFields()
    this.updateVisiblePanels()
  }

  selectPerson(event) {
    this.selectedPersonCard = event.currentTarget
    this.personIdFieldTarget.value = event.currentTarget.dataset.personId
    this.updateCardSelection(this.personCardTargets, event.currentTarget)
    this.clearReview()
    this.updateHiddenFields()
  }

  selectScheduleType(event) {
    this.scheduleTypeValue = event.currentTarget.dataset.scheduleType
    this.updateCardSelection(this.scheduleTypeCardTargets, event.currentTarget)
    this.updateVisiblePanels()
    this.clearReview()
    this.updateHiddenFields()
  }

  update() {
    this.clearReview()
    this.updateHiddenFields()
  }

  review() {
    this.updateHiddenFields()
    this.reviewTextTarget.textContent = this.reviewSentence()
    this.reviewCompleteInputTarget.value = "reviewed"
  }

  updateHiddenFields() {
    const timing = this.timingDefaults()

    this.amountFieldTarget.value = this.valueFor("amountInput")
    this.unitFieldTarget.value = this.valueFor("unitInput")
    this.dosageFrequencyFieldTarget.value = timing.frequency
    this.defaultMaxDailyDosesFieldTarget.value = timing.maxDailyDoses
    this.defaultMinHoursBetweenDosesFieldTarget.value = timing.minHoursBetweenDoses
    this.defaultDoseCycleFieldTarget.value = timing.doseCycle
    this.defaultForAdultsFieldTarget.value = this.selectedPersonType() === "adult" ? "1" : "0"
    this.defaultForChildrenFieldTarget.value = this.selectedPersonType() === "adult" ? "0" : "1"

    this.scheduleTypeFieldTarget.value = this.scheduleTypeValue
    this.frequencyFieldTarget.value = timing.frequency
    this.startDateFieldTarget.value = this.valueFor("startDateInput")
    this.endDateFieldTarget.value = this.valueFor("endDateInput")
    this.maxDailyDosesFieldTarget.value = timing.maxDailyDoses
    this.minHoursBetweenDosesFieldTarget.value = timing.minHoursBetweenDoses
    this.doseCycleFieldTarget.value = timing.doseCycle
    this.scheduleConfigFieldTarget.value = JSON.stringify(this.scheduleConfig(timing))
  }

  updateVisiblePanels() {
    this.schedulePanelTargets.forEach((panel) => {
      panel.classList.toggle("hidden", panel.dataset.scheduleType !== this.scheduleTypeValue)
    })
  }

  updateCardSelection(cards, selectedCard) {
    cards.forEach((card) => {
      const selected = card === selectedCard
      card.classList.toggle("border-primary", selected)
      card.classList.toggle("bg-primary/10", selected)
      card.classList.toggle("shadow-elevation-1", selected)
      card.classList.toggle("border-outline-variant/60", !selected)
      card.classList.toggle("bg-surface", !selected)
    })
  }

  clearReview() {
    if (this.hasReviewCompleteInputTarget) this.reviewCompleteInputTarget.value = ""
  }

  timingDefaults() {
    switch (this.scheduleTypeValue) {
      case "daily":
        return { frequency: "Once daily", maxDailyDoses: "1", minHoursBetweenDoses: "24", doseCycle: "daily" }
      case "weekly":
        return { frequency: "Once weekly", maxDailyDoses: "1", minHoursBetweenDoses: "168", doseCycle: "weekly" }
      case "specific_dates":
        return { frequency: "Specific dates", maxDailyDoses: "1", minHoursBetweenDoses: "24", doseCycle: "daily" }
      case "prn":
        return {
          frequency: "As needed",
          maxDailyDoses: this.valueFor("prnMaxDailyInput", "4"),
          minHoursBetweenDoses: this.valueFor("prnHoursApartInput", "4"),
          doseCycle: "daily"
        }
      case "tapering":
        return { frequency: "Tapering schedule", maxDailyDoses: "1", minHoursBetweenDoses: "24", doseCycle: "daily" }
      default:
        return {
          frequency: this.multipleDailyFrequency(),
          maxDailyDoses: this.valueFor("dosesPerDayInput", "2"),
          minHoursBetweenDoses: this.valueFor("hoursApartInput", "12"),
          doseCycle: "daily"
        }
    }
  }

  scheduleConfig(timing) {
    const config = { schedule_type: this.scheduleTypeValue, frequency: timing.frequency }

    if (this.scheduleTypeValue === "multiple_daily") {
      config.times = this.multipleDailyTimes()
    } else if (this.scheduleTypeValue === "daily") {
      config.times = [this.valueFor("dailyTimeInput", "08:00")]
    } else if (this.scheduleTypeValue === "weekly") {
      config.weekdays = [this.valueFor("weeklyDayInput", "monday")]
      config.times = [this.valueFor("weeklyTimeInput", "08:00")]
    } else if (this.scheduleTypeValue === "specific_dates") {
      config.dates = this.valueFor("specificDatesInput").split(",").map((date) => date.trim()).filter(Boolean)
    } else if (this.scheduleTypeValue === "prn") {
      config.as_needed = true
    } else if (this.scheduleTypeValue === "tapering") {
      config.tapering_plan = this.valueFor("taperingPlanInput")
      config.taper_steps = [this.taperStep(timing)]
    }

    return config
  }

  taperStep(timing) {
    return {
      start_date: this.valueFor("startDateInput"),
      end_date: this.valueFor("endDateInput"),
      amount: this.valueFor("amountInput"),
      unit: this.valueFor("unitInput"),
      frequency: timing.frequency,
      max_daily_doses: timing.maxDailyDoses,
      min_hours_between_doses: timing.minHoursBetweenDoses
    }
  }

  reviewSentence() {
    const timing = this.timingDefaults()
    const dose = [this.valueFor("amountInput"), this.valueFor("unitInput")].filter(Boolean).join(" ")
    const person = this.selectedPersonCard?.dataset.personName || "the selected person"
    const dates = [this.valueFor("startDateInput"), this.valueFor("endDateInput")].filter(Boolean).join(" to ")

    return `${dose}, ${timing.frequency} for ${person}${dates ? ` from ${dates}` : ""}.`
  }

  multipleDailyFrequency() {
    const count = Number.parseInt(this.valueFor("dosesPerDayInput", "2"), 10)
    if (count === 1) return "Once daily"
    if (count === 2) return "Twice daily"
    if (count === 3) return "Three times daily"
    return `${count} times daily`
  }

  multipleDailyTimes() {
    const count = Math.max(Number.parseInt(this.valueFor("dosesPerDayInput", "2"), 10) || 2, 1)
    const firstTime = this.valueFor("firstTimeInput", "08:00")
    const secondTime = this.valueFor("secondTimeInput", "20:00")

    if (count === 1) return [firstTime].filter(Boolean)
    if (count === 2) return [firstTime, secondTime].filter(Boolean)

    const hoursApart = Number.parseFloat(this.valueFor("hoursApartInput", "12"))
    if (!Number.isFinite(hoursApart) || hoursApart <= 0) return [firstTime, secondTime].filter(Boolean)

    return Array.from({ length: count }, (_value, index) => this.shiftTime(firstTime, hoursApart * index)).filter(Boolean)
  }

  shiftTime(time, hours) {
    const match = /^(\d{1,2}):(\d{2})$/.exec(time)
    if (!match) return ""

    const startMinutes = Number.parseInt(match[1], 10) * 60 + Number.parseInt(match[2], 10)
    const shiftedMinutes = (startMinutes + Math.round(hours * 60)) % (24 * 60)
    const normalizedMinutes = shiftedMinutes < 0 ? shiftedMinutes + 24 * 60 : shiftedMinutes
    const hour = Math.floor(normalizedMinutes / 60).toString().padStart(2, "0")
    const minute = (normalizedMinutes % 60).toString().padStart(2, "0")

    return `${hour}:${minute}`
  }

  selectedPersonType() {
    return this.selectedPersonCard?.dataset.personType || "adult"
  }

  valueFor(targetName, fallback = "") {
    const presenceName = `has${targetName.charAt(0).toUpperCase()}${targetName.slice(1)}Target`
    if (!this[presenceName]) return fallback

    const target = this[`${targetName}Target`]
    return target?.value || fallback
  }
}
