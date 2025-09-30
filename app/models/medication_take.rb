# frozen_string_literal: true

class MedicationTake < ApplicationRecord
  belongs_to :prescription, optional: true
  belongs_to :person_medicine, optional: true

  validates :taken_at, presence: true
  validate :exactly_one_source

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

  def exactly_one_source
    sources = [prescription_id, person_medicine_id].compact
    return if sources.one?

    errors.add(:base, 'Must have exactly one source (prescription or person_medicine)')
  end
end
