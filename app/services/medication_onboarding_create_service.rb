# frozen_string_literal: true

class MedicationOnboardingCreateService
  Result = Data.define(:success, :medication, :schedule)

  attr_reader :medication, :schedule_attributes, :people_scope

  def initialize(medication:, schedule_attributes:, people_scope:)
    @medication = medication
    @schedule_attributes = schedule_attributes
    @people_scope = people_scope
  end

  def call
    schedule = nil
    success = false

    ActiveRecord::Base.transaction do
      raise ActiveRecord::Rollback unless medication.save

      schedule = build_schedule
      if schedule.save
        success = true
      else
        copy_schedule_errors(schedule)
        raise ActiveRecord::Rollback
      end
    end

    Result.new(success: success, medication: medication, schedule: schedule)
  end

  private

  def build_schedule
    schedule_person.schedules.build(schedule_attributes_for(primary_dosage_option))
  end

  def primary_dosage_option
    medication.dosage_records.find(&:default_for_adults?) || medication.dosage_records.first
  end

  def schedule_person
    people_scope.find(schedule_attributes.fetch(:person_id))
  end

  def schedule_attributes_for(dosage)
    schedule_dose_attributes(dosage).merge(schedule_timing_attributes(dosage))
  end

  def schedule_dose_attributes(dosage)
    {
      medication: medication,
      source_dosage_option: dosage,
      dose_amount: dosage.amount,
      dose_unit: dosage.unit,
      frequency: schedule_attributes[:frequency].presence || dosage.frequency
    }
  end

  def schedule_timing_attributes(_dosage)
    {
      start_date: schedule_attributes[:start_date],
      end_date: schedule_attributes[:end_date],
      max_daily_doses: schedule_attributes[:max_daily_doses],
      min_hours_between_doses: schedule_attributes[:min_hours_between_doses],
      dose_cycle: schedule_attributes[:dose_cycle],
      schedule_type: schedule_attributes[:schedule_type],
      schedule_config: normalized_schedule_config
    }
  end

  def normalized_schedule_config
    raw = schedule_attributes[:schedule_config]
    return {} if raw.blank?
    return raw.to_unsafe_h if raw.respond_to?(:to_unsafe_h)
    return raw.to_h if raw.respond_to?(:to_h)

    JSON.parse(raw.to_s)
  rescue JSON::ParserError
    {}
  end

  def copy_schedule_errors(schedule)
    schedule.errors.full_messages.each do |message|
      medication.errors.add(:base, message)
    end
  end
end
