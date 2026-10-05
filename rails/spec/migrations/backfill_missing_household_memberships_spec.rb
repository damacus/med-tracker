# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'BackfillMissingHouseholdMemberships' do
  before do
    unless defined?(BackfillMissingHouseholdMemberships)
      load Rails.root.join('db/migrate/20260630183000_backfill_missing_household_memberships.rb')
    end
  end

  it 'creates memberships and access grants for account-linked household people' do
    household = Household.create!(name: 'Membership Backfill', slug: 'membership-backfill')
    account = Account.create!(email: 'membership-backfill@example.test', status: :verified)
    person = household.people.create!(
      account: account,
      name: 'Membership Backfill Person',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )

    BackfillMissingHouseholdMemberships.new.up

    membership = household.household_memberships.find_by!(account: account)
    grant = household.person_access_grants.find_by!(household_membership: membership, person: person)

    expect(membership).to have_attributes(person: person, role: 'owner', status: 'active')
    expect(grant).to have_attributes(access_level: 'manage', relationship_type: 'self')
  end

  it 'promotes an active platform admin when a household has no owner' do
    household = Household.create!(name: 'Owner Backfill', slug: 'owner-backfill')
    member_account = create_account('owner-backfill-member@example.test')
    admin_account = create_account('owner-backfill-admin@example.test')
    create_person(household, member_account, 'Owner Backfill Member')
    admin_person = create_person(household, admin_account, 'Owner Backfill Admin')
    PlatformAdmin.create!(account: admin_account, status: :active)

    BackfillMissingHouseholdMemberships.new.up

    expect(household.household_memberships.find_by!(account: admin_account)).to have_attributes(
      person: admin_person,
      role: 'owner',
      status: 'active'
    )
    expect(household.household_memberships.find_by!(account: member_account).role).to eq('member')
  end

  it 'creates access grants from active carer relationships' do
    household = Household.create!(name: 'Relationship Backfill', slug: 'relationship-backfill')
    carer_account = create_account('relationship-backfill-carer@example.test')
    carer = create_person(household, carer_account, 'Relationship Backfill Carer')
    child = household.people.create!(
      name: 'Relationship Backfill Child',
      date_of_birth: 20.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
    CarerRelationship.create!(carer: carer, patient: child, relationship_type: 'parent', active: true)

    BackfillMissingHouseholdMemberships.new.up

    membership = household.household_memberships.find_by!(account: carer_account)
    grant = household.person_access_grants.find_by!(household_membership: membership, person: child)

    expect(grant).to have_attributes(access_level: 'manage', relationship_type: 'parent')
    expect(grant.granted_by_membership).to eq(household.household_memberships.owner.active.sole)
  end

  it 'uses one active owner grantor when backfilling relationships for households with multiple owners' do
    household = Household.create!(name: 'Multiple Owner Backfill', slug: 'multiple-owner-backfill')
    owner_one_membership = create_owner_membership(household, 'multiple-owner-one@example.test', 'Multiple Owner One')
    owner_two_membership = create_owner_membership(household, 'multiple-owner-two@example.test', 'Multiple Owner Two')
    carer_account = create_account('multiple-owner-carer@example.test')
    carer = create_person(household, carer_account, 'Multiple Owner Carer')
    child = create_adult_person(household, 'Multiple Owner Child')
    CarerRelationship.create!(carer: carer, patient: child, relationship_type: 'parent', active: true)

    BackfillMissingHouseholdMemberships.new.up

    membership = household.household_memberships.find_by!(account: carer_account)
    grant = household.person_access_grants.find_by!(household_membership: membership, person: child)
    owner_ids = [owner_one_membership.id, owner_two_membership.id]

    expect(grant).to have_attributes(access_level: 'manage', relationship_type: 'parent')
    expect(owner_ids).to include(grant.granted_by_membership_id)
  end

  def create_owner_membership(household, email, name)
    account = create_account(email)
    person = create_person(household, account, name)
    household.household_memberships.create!(account: account, person: person, role: :owner, status: :active)
  end

  def create_account(email)
    Account.create!(email: email, status: :verified)
  end

  def create_person(household, account, name)
    create_adult_person(household, name).tap { |person| person.update!(account: account) }
  end

  def create_adult_person(household, name)
    household.people.create!(
      name: name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end
end
