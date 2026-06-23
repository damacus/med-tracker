# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonAccessGrant do
  def household_bundle
    account = Account.create!(email: "grant-#{SecureRandom.hex(4)}@example.test", status: :verified)
    household = Household.create_with_owner!(
      name: "Grant #{SecureRandom.hex(4)}",
      owner_account: account,
      owner_person_attributes: {
        name: 'Grant Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
    [household, household.household_memberships.sole, household.people.sole]
  end

  def grant_target_person(household, name:)
    household.people.create!(
      name: name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def create_grant_for(household, membership, name, attributes = {})
    person = grant_target_person(household, name: name)
    described_class.create!(
      {
        household: household,
        household_membership: membership,
        person: person,
        access_level: :view,
        relationship_type: :self,
        granted_by_membership: membership
      }.merge(attributes)
    )
  end

  def build_grant(household:, membership:, person:, grantor: membership)
    described_class.new(
      household: household,
      household_membership: membership,
      person: person,
      access_level: :view,
      relationship_type: :family_member,
      granted_by_membership: grantor
    )
  end

  it 'treats relationship type as descriptive and access level as the authority' do
    household, membership = household_bundle
    person = grant_target_person(household, name: 'Record Target')
    grant = described_class.create!(
      household: household,
      household_membership: membership,
      person: person,
      access_level: :record,
      relationship_type: :professional,
      granted_by_membership: membership
    )

    expect(grant.cover_access?(:view)).to be(true)
    expect(grant.cover_access?(:record)).to be(true)
    expect(grant.cover_access?(:manage)).to be(false)
  end

  it 'excludes revoked and expired grants from active authorization' do
    household, membership = household_bundle
    active = create_grant_for(household, membership, 'Active Target')
    revoked = create_grant_for(household, membership, 'Revoked Target', revoked_at: Time.current)
    expired = create_grant_for(household, membership, 'Expired Target', expires_at: 1.minute.ago)

    expect(described_class.active.where(id: [active.id, revoked.id, expired.id])).to contain_exactly(active)
  end

  it 'requires all linked records to belong to the same household' do
    household, membership = household_bundle
    other_household, = household_bundle
    other_person = other_household.people.sole

    grant = build_grant(household: household, membership: membership, person: other_person)

    expect(grant).not_to be_valid
    expect(grant.errors[:person]).to include('must belong to the same household')
  end

  it 'requires the membership and grantor membership to belong to the household' do
    household, membership = household_bundle
    _, other_membership = household_bundle
    person = grant_target_person(household, name: 'Cross Household Target')

    grant = build_grant(household: household, membership: other_membership, person: person)

    expect(grant).not_to be_valid
    expect(grant.errors[:household_membership]).to include('must belong to the same household')
    expect(grant.errors[:granted_by_membership]).to include('must belong to the same household')

    valid_grant = build_grant(household: household, membership: membership, person: person, grantor: nil)
    expect(valid_grant).to be_valid
  end
end
