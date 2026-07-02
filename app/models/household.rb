# frozen_string_literal: true

class Household < ApplicationRecord
  enum :status, { active: 'active', archived: 'archived' }, validate: true
  enum :subscription_plan, { free: 'free', family_plus: 'family_plus' }, validate: true

  belongs_to :created_by_account, class_name: 'Account', optional: true

  has_many :household_memberships, dependent: :destroy
  has_many :accounts, through: :household_memberships
  has_many :people, dependent: :restrict_with_error
  has_many :locations, dependent: :restrict_with_error
  has_many :location_memberships, dependent: :restrict_with_error
  has_many :medications, dependent: :restrict_with_error
  has_many :dosage_records, class_name: 'MedicationDosageOption', dependent: :restrict_with_error
  has_many :schedules, dependent: :restrict_with_error
  has_many :person_medications, dependent: :restrict_with_error
  has_many :medication_takes, dependent: :restrict_with_error
  has_many :health_events, dependent: :restrict_with_error
  has_many :notification_preferences, dependent: :restrict_with_error
  has_many :person_access_grants, dependent: :destroy
  has_many :household_invitations, dependent: :destroy
  has_many :security_audit_events, dependent: :destroy

  before_validation :assign_timezone, :assign_slug

  validates :name, :slug, :timezone, presence: true
  validates :slug, uniqueness: true
  validate :must_have_active_owner, if: :active_owner_validation_required?

  class << self
    def create_with_owner!(name:, owner_account:, owner_person_attributes:, timezone: Time.zone.name)
      transaction do
        household = create_owner_household(name, owner_account, timezone)
        person = create_owner_person(household, owner_account, owner_person_attributes)
        membership = create_owner_membership(household, owner_account, person)
        create_owner_grant(household, membership, person)
        household
      end
    end

    private

    def create_owner_household(name, owner_account, timezone)
      create!(
        name: name,
        timezone: timezone,
        created_by_account: owner_account
      )
    end

    def create_owner_person(household, owner_account, owner_person_attributes)
      household.people.create!(owner_person_attributes.merge(account: owner_account))
    end

    def create_owner_membership(household, owner_account, person)
      household.household_memberships.create!(
        account: owner_account,
        person: person,
        role: :owner,
        status: :active,
        joined_at: Time.current
      )
    end

    def create_owner_grant(household, membership, person)
      household.person_access_grants.create!(
        household_membership: membership,
        person: person,
        access_level: :manage,
        relationship_type: :self,
        granted_by_membership: membership
      )
    end
  end

  private

  def assign_timezone
    self.timezone ||= Time.zone.name
  end

  def assign_slug
    return if slug.present?
    return if name.blank?

    self.slug = "#{name.parameterize}-#{SecureRandom.hex(4)}"
  end

  def active_owner_validation_required?
    persisted? && household_memberships.exists?
  end

  def must_have_active_owner
    return if household_memberships.owner.active.exists?

    errors.add(:base, 'Household must have at least one active owner')
  end
end
