# frozen_string_literal: true

class PersonAccessGrant < ApplicationRecord
  ACCESS_LEVEL_ORDER = {
    'view' => 0,
    'record' => 1,
    'manage' => 2
  }.freeze

  belongs_to :household
  belongs_to :household_membership
  belongs_to :person
  belongs_to :granted_by_membership, class_name: 'HouseholdMembership', optional: true
  belongs_to :carer_relationship, optional: true

  enum :access_level, { view: 'view', record: 'record', manage: 'manage' }, validate: true
  enum :relationship_type,
       { self: 'self', parent: 'parent', family_member: 'family_member', carer: 'carer', professional: 'professional' },
       validate: true

  validates :household_membership_id, uniqueness: { scope: :person_id, conditions: -> { where(revoked_at: nil) } }
  validate :membership_must_belong_to_household
  validate :person_must_belong_to_household
  validate :relationship_must_belong_to_household
  validate :grantor_must_belong_to_household
  validate :delegation_must_match_relationship

  scope :active, -> { where(revoked_at: nil).where('expires_at IS NULL OR expires_at > ?', Time.current) }

  def cover_access?(requested_access_level)
    ACCESS_LEVEL_ORDER.fetch(access_level) >= ACCESS_LEVEL_ORDER.fetch(requested_access_level.to_s)
  end

  def cover_expiry?(requested_expires_at)
    return expires_at.nil? if requested_expires_at.nil?

    expires_at.nil? || expires_at >= requested_expires_at
  end

  private

  def membership_must_belong_to_household
    return if household_membership&.household_id == household_id

    errors.add(:household_membership, 'must belong to the same household')
  end

  def person_must_belong_to_household
    errors.add(:person, 'must belong to the same household') if person&.household_id != household_id
  end

  def relationship_must_belong_to_household
    return if carer_relationship.blank? || carer_relationship.household_id == household_id

    errors.add(:carer_relationship, 'must belong to the same household')
  end

  def grantor_must_belong_to_household
    return if granted_by_membership.blank? || granted_by_membership.household_id == household_id

    errors.add(:granted_by_membership, 'must belong to the same household')
  end

  def delegation_must_match_relationship
    return unless carer_relationship

    errors.add(:person, 'must match the delegated patient') if person_id != carer_relationship.patient_id
    return if household_membership&.person_id == carer_relationship.carer_id

    errors.add(:household_membership, 'must belong to the delegated carer')
  end
end
