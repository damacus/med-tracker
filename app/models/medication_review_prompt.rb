# frozen_string_literal: true

class MedicationReviewPrompt < ApplicationRecord
  STATUSES = %w[
    needs_review
    reviewed_with_practitioner
    expected_prescribed_combination
    not_relevant
    hidden_low_signal
  ].freeze
  PRACTITIONER_REVIEW_STATUSES = %w[reviewed_with_practitioner expected_prescribed_combination].freeze
  SNAPSHOT_ATTRIBUTES = %w[
    household_id
    person_id
    primary_medication_id
    interacting_medication_id
    evidence_record_id
    risk_level
    match_confidence
    primary_medication_name
    interacting_medication_name
    evidence_source_name
    evidence_source_url
    evidence_source_checked_on
    evidence_text
  ].freeze

  belongs_to :household
  belongs_to :person
  belongs_to :primary_medication, class_name: 'Medication'
  belongs_to :interacting_medication, class_name: 'Medication'
  belongs_to :evidence_record, class_name: 'MedicationReviewEvidenceRecord',
                               inverse_of: :medication_review_prompts
  belongs_to :reviewed_by_membership, class_name: 'HouseholdMembership', optional: true

  validates :status, inclusion: { in: STATUSES }
  validates :risk_level, inclusion: { in: MedicationReviewEvidenceRecord::RISK_LEVELS }
  validates :match_confidence, inclusion: { in: MedicationReviewEvidenceRecord::MATCH_CONFIDENCES }
  validates :primary_medication_name, :interacting_medication_name, :evidence_source_name, :evidence_source_url,
            :evidence_source_checked_on, :evidence_text, presence: true
  validates :practitioner_name, :practitioner_role, :reviewed_on, presence: true, if: :practitioner_review_status?
  validate :associations_belong_to_household
  validate :evidence_snapshot_is_immutable, on: :update

  scope :hidden_low_signal, -> { where(status: 'hidden_low_signal') }
  scope :visible_by_default, -> { where.not(status: 'hidden_low_signal') }

  def practitioner_review_status?
    status.in?(PRACTITIONER_REVIEW_STATUSES)
  end

  private

  def associations_belong_to_household
    validate_household_association(:person)
    validate_household_association(:primary_medication)
    validate_household_association(:interacting_medication)
    validate_household_association(:reviewed_by_membership) if reviewed_by_membership
  end

  def validate_household_association(association_name)
    associated_record = public_send(association_name)
    return if associated_record.blank? || associated_record.household_id == household_id

    errors.add(association_name, 'must belong to the same household')
  end

  def evidence_snapshot_is_immutable
    SNAPSHOT_ATTRIBUTES.each do |attribute|
      if will_save_change_to_attribute?(attribute)
        errors.add(attribute,
                   'cannot be changed after the review prompt is created')
      end
    end
  end
end
