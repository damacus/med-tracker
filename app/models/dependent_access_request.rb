# frozen_string_literal: true

# DependentAccessRequest captures a parent-initiated request for access to a dependent.
# Requests do not grant access until an administrator approves them.
class DependentAccessRequest < ApplicationRecord
  belongs_to :requester, class_name: 'User'
  belongs_to :reviewer, class_name: 'User', optional: true
  belongs_to :carer, class_name: 'Person'
  belongs_to :patient, class_name: 'Person'

  has_paper_trail

  enum :status, { pending: 0, approved: 1, rejected: 2 }, validate: true

  validates :relationship_type, presence: true, inclusion: { in: %w[parent] }
  validates :patient_id, uniqueness: {
    scope: :carer_id,
    conditions: -> { pending },
    message: 'already has a pending access request for this parent'
  }
  validate :requester_must_be_parent
  validate :requester_must_match_carer
  validate :carer_must_be_adult_with_capacity
  validate :patient_must_require_carer
  validate :active_relationship_must_not_exist, on: :create

  scope :recent_first, -> { order(created_at: :desc) }

  def approve!(reviewer:)
    ensure_pending!
    ensure_reviewer_is_not_requester!(reviewer)

    transaction do
      DependentRelationshipAssigner.new(
        carer: carer,
        dependent_ids: [patient_id],
        relationship_type: relationship_type
      ).call

      update!(status: :approved, reviewer: reviewer, reviewed_at: Time.current)
    end
  end

  def reject!(reviewer:)
    ensure_pending!

    update!(status: :rejected, reviewer: reviewer, reviewed_at: Time.current)
  end

  private

  def ensure_pending!
    return if pending?

    errors.add(:status, 'must be pending')
    raise ActiveRecord::RecordInvalid, self
  end

  def ensure_reviewer_is_not_requester!(reviewer)
    return unless reviewer == requester

    errors.add(:reviewer, 'must be different from the requesting parent')
    raise ActiveRecord::RecordInvalid, self
  end

  def requester_must_be_parent
    errors.add(:requester, 'must be a parent') unless requester&.parent?
  end

  def requester_must_match_carer
    return if requester&.person_id.present? && requester.person_id == carer_id

    errors.add(:carer, 'must belong to the requesting parent')
  end

  def carer_must_be_adult_with_capacity
    return if carer&.person_type == 'adult' && carer.has_capacity?

    errors.add(:carer, 'must be an adult with capacity')
  end

  def patient_must_require_carer
    return if patient&.person_type.in?(%w[minor dependent_adult]) && patient.has_capacity == false

    errors.add(:patient, 'must be a child or dependent adult without capacity')
  end

  def active_relationship_must_not_exist
    return unless CarerRelationship.active.exists?(carer_id: carer_id, patient_id: patient_id)

    errors.add(:base, 'An active relationship already exists for this dependent')
  end
end
