# frozen_string_literal: true

# MedicationTake records when a dose of medication was administered
class MedicationTake < ApplicationRecord
  include OtelInstrumented

  belongs_to :schedule, optional: true
  belongs_to :person_medication, optional: true

  # CRITICAL: Audit trail for medication safety
  # Every dose must be tracked for legal/clinical compliance
  # Tracks: all medication administrations, timing, amounts
  # Cannot be edited or deleted - immutable record for patient safety
  # @see docs/audit-trail.md
  has_paper_trail

  validates :taken_at, presence: true
  validates :amount_ml, presence: true, numericality: { greater_than: 0 }
  validate :exactly_one_source

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

  private

  def decrement_medication_stock
    return unless medication
    return if medication.current_supply.blank?

    # rubocop:disable Rails/SkipsModelValidations -- Intentional: atomic SQL update prevents race conditions
    # Using update_all with GREATEST ensures supply never goes negative even under concurrent requests
    # We only decrement current_supply (the live dispensable-units counter)
    medication.class.where(id: medication.id)
              .update_all('current_supply = GREATEST(COALESCE(current_supply, 0) - 1, 0)')
    # rubocop:enable Rails/SkipsModelValidations
  end

  def exactly_one_source
    sources = [schedule_id, person_medication_id].compact
    return if sources.one?

    errors.add(:base, 'Must have exactly one source (schedule or person_medication)')
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

    attrs
  end
end
