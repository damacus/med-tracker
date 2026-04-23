# frozen_string_literal: true

# PersonMedication represents a direct association between a person and a medication
# without requiring a formal schedule. This is useful for vitamins, supplements,
# and over-the-counter medications.
class PersonMedication < ApplicationRecord
  include TimingRestrictions

  belongs_to :person
  belongs_to :medication
  belongs_to :source_dosage_option, class_name: 'MedicationDosageOption', optional: true
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
  validate :source_dosage_option_matches_medication
  validate :source_dosage_option_matches_snapshot

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
    return if medication.blank?

    self.source_dosage_option ||= resolved_dosage_record
    self.source_dosage_option ||= uniquely_matching_dosage_option
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
    dosage = resolved_dosage_record
    dosage&.amount || medication&.dosage_amount
  end

  def resolved_dose_unit
    dosage = resolved_dosage_record
    dosage&.unit || medication&.dosage_unit
  end

  def resolved_dosage_record
    return if medication.blank?

    if child_person_type?
      child_default = medication.dosage_records.child_default.first
      return child_default if child_default
    end

    medication.dosage_records.adult_default.first || medication.dosage_records.order(:amount, :id).first
  end

  def child_person_type?
    %w[minor dependent_adult].include?(person&.person_type.to_s)
  end

  def source_dosage_option_matches_medication
    return if source_dosage_option.blank? || medication.blank?
    return if source_dosage_option.medication_id == medication_id

    errors.add(:source_dosage_option, 'must belong to the selected medication')
  end

  def source_dosage_option_matches_snapshot
    return if source_dosage_option.blank? || dose_amount.blank? || dose_unit.blank?
    return if source_dosage_option.amount.to_s == dose_amount.to_s && source_dosage_option.unit == dose_unit

    errors.add(:source_dosage_option, 'must match the selected dose')
  end

  def uniquely_matching_dosage_option
    return if medication.blank? || dose_amount.blank? || dose_unit.blank?

    matches = medication.dosage_records.where(amount: dose_amount, unit: dose_unit)
    return matches.first if matches.one?

    nil
  end
end
