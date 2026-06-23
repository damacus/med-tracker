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

  enum :access_level, { view: 'view', record: 'record', manage: 'manage' }, validate: true
  enum :relationship_type,
       { self: 'self', parent: 'parent', family_member: 'family_member', carer: 'carer', professional: 'professional' },
       validate: true

  validates :household_membership_id, uniqueness: { scope: :person_id, conditions: -> { where(revoked_at: nil) } }
  validate :linked_records_must_belong_to_household

  scope :active, -> { where(revoked_at: nil).where('expires_at IS NULL OR expires_at > ?', Time.current) }

  def cover_access?(requested_access_level)
    ACCESS_LEVEL_ORDER.fetch(access_level) >= ACCESS_LEVEL_ORDER.fetch(requested_access_level.to_s)
  end

  private

  def linked_records_must_belong_to_household
    if household_membership&.household_id != household_id
      errors.add(:household_membership,
                 'must belong to the same household')
    end
    errors.add(:person, 'must belong to the same household') if person&.household_id != household_id
    return if granted_by_membership.blank? || granted_by_membership.household_id == household_id

    errors.add(:granted_by_membership, 'must belong to the same household')
  end
end
