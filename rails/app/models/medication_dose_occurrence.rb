class MedicationDoseOccurrence < ApplicationRecord
  include PortableIdentifiable
  include SyncTrackable

  OUTCOMES = %w[open taken not_taken].freeze
  REASONS = %w[refused unwell asleep medicine_unavailable clinician_advice other].freeze
  IDENTITY_ATTRIBUTES = %w[
    household_id schedule_id person_medication_id window_starts_on window_ends_on position scheduled_at
  ].freeze

  belongs_to :household
  belongs_to :schedule, optional: true, inverse_of: :medication_dose_occurrences
  belongs_to :person_medication, optional: true, inverse_of: :medication_dose_occurrences
  belongs_to :medication_take, optional: true
  belongs_to :resolved_by_membership, class_name: 'HouseholdMembership', optional: true

  has_paper_trail

  before_validation :assign_household
  before_validation :assign_window_end, on: :create

  validates :window_starts_on, presence: true
  validates :window_ends_on, presence: true, on: :create
  validates :window_ends_on, comparison: { greater_than_or_equal_to: :window_starts_on },
                             if: -> { window_starts_on.present? && window_ends_on.present? }
  validates :position, numericality: { only_integer: true, greater_than: 0 }
  validates :outcome, inclusion: { in: OUTCOMES }
  validates :reason, inclusion: { in: REASONS }, allow_nil: true
  validates :note, length: { maximum: 2000 }
  validates :resolved_at, :resolved_by_membership, presence: true, unless: :open?
  validates :resolved_at, :resolved_by_membership, absence: true, if: :open?
  validates :reason, :note, absence: true, unless: :not_taken?
  validates :medication_take, presence: true, if: :taken?
  validates :medication_take, absence: true, unless: :taken?
  validate :exactly_one_source
  validate :related_records_belong_to_household
  validate :take_matches_source
  validate :identity_is_immutable, on: :update
  validate :taken_outcome_is_immutable, on: :update

  def source = schedule || person_medication
  def open? = outcome == 'open'
  def taken? = outcome == 'taken'
  def not_taken? = outcome == 'not_taken'

  private

  def assign_window_end
    return if window_ends_on.present? || window_starts_on.blank? || source.blank?

    self.window_ends_on = schedule ? window_starts_on : routine_window_end
  end

  def routine_window_end
    DoseCycle.new(person_medication.dose_cycle).range_for(window_starts_on.in_time_zone).end.to_date
  end

  def assign_household
    self.household ||= source&.household
  end

  def exactly_one_source
    errors.add(:base, 'Must have exactly one source') unless [schedule_id, person_medication_id].compact.one?
  end

  def related_records_belong_to_household
    { schedule: schedule, person_medication: person_medication, medication_take: medication_take,
      resolved_by_membership: resolved_by_membership }.each do |attribute, record|
      next if record.blank? || household_id.blank? || record.household_id == household_id

      errors.add(attribute, 'must belong to the same household')
    end
  end

  def take_matches_source
    return if medication_take.blank? || medication_take.source == source

    errors.add(:medication_take, 'must belong to the occurrence source')
  end

  def identity_is_immutable
    IDENTITY_ATTRIBUTES.each do |attribute|
      errors.add(attribute, 'cannot be changed') if will_save_change_to_attribute?(attribute)
    end
  end

  def taken_outcome_is_immutable
    return unless outcome_in_database == 'taken'
    return if (changed - ['updated_at']).empty?

    errors.add(:base, 'Taken outcomes cannot be changed')
  end
end
