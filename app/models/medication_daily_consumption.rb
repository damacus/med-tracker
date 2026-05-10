# frozen_string_literal: true

class MedicationDailyConsumption
  def initialize(medication)
    @medication = medication
  end

  def call
    schedule_rate + person_medication_rate
  end

  private

  attr_reader :medication

  def schedule_rate
    medication.schedules.select(&:active?).sum do |schedule|
      next 0.0 if schedule.max_daily_doses.blank?

      daily_rate(schedule) *
        consumption_for(
          schedule.effective_dose_amount(Time.zone.today),
          schedule.effective_dose_unit(Time.zone.today)
        )
    end
  end

  def person_medication_rate
    medication.person_medications.sum do |person_medication|
      next 0.0 if person_medication.max_daily_doses.blank?

      person_medication.max_daily_doses.to_f *
        consumption_for(
          person_medication.default_dose_amount,
          person_medication.dose_unit
        )
    end
  end

  def daily_rate(schedule)
    schedule.max_daily_doses.to_f / (schedule.cycle_period / 1.day)
  end

  def consumption_for(dose_amount, dose_unit)
    MedicationStockConsumption.quantity_for(dose_amount: dose_amount, dose_unit: dose_unit).to_f
  end
end
