# frozen_string_literal: true

class HouseholdMembership < ApplicationRecord
  belongs_to :household
  belongs_to :account
  belongs_to :person, optional: true

  has_many :person_access_grants, dependent: :destroy
  has_many :granted_person_access_grants,
           class_name: 'PersonAccessGrant',
           foreign_key: :granted_by_membership_id,
           dependent: :nullify,
           inverse_of: :granted_by_membership

  enum :role, { owner: 'owner', administrator: 'administrator', member: 'member' }, validate: true
  enum :status, { active: 'active', suspended: 'suspended', revoked: 'revoked' }, validate: true

  before_validation :assign_joined_at

  validates :account_id, uniqueness: { scope: :household_id }
  validate :person_must_belong_to_household
  validate :last_active_owner_cannot_be_removed, if: :removing_active_owner?

  private

  def assign_joined_at
    self.joined_at ||= Time.current
  end

  def person_must_belong_to_household
    return if person.blank? || person.household_id == household_id

    errors.add(:person, 'must belong to the same household')
  end

  def removing_active_owner?
    active_owner_in_database? && owner_access_changing? && !active_owner?
  end

  def active_owner_in_database?
    persisted? && household.operational? && role_in_database == 'owner' && status_in_database == 'active'
  end

  def owner_access_changing? = will_save_change_to_role? || will_save_change_to_status?
  def active_owner? = role == 'owner' && status == 'active'

  def last_active_owner_cannot_be_removed
    return if household.household_memberships.owner.active.where.not(id: id).exists?

    errors.add(:base, 'Last active owner cannot be removed')
  end
end
