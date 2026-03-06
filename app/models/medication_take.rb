# frozen_string_literal: true

# MedicationTake records when a dose of medication was administered
class MedicationTake < ApplicationRecord
  include OtelInstrumented

  belongs_to :schedule, optional: true
  belongs_to :person_medication, optional: true
  belongs_to :taken_from_medication, class_name: 'Medication', optional: true
  belongs_to :taken_from_location, class_name: 'Location', optional: true

  # CRITICAL: Audit trail for medication safety
  # Every dose must be tracked for legal/clinical compliance
  # Tracks: all medication administrations, timing, amounts
  # Cannot be edited or deleted - immutable record for patient safety
  # @see docs/audit-trail.md
  has_paper_trail

  validates :taken_at, presence: true
  validates :amount_ml, presence: true, numericality: { greater_than: 0 }
  validate :exactly_one_source
  validate :taken_from_medication_matches_source
  validate :taken_from_location_matches_medication
  validate :taken_from_medication_is_in_stock

  before_validation :assign_taken_from_location
  after_create :decrement_medication_stock

  # Delegate to get the source (schedule or person_medication)
  def source
    schedule || person_medication
  end

  def person
    schedule&.person || person_medication&.person
  end

  def medication
    schedule&.medication || person_medication&.medication
  end

  def inventory_medication
    taken_from_medication || medication
  end

  def inventory_location
    taken_from_location || taken_from_medication&.location || medication&.location
  end

  private

  def decrement_medication_stock
    inventory = inventory_medication
    return unless inventory
    return if inventory.current_supply.blank?

    # rubocop:disable Rails/SkipsModelValidations -- Intentional: atomic SQL update prevents race conditions
    # Using update_all with GREATEST ensures supply never goes negative even under concurrent requests
    # We only decrement current_supply (the live dispensable-units counter)
    # Each 'take' represents one dispensable unit (e.g. 1 tablet, 1 sachet, or 1 multi-ml dose)
    inventory.class.where(id: inventory.id)
             .update_all('current_supply = GREATEST(COALESCE(current_supply, 0) - 1, 0)')
    # rubocop:enable Rails/SkipsModelValidations
  end

  def exactly_one_source
    sources = [schedule_id, person_medication_id].compact
    return if sources.one?

    errors.add(:base, 'Must have exactly one source (schedule or person_medication)')
  end

  def assign_taken_from_location
    self.taken_from_location ||= taken_from_medication&.location
  end

  def taken_from_medication_matches_source
    return if taken_from_medication.blank? || medication.blank?
    return if taken_from_medication.name == medication.name &&
              taken_from_medication.dosage_amount.to_s == medication.dosage_amount.to_s &&
              taken_from_medication.dosage_unit == medication.dosage_unit

    errors.add(:taken_from_medication, 'must match the assigned medication')
  end

  def taken_from_location_matches_medication
    return if taken_from_medication.blank? && taken_from_location.blank?
    return if taken_from_medication&.location == taken_from_location

    errors.add(:taken_from_location, 'must match the selected medication location')
  end

  def taken_from_medication_is_in_stock
    return if taken_from_medication.blank? || !taken_from_medication.out_of_stock?

    errors.add(:taken_from_medication, 'must be in stock')
  end

  # Custom OpenTelemetry span attributes for medication tracking
  def otel_span_attributes(operation)
    attrs = {
      'model.name' => self.class.name,
      'model.id' => id.to_s,
      'model.operation' => operation,
      'medication_take.taken_at' => taken_at&.iso8601
    }

    # Add source-specific attributes
    if schedule_id
      attrs['medication_take.source_type'] = 'schedule'
      attrs['medication_take.schedule_id'] = schedule_id.to_s
    elsif person_medication_id
      attrs['medication_take.source_type'] = 'person_medication'
      attrs['medication_take.person_medication_id'] = person_medication_id.to_s
    end

    attrs.merge(taken_from_span_attributes)
  end

  def taken_from_span_attributes
    return {} unless taken_from_medication_id

    attrs = { 'medication_take.taken_from_medication_id' => taken_from_medication_id.to_s }
    attrs['medication_take.taken_from_location_id'] = taken_from_location_id.to_s if taken_from_location_id
    attrs
  end
end
