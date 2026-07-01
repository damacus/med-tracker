# frozen_string_literal: true

require 'rails_helper'

RSpec.describe FixtureHouseholdSetup do
  self.use_transactional_tests = false

  describe '.apply!' do
    before do
      PersonAccessGrant.delete_all
      HouseholdMembership.delete_all
      SpecFixtureLoader.load(:accounts, :people, :users)
    end

    after do
      PersonAccessGrant.delete_all
      HouseholdMembership.delete_all
    end

    it 'creates tenant memberships and self grants for account-linked fixture people', :aggregate_failures do
      described_class.apply!

      user = User.find_by!(email_address: 'jane.doe@example.com')
      household = Household.find_by!(slug: 'fixture-household')
      membership = user.person.account.first_active_household_membership
      grant = household.person_access_grants.find_by(household_membership: membership, person: user.person)

      expect(membership).to have_attributes(household: household, person: user.person, role: 'member', status: 'active')
      expect(grant).to have_attributes(access_level: 'manage', relationship_type: 'self')
    end
  end
end
