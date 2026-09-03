# frozen_string_literal: true

class MedicationPausePeriod < ApplicationRecord
  include PortableIdentifiable

  PUBLIC_REASONS = %w[
    out_of_supply
    temporarily_not_needed
    clinician_advice
    side_effects
    other
  ].map(&:freeze).freeze
  LEGACY_REASON = 'reason_not_recorded'
  REASONS = [*PUBLIC_REASONS, LEGACY_REASON].map(&:freeze).freeze

  belongs_to :household
  belongs_to :schedule, optional: true, inverse_of: :medication_pause_periods
  belongs_to :person_medication, optional: true, inverse_of: :medication_pause_periods
  belongs_to :recorded_by_membership, class_name: 'HouseholdMembership', optional: true
  belongs_to :resumed_by_membership, class_name: 'HouseholdMembership', optional: true

  has_paper_trail

  before_validation :assign_household

  validates :reason, presence: true, inclusion: { in: REASONS }
  validates :started_at, :recorded_by_membership, presence: true, unless: :legacy_context?
  validates :resumed_by_membership, presence: true, if: :ended_at?
  validates :resumed_by_membership, absence: true, unless: :ended_at?
  validate :exactly_one_source
  validate :legacy_context_is_explicit
  validate :ended_at_is_not_before_started_at
  validate :related_records_belong_to_household

  private

  def assign_household
    self.household ||= schedule&.household || person_medication&.household
  end

  def exactly_one_source
    return if [schedule_id, person_medication_id].compact.one?

    errors.add(:base, 'Must have exactly one source')
  end

  def legacy_context_is_explicit
    if legacy_context?
      errors.add(:reason, 'must be reason_not_recorded for legacy context') unless reason == LEGACY_REASON
    elsif reason == LEGACY_REASON
      errors.add(:reason, 'is reserved for legacy context')
    end
  end

  def ended_at_is_not_before_started_at
    return if started_at.blank? || ended_at.blank? || ended_at >= started_at

    errors.add(:ended_at, 'must be on or after the start time')
  end

  def related_records_belong_to_household
    {
      schedule: schedule,
      person_medication: person_medication,
      recorded_by_membership: recorded_by_membership,
      resumed_by_membership: resumed_by_membership
    }.each do |attribute, record|
      next if record.blank? || household_id.blank? || record.household_id == household_id

      errors.add(attribute, 'must belong to the same household')
    end
  end
end
