# frozen_string_literal: true

class MedicationOnboardingCreateService
  Result = Data.define(:success, :medication, :schedule, :restocked) do
    def restocked?
      restocked
    end
  end

  SCHEDULE_CONFIG_KEYS = [
    :schedule_type,
    :frequency,
    :as_needed,
    :tapering_plan,
    { times: [] },
    { weekdays: [] },
    { dates: [] },
    { taper_steps: %i[
      start_date
      end_date
      amount
      unit
      frequency
      max_daily_doses
      min_hours_between_doses
    ] }
  ].freeze

  attr_reader :medication, :schedule_attributes, :people_scope, :medication_scope

  def initialize(medication:, schedule_attributes: nil, people_scope: nil, medication_scope: nil)
    @medication = medication
    @schedule_attributes = schedule_attributes
    @people_scope = people_scope
    @medication_scope = medication_scope
  end

  def call
    return invalid_schedule_result if schedule_requested? && invalid_schedule_date_range?

    existing_medication = matching_existing_medication
    return restock_existing_medication(existing_medication) if existing_medication

    return save_medication unless schedule_requested?

    save_medication_with_plan
  end

  private

  def save_medication
    medication.paper_trail_event = 'create'
    Result.new(success: medication.save, medication: medication, schedule: nil, restocked: false)
  end

  def save_medication_with_plan
    record = nil
    success = false

    ActiveRecord::Base.transaction do
      record, success = persist_medication_with_plan
    end

    reset_rolled_back_records unless success

    Result.new(success: success, medication: medication, schedule: schedule_result(record), restocked: false)
  end

  def persist_medication_with_plan
    assign_medication_schedule_defaults
    medication.paper_trail_event = 'create'
    raise ActiveRecord::Rollback unless medication.save

    record = build_plan_record(medication)
    return [record, true] if record.save

    copy_record_errors(record)
    raise ActiveRecord::Rollback
  end

  def schedule_requested?
    schedule_attributes.present? && people_scope.present?
  end

  def invalid_schedule_date_range?
    start_date = cast_schedule_date(schedule_attributes[:start_date])
    end_date = cast_schedule_date(schedule_attributes[:end_date])
    return false if start_date.blank? || end_date.blank?

    end_date < start_date
  end

  def invalid_schedule_result
    medication.errors.add(:end_date, 'must be after the start date')
    Result.new(success: false, medication: medication, schedule: nil, restocked: false)
  end

  def cast_schedule_date(value)
    ActiveRecord::Type::Date.new.cast(value)
  end

  def build_plan_record(plan_medication)
    MedicationOnboardingPlanBuilder.new(
      person: schedule_person,
      medication: plan_medication,
      dosage: primary_dosage_option(plan_medication),
      schedule_attributes: schedule_attributes,
      schedule_config: normalized_schedule_config
    ).record
  end

  def assign_medication_schedule_defaults
    medication.default_schedule_type = schedule_attributes[:schedule_type].presence || 'multiple_daily'
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

  def matching_existing_medication
    return nil unless medication_scope
    return nil unless stock_merge_candidate?

    MedicationInventoryMatcher.new(scope: medication_scope).call(medication)
  end

  def stock_merge_candidate?
    medication.current_supply.to_d.positive? || incoming_tracked_dosages.any?
  end

  def restock_existing_medication(existing_medication)
    record = nil
    success = false

    ActiveRecord::Base.transaction do
      merge_stock_into_existing_medication(existing_medication)
      record = build_plan_record(existing_medication) if schedule_requested?

      if record.blank? || record.save
        success = true
      else
        copy_record_errors(record)
        raise ActiveRecord::Rollback
      end
    end

    Result.new(success: success, medication: existing_medication, schedule: schedule_result(record), restocked: true)
  end

  def merge_stock_into_existing_medication(existing_medication)
    if dosage_stock_mergeable?(existing_medication)
      return merge_dosage_stock_into_existing_medication(existing_medication)
    end

    quantity = medication.current_supply.to_d
    existing_medication.restock!(quantity: quantity) if quantity.positive?
  end

  def dosage_stock_mergeable?(existing_medication)
    incoming_tracked_dosages.any? && existing_medication.dosage_records.any? &&
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
    return raw.permit(*SCHEDULE_CONFIG_KEYS).to_h if raw.respond_to?(:permit)
    return raw.to_h if raw.respond_to?(:to_h)

    JSON.parse(raw.to_s)
  rescue JSON::ParserError
    {}
  end

  def schedule_result(record)
    record if record.is_a?(Schedule)
  end

  def copy_record_errors(record)
    record.errors.full_messages.each do |message|
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
