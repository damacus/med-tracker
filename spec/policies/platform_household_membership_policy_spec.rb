# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PlatformHouseholdMembershipPolicy do
  fixtures :accounts

  let(:household) { Household.create!(name: 'Policy Household', slug: "policy-household-#{SecureRandom.hex(4)}") }
  let(:membership) do
    account = accounts(:jane_doe)
    person = household.people.create!(
      account: account,
      name: 'Policy Person',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
    household.household_memberships.create!(account: account, person: person, role: :member, status: :active)
  end

  it 'allows only active platform administrators to promote owners' do
    platform_account = accounts(:admin)
    PlatformAdmin.create!(account: platform_account)
    platform_context = AuthorizationContext.new(account: platform_account, household: nil, membership: nil)
    household_context = AuthorizationContext.new(account: accounts(:damacus), household: nil, membership: nil)

    expect(described_class.new(platform_context, membership)).to be_promote_owner
    expect(described_class.new(household_context, membership)).not_to be_promote_owner
  end
end
