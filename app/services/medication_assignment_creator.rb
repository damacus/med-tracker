# frozen_string_literal: true

class MedicationAssignmentCreator
  Result = Data.define(:success, :assignment, :record, :schedule)

  attr_reader :person, :medication_scope, :assignment

  def initialize(person:, medication_scope:, assignment:)
    @person = person
    @medication_scope = medication_scope
    @assignment = assignment
  end

  def call
    return failure unless valid_assignment?

    persist_record
  end

  private

  def persist_record
    direct_assignment? ? persist_person_medication : persist_schedule
  end

  def persist_person_medication
    person_medication = person.person_medications.build(person_medication_attributes)
    if person_medication.save
      return Result.new(success: true, assignment: assignment, record: person_medication, schedule: nil)
    end

    copy_record_errors(person_medication)
    Result.new(success: false, assignment: assignment, record: person_medication, schedule: nil)
  end

  def persist_schedule
    schedule = person.schedules.build(schedule_attributes)
    return Result.new(success: true, assignment: assignment, record: schedule, schedule: schedule) if schedule.save

    copy_record_errors(schedule)
    Result.new(success: false, assignment: assignment, record: schedule, schedule: schedule)
  end

  def copy_record_errors(record)
    record.errors.full_messages.each { |message| assignment.errors.add(:base, message) }
  end

  def valid_assignment?
    medication.present? && selected_dose.present?
  end

  def medication
    return @medication if defined?(@medication)

    @medication = medication_scope.find_by(id: assignment.medication_id)
  end

  def selected_dose
    @selected_dose ||= selected_dosage_option || selected_legacy_dose
  end

  def selected_dosage_option
    return if assignment.source_dosage_option_id.blank?

    dosage = medication&.dosage_records&.find_by(id: assignment.source_dosage_option_id)
    assignment.errors.add(:source_dosage_option, 'Select a valid predefined dose') if dosage.blank?
    dosage
  end

  def selected_legacy_dose
    return if assignment.source_dosage_option_id.present?
    return unless legacy_dose_selected?

    MedicationDosage.new(
      amount: medication.dosage_amount,
      unit: medication.dosage_unit,
      frequency: fallback_frequency,
      description: nil,
      default_for_adults: false,
      default_for_children: false,
      default_max_daily_doses: nil,
      default_min_hours_between_doses: nil,
      default_dose_cycle: 'daily'
    )
  end

  def legacy_dose_selected?
    complete_legacy_dose? && submitted_legacy_dose_matches?
  end

  def complete_legacy_dose?
    medication.present? &&
      medication.dosage_amount.present? &&
      medication.dosage_unit.present? &&
      assignment.dose_amount.present? &&
      assignment.dose_unit.present?
  end

  def submitted_legacy_dose_matches?
    BigDecimal(medication.dosage_amount.to_s) == assignment.dose_amount &&
      medication.dosage_unit == assignment.dose_unit
  end

  def schedule_attributes
    schedule_dose_attributes.merge(schedule_timing_attributes)
  end

  def person_medication_attributes
    {
      medication: medication,
      source_dosage_option: selected_dosage_option,
      dose_amount: selected_dose.amount,
      dose_unit: selected_dose.unit,
      administration_kind: person_medication_administration_kind,
      max_daily_doses: selected_dose.default_max_daily_doses,
      min_hours_between_doses: selected_dose.default_min_hours_between_doses,
      dose_cycle: selected_dose.default_dose_cycle.presence || 'daily'
    }
  end

  def schedule_dose_attributes
    {
      medication: medication,
      source_dosage_option: selected_dosage_option,
      dose_amount: selected_dose.amount,
      dose_unit: selected_dose.unit,
      frequency: selected_dose.frequency.presence || fallback_frequency
    }
  end

  def schedule_timing_attributes
    {
      start_date: Time.zone.today,
      end_date: 1.month.from_now.to_date,
      max_daily_doses: selected_dose.default_max_daily_doses,
      min_hours_between_doses: selected_dose.default_min_hours_between_doses,
      dose_cycle: selected_dose.default_dose_cycle.presence || 'daily',
      schedule_type: schedule_type,
      schedule_config: schedule_config
    }
  end

  def schedule_type
    plan_classifier.schedule_type
  end

  def person_medication_administration_kind
    plan_classifier.administration_kind
  end

  def direct_assignment?
    plan_classifier.direct?
  end

  def plan_classifier
    @plan_classifier ||= MedicationPlanClassifier.new(medication: medication)
  end

  def schedule_config
    config = medication.default_schedule_config.to_h.deep_dup
    config['schedule_type'] = schedule_type
    config['frequency'] = selected_dose.frequency.presence || fallback_frequency
    config['as_needed'] = true if schedule_type == 'prn'
    config
  end

  def fallback_frequency
    schedule_type == 'prn' ? 'As needed' : 'As directed'
  end

  def failure
    add_failure_errors
    Result.new(success: false, assignment: assignment, record: nil, schedule: nil)
  end

  def add_failure_errors
    assignment.errors.add(:medication, :blank) if medication.blank?
    assignment.errors.add(:source_dosage_option, 'Select a valid predefined dose') if missing_dose_error?
  end

  def missing_dose_error?
    medication.present? &&
      selected_dose.blank? &&
      assignment.errors[:source_dosage_option].blank?
  end
end
