# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260713150000_add_carer_relationship_source_to_person_access_grants')

RSpec.describe AddCarerRelationshipSourceToPersonAccessGrants do
  self.use_transactional_tests = false

  let(:household) { Household.create!(name: "Provenance Migration Test #{SecureRandom.hex(4)}") }
  let(:carer) { create_account_person('backfill-carer') }
  let(:patient) { create(:person, household: household) }
  let(:membership) do
    household.household_memberships.create!(
      account: carer.account,
      person: carer,
      role: :member,
      status: :active
    )
  end
  let(:relationship) do
    CarerRelationship.create!(
      household: household,
      carer: carer,
      patient: patient,
      relationship_type: :parent,
      active: true
    )
  end

  before { ensure_migration_applied }

  after do
    ensure_migration_applied
    cleanup_households
  end

  it 'leaves an exact legacy grant unowned because provenance cannot be inferred safely' do
    migrate_down
    grant = create_grant(access_level: :manage, relationship_type: :parent)

    migrate_up

    expect(grant.reload.carer_relationship_id).to be_nil
  end

  it 'does not classify inactive, revoked, or expired legacy grants' do
    migrate_down
    inactive_grant = create_legacy_grant(relationship_active: false)
    revoked_grant = create_legacy_grant(revoked_at: 1.day.ago)
    expired_grant = create_legacy_grant(expires_at: 1.day.ago)

    migrate_up

    expect([inactive_grant, revoked_grant, expired_grant].map { it.reload.carer_relationship_id }).to all(be_nil)
  end

  it 'supports structural rollback and reapplication' do
    migrate_down
    expect(PersonAccessGrant.column_names).not_to include('carer_relationship_id')

    migrate_up
    expect(PersonAccessGrant.column_names).to include('carer_relationship_id')
  end

  def create_account_person(prefix)
    account = Account.create!(
      email: "#{prefix}-#{SecureRandom.hex(4)}@example.com",
      password_hash: BCrypt::Password.create('password'),
      status: :verified
    )
    create(:person, household: household, account: account)
  end

  def create_grant(access_level:, relationship_type:)
    household.person_access_grants.create!(
      household_membership: membership,
      person: patient,
      access_level: access_level,
      relationship_type: relationship_type,
      granted_by_membership: membership
    ).tap { relationship }
  end

  def create_legacy_grant(relationship_active: true, revoked_at: nil, expires_at: nil)
    suffix = SecureRandom.hex(4)
    legacy_carer = create_account_person("legacy-#{suffix}")
    legacy_patient = create(:person, household: household)
    legacy_membership = create_legacy_membership(legacy_carer)
    create_legacy_relationship(legacy_carer, legacy_patient, relationship_active)
    household.person_access_grants.create!(
      household_membership: legacy_membership,
      person: legacy_patient,
      access_level: :manage,
      relationship_type: :parent,
      granted_by_membership: legacy_membership,
      revoked_at: revoked_at,
      expires_at: expires_at
    )
  end

  def create_legacy_membership(legacy_carer)
    household.household_memberships.create!(
      account: legacy_carer.account,
      person: legacy_carer,
      role: :member,
      status: :active
    )
  end

  def create_legacy_relationship(legacy_carer, legacy_patient, relationship_active)
    CarerRelationship.create!(
      household: household,
      carer: legacy_carer,
      patient: legacy_patient,
      relationship_type: :parent,
      active: relationship_active
    )
  end

  def migrate_down
    described_class.new.migrate(:down)
    PersonAccessGrant.reset_column_information
  end

  def migrate_up
    described_class.new.migrate(:up)
    PersonAccessGrant.reset_column_information
  end

  def ensure_migration_applied
    return if PersonAccessGrant.connection.column_exists?(:person_access_grants, :carer_relationship_id)

    migrate_up
  end

  def cleanup_households
    Household.where("name LIKE 'Provenance Migration Test %'").find_each do |target_household|
      cleanup_household(target_household)
    end
  end

  def cleanup_household(target_household)
    account_ids = target_household.household_memberships.pluck(:account_id) +
                  target_household.people.pluck(:account_id).compact
    [PersonAccessGrant, CarerRelationship, HouseholdMembership, LocationMembership, Location, Person].each do |model|
      model.where(household: target_household).delete_all
    end
    target_household.delete
    Account.where(id: account_ids).delete_all
  end
end
