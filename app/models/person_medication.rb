# frozen_string_literal: true

# PersonMedication represents a direct association between a person and a medication
# without requiring a formal schedule. This is useful for vitamins, supplements,
# and over-the-counter medications.
class PersonMedication < ApplicationRecord
  include TimingRestrictions

  belongs_to :person
  belongs_to :medication
  has_many :medication_takes, dependent: :destroy

  enum :dose_cycle, { daily: 0, weekly: 1, monthly: 2 }, prefix: :dose

  # CRITICAL: Audit trail for person-medication links
  # Tracks: which medications are assigned to which people, and their non-scheduled dosing rules
  # @see docs/audit-trail.md
  has_paper_trail

  scope :ordered, -> { order(:position, :id) }

  before_validation :assign_default_dose
  before_validation :assign_position, on: :create

  validates :person_id, uniqueness: { scope: :medication_id }
  validates :dose_amount, presence: true, numericality: { greater_than: 0 },
                          unless: :legacy_record_without_resolvable_dose?
  validates :dose_unit, presence: true, inclusion: { in: Medication::DOSAGE_UNITS },
                        unless: :legacy_record_without_resolvable_dose?

  def default_dose_amount
    dose_amount
  end

  def reorder(direction)
    adjacent = adjacent_record(direction)
    return false unless adjacent

    swap_positions_with(adjacent)
  end

  def cycle_period
    DoseCycle.new(dose_cycle).period
  end

  def dose_display
    DoseAmount.new(dose_amount, dose_unit).to_s.presence
  end

  private

  def assign_default_dose
    return if dose_amount.present? && dose_unit.present?
    return if medication.blank?

    self.dose_amount ||= resolved_dose_amount
    self.dose_unit ||= resolved_dose_unit
  end

  def assign_position
    return if position.present?

    self.position = person.person_medications.maximum(:position).to_i + 1
  end

  def adjacent_record(direction)
    case direction
    when 'up'
      person.person_medications.where(position: ...position).order(position: :desc, id: :desc).first
    when 'down'
      person.person_medications.where(position: (position + 1)..).order(position: :asc, id: :asc).first
    end
  end

  def swap_positions_with(other)
    self_position = position

    transaction do
      update!(position: other.position)
      other.update!(position: self_position)
    end
  end

  def legacy_record_without_resolvable_dose?
    persisted? && dose_amount.blank? && dose_unit.blank? &&
      resolved_dose_amount.blank? && resolved_dose_unit.blank?
  end

  def resolved_dose_amount
    dosage = resolved_dosage
    dosage&.amount || medication&.dosage_amount
  end

  def resolved_dose_unit
    dosage = resolved_dosage
    dosage&.unit || medication&.dosage_unit
  end

  def resolved_dosage
    return if medication.blank?

    medication.default_dosage_for_person_type(person&.person_type) || medication.dosages.first
  end
end
