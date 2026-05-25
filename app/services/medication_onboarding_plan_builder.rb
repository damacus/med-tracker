# frozen_string_literal: true

class MedicationOnboardingPlanBuilder
  attr_reader :person, :medication, :dosage, :schedule_attributes, :schedule_config

  def initialize(person:, medication:, dosage:, schedule_attributes:, schedule_config:)
    @person = person
    @medication = medication
    @dosage = dosage
    @schedule_attributes = schedule_attributes
    @schedule_config = schedule_config
  end

  def record
    return person.person_medications.build(person_medication_attributes) if plan_classifier.direct?

    person.schedules.build(schedule_record_attributes)
  end

  private

  def person_medication_attributes
    {
      medication: medication,
      source_dosage_option: dosage,
      dose_amount: dosage.amount,
      dose_unit: dosage.unit,
      administration_kind: plan_classifier.administration_kind,
      max_daily_doses: schedule_attributes[:max_daily_doses],
      min_hours_between_doses: schedule_attributes[:min_hours_between_doses],
      dose_cycle: schedule_attributes[:dose_cycle]
    }
  end

  def schedule_record_attributes
    schedule_dose_attributes.merge(schedule_timing_attributes)
  end

  def schedule_dose_attributes
    {
      medication: medication,
      source_dosage_option: dosage,
      dose_amount: dosage.amount,
      dose_unit: dosage.unit,
      frequency: schedule_attributes[:frequency].presence || dosage.frequency
    }
  end

  def schedule_timing_attributes
    {
      start_date: schedule_attributes[:start_date],
      end_date: schedule_attributes[:end_date],
      max_daily_doses: schedule_attributes[:max_daily_doses],
      min_hours_between_doses: schedule_attributes[:min_hours_between_doses],
      dose_cycle: schedule_attributes[:dose_cycle],
      schedule_type: schedule_attributes[:schedule_type],
      schedule_config: schedule_config
    }
  end

  def plan_classifier
    @plan_classifier ||= MedicationPlanClassifier.new(
      medication: medication,
      schedule_type: schedule_attributes[:schedule_type]
    )
  end
end
