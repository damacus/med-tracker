# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonPolicy, type: :policy do
  def household_with_owner(email:, name:)
    account = Account.create!(email: email, status: :verified)
    household = Household.create_with_owner!(
      name: name,
      owner_account: account,
      owner_person_attributes: {
        name: "#{name} Owner",
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
    [household, account, household.household_memberships.sole]
  end

  def adult_person(household, name:)
    household.people.create!(
      name: name,
      date_of_birth: 32.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def context_for(account:, household:, membership:)
    AuthorizationContext.new(account: account, household: household, membership: membership)
  end

  def grant_access(household:, membership:, person:, access_level:)
    household.person_access_grants.create!(
      household_membership: membership,
      person: person,
      access_level: access_level,
      relationship_type: :family_member,
      granted_by_membership: membership
    )
  end

  it 'does not let household owner role alone expose a capable adult' do
    household, account, membership = household_with_owner(email: 'owner-no-grant@example.test', name: 'No Grant Family')
    capable_adult = adult_person(household, name: 'Private Adult')
    context = context_for(account: account, household: household, membership: membership)

    policy = described_class.new(context, capable_adult)

    expect(policy.show?).to be(false)
    expect(policy.update?).to be(false)
  end

  it 'uses explicit record grants for view-only person access' do
    household, account, membership = household_with_owner(email: 'owner-grant@example.test',
                                                          name: 'Grant Policy Family')
    person = adult_person(household, name: 'Grant Adult')
    context = context_for(account: account, household: household, membership: membership)

    grant_access(household: household, membership: membership, person: person, access_level: :record)

    policy = described_class.new(context, person)

    expect(policy.show?).to be(true)
    expect(policy.update?).to be(false)
    expect(policy.add_medication?).to be(false)
  end

  it 'uses explicit manage grants for writable person access' do
    household, account, membership = household_with_owner(email: 'owner-manage@example.test',
                                                          name: 'Manage Policy Family')
    person = adult_person(household, name: 'Manage Adult')
    context = context_for(account: account, household: household, membership: membership)

    grant_access(household: household, membership: membership, person: person, access_level: :manage)

    policy = described_class.new(context, person)
    expect(policy.update?).to be(true)
    expect(policy.add_medication?).to be(true)
  end

  it 'denies expired and revoked grants immediately' do
    household, account, membership = household_with_owner(email: 'expired-grant@example.test',
                                                          name: 'Expired Grant Family')
    person = adult_person(household, name: 'Expired Adult')
    context = context_for(account: account, household: household, membership: membership)
    grant = household.person_access_grants.create!(
      household_membership: membership,
      person: person,
      access_level: :manage,
      relationship_type: :family_member,
      granted_by_membership: membership,
      expires_at: 1.minute.ago
    )

    expect(described_class.new(context, person).show?).to be(false)

    grant.update!(expires_at: nil, revoked_at: Time.current)

    expect(described_class.new(context, person).show?).to be(false)
  end

  it 'scopes people to the active household and active grants' do
    household, account, membership = household_with_owner(email: 'scope-owner@example.test', name: 'Scope Family')
    other_household, = household_with_owner(email: 'other-scope-owner@example.test', name: 'Other Scope Family')
    granted = adult_person(household, name: 'Visible Adult')
    ungranted = adult_person(household, name: 'Hidden Adult')
    other_person = adult_person(other_household, name: 'Foreign Adult')
    context = context_for(account: account, household: household, membership: membership)
    grant_access(household: household, membership: membership, person: granted, access_level: :view)

    resolved = described_class::Scope.new(context, Person.all).resolve

    expect(resolved).to include(granted, membership.person)
    expect(resolved).not_to include(ungranted, other_person)
  end

  it 'allows active household managers to create people in their household' do
    household, account, membership = household_with_owner(email: 'create-person-owner@example.test',
                                                          name: 'Create Family')
    context = context_for(account: account, household: household, membership: membership)
    person = household.people.build(name: 'New Dependent', person_type: :minor)

    expect(described_class.new(context, person).create?).to be(true)
  end
end
