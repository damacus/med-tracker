# frozen_string_literal: true

class HouseholdRetentionHold < ApplicationRecord
  belongs_to :household
  belongs_to :approved_by_account, class_name: 'Account'
  belongs_to :released_by_account, class_name: 'Account', optional: true

  enum :status, { active: 'active', released: 'released' }, validate: true

  validates :reason, :review_on, :placed_at, presence: true
  validate :review_must_be_future, on: :create
  validate :release_fields_match_status
  validate :preservation_evidence_immutable, on: :update
  validate :release_evidence_immutable, on: :update

  private

  def review_must_be_future
    return if review_on.blank? || review_on.future?

    errors.add(:review_on, 'must be in the future')
  end

  def release_fields_match_status
    return unless released?
    return if released_at.present? && released_by_account.present?

    errors.add(:base, 'Released holds require a releaser and release time')
  end

  def preservation_evidence_immutable
    immutable_fields = %w[household_id approved_by_account_id reason review_on placed_at]
    return unless immutable_fields.any? { |attribute| will_save_change_to_attribute?(attribute) }

    errors.add(:base, 'Retention hold evidence is immutable')
  end

  def release_evidence_immutable
    return unless attribute_in_database('status') == 'released'

    immutable_fields = %w[status released_by_account_id released_at]
    return unless immutable_fields.any? { |attribute| will_save_change_to_attribute?(attribute) }

    errors.add(:base, 'Retention hold release evidence is immutable')
  end
end
