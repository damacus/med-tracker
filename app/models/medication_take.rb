# frozen_string_literal: true

# MedicationTake records when a dose of medicine was administered
class MedicationTake < ApplicationRecord
  include OtelInstrumented

  belongs_to :prescription, optional: true
  belongs_to :person_medicine, optional: true

  # CRITICAL: Audit trail for medication safety
  # Every dose must be tracked for legal/clinical compliance
  # Tracks: all medication administrations, timing, amounts
  # Cannot be edited or deleted - immutable record for patient safety
  # @see docs/audit-trail.md
  has_paper_trail

  validates :taken_at, presence: true
  validate :exactly_one_source

  after_create :decrement_medicine_stock

  # Delegate to get the source (prescription or person_medicine)
  def source
    prescription || person_medicine
  end

  def person
    prescription&.person || person_medicine&.person
  end

  def medicine
    prescription&.medicine || person_medicine&.medicine
  end

  private

  def decrement_medicine_stock
    return unless medicine

    # rubocop:disable Rails/SkipsModelValidations -- Intentional: atomic SQL update prevents race conditions
    # Using update_all with GREATEST ensures stock never goes negative even under concurrent requests
    medicine.class.where(id: medicine.id).update_all('stock = GREATEST(COALESCE(stock, 0) - 1, 0)')

    return if medicine.current_supply.blank?

    medicine.class.where(id: medicine.id).update_all('current_supply = GREATEST(COALESCE(current_supply, 0) - 1, 0)')

    # rubocop:enable Rails/SkipsModelValidations
  end

  def exactly_one_source
    sources = [prescription_id, person_medicine_id].compact
    return if sources.one?

    errors.add(:base, 'Must have exactly one source (prescription or person_medicine)')
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
    if prescription_id
      attrs['medication_take.source_type'] = 'prescription'
      attrs['medication_take.prescription_id'] = prescription_id.to_s
    elsif person_medicine_id
      attrs['medication_take.source_type'] = 'person_medicine'
      attrs['medication_take.person_medicine_id'] = person_medicine_id.to_s
    end

    attrs
  end
end
