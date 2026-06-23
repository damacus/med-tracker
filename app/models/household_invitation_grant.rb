# frozen_string_literal: true

class HouseholdInvitationGrant < ApplicationRecord
  belongs_to :household
  belongs_to :household_invitation
  belongs_to :person

  enum :access_level, { view: 'view', record: 'record', manage: 'manage' }, validate: true
  enum :relationship_type,
       { self: 'self', parent: 'parent', family_member: 'family_member', carer: 'carer', professional: 'professional' },
       validate: true

  validate :linked_records_must_belong_to_household

  private

  def linked_records_must_belong_to_household
    if household_invitation&.household_id != household_id
      errors.add(:household_invitation,
                 'must belong to the same household')
    end
    errors.add(:person, 'must belong to the same household') if person&.household_id != household_id
  end
end
