# frozen_string_literal: true

class MedicationOnboardingCreateService
  Result = Data.define(:success, :medication, :schedule, :restocked) do
    def restocked?
      restocked
    end
  end

  attr_reader :medication, :schedule_attributes, :people_scope, :medication_scope

  def initialize(medication:, schedule_attributes: nil, people_scope: nil, medication_scope: nil)
    @medication = medication
    @schedule_attributes = schedule_attributes
    @people_scope = people_scope
    @medication_scope = medication_scope
  end

  def call
    existing_medication = matching_existing_medication
    return restock_existing_medication(existing_medication) if existing_medication

    return save_medication unless schedule_requested?

    save_medication_with_schedule
  end

  private

  def save_medication
    medication.paper_trail_event = "create"
    Result.new(success: medication.save, medication: medication, schedule: nil, restocked: false)
  end

  def save_medication_with_schedule
    schedule = nil
    success = false

    ActiveRecord::Base.transaction do
      schedule, success = persist_medication_with_schedule
    end

    reset_rolled_back_records unless success

    Result.new(success: success, medication: medication, schedule: schedule, restocked: false)
  end

  def persist_medication_with_schedule
    assign_medication_schedule_defaults
    medication.paper_trail_event = "create"
    raise ActiveRecord::Rollback unless medication.save

    schedule = build_schedule(medication)
    return [schedule, true] if schedule.save

    copy_schedule_errors(schedule)
    raise ActiveRecord::Rollback
  end

  def schedule_requested?
    schedule_attributes.present? && people_scope.present?
  end

  def build_schedule(schedule_medication)
    schedule_person.schedules.build(
      schedule_attributes_for(schedule_medication, primary_dosage_option(schedule_medication))
    )
  end

  def assign_medication_schedule_defaults
    medication.default_schedule_type = schedule_attributes[:schedule_type].presence || "multiple_daily"
    medication.default_schedule_config = normalized_schedule_config
  end

  def primary_dosage_option(schedule_medication)
    default_dosage_for_schedule_person(schedule_medication) || schedule_medication.dosage_records.first
  end

  def default_dosage_for_schedule_person(schedule_medication)
    if dependent_person_type?
      schedule_medication.dosage_records.find(&:default_for_children?)
    else
      schedule_medication.dosage_records.find(&:default_for_adults?)
    end
  end

  def dependent_person_type?
    %w[minor dependent_adult].include?(schedule_person.person_type.to_s)
  end

  def schedule_person
    @schedule_person ||= people_scope.find(schedule_attributes.fetch(:person_id))
  end

  def schedule_attributes_for(schedule_medication, dosage)
    schedule_dose_attributes(schedule_medication, dosage).merge(schedule_timing_attributes(dosage))
  end

  def schedule_dose_attributes(schedule_medication, dosage)
    {
      medication: schedule_medication,
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

  def matching_existing_medication
    return nil unless medication_scope
    return nil unless stock_merge_candidate?

    MedicationInventoryMatcher.new(scope: medication_scope).call(medication)
  end

  def stock_merge_candidate?
    medication.current_supply.to_d.positive? || incoming_tracked_dosages.any?
  end

  def restock_existing_medication(existing_medication)
    schedule = nil
    success = false

    ActiveRecord::Base.transaction do
      merge_stock_into_existing_medication(existing_medication)
      schedule = build_schedule(existing_medication) if schedule_requested?

      if schedule.blank? || schedule.save
        success = true
      else
        copy_schedule_errors(schedule)
        raise ActiveRecord::Rollback
      end
    end

    Result.new(success: success, medication: existing_medication, schedule: schedule, restocked: true)
  end

  def merge_stock_into_existing_medication(existing_medication)
    if dosage_stock_mergeable?(existing_medication)
      return merge_dosage_stock_into_existing_medication(existing_medication)
    end

    quantity = medication.current_supply.to_d
    existing_medication.restock!(quantity: quantity) if quantity.positive?
  end

  def dosage_stock_mergeable?(existing_medication)
    incoming_tracked_dosages.any? &&
      existing_medication.dosage_records.any? &&
      incoming_tracked_dosages.all? do |incoming_dosage|
        matching_existing_dosage(existing_medication, incoming_dosage).present?
      end
  end

  def merge_dosage_stock_into_existing_medication(existing_medication)
    incoming_tracked_dosages.each do |incoming_dosage|
      existing_dosage = matching_existing_dosage(existing_medication, incoming_dosage)
      existing_dosage.update!(current_supply: existing_dosage.current_supply.to_d + incoming_dosage.current_supply.to_d)
    end
  end

  def incoming_tracked_dosages
    @incoming_tracked_dosages ||= medication.dosage_records.reject(&:marked_for_destruction?).select do |dosage_record|
      dosage_record.current_supply.present?
    end
  end

  def matching_existing_dosage(existing_medication, incoming_dosage)
    existing_medication.dosage_records.find do |existing_dosage|
      existing_dosage.inventory_match_signature == incoming_dosage.inventory_match_signature
    end
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

  def reset_rolled_back_records
    reset_record_for_resubmission(medication)
    medication.dosage_records.each { |dosage_record| reset_record_for_resubmission(dosage_record) }
  end

  def reset_record_for_resubmission(record)
    record.id = nil
    record.instance_variable_set(:@new_record, true)
    record.instance_variable_set(:@destroyed, false)
  end
end
