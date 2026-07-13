# frozen_string_literal: true

require 'rails_helper'

RSpec.describe FixtureHouseholdSetup do
  self.use_transactional_tests = false

  describe '.apply!' do
    before do
      PersonAccessGrant.delete_all
      HouseholdMembership.delete_all
      SpecFixtureLoader.load(:accounts, :people, :users, :carer_relationships)
    end

    after { described_class.apply! }

    it 'creates tenant memberships and self grants for account-linked fixture people', :aggregate_failures do
      described_class.apply!

      user = User.find_by!(email_address: 'jane.doe@example.com')
      household = Household.find_by!(slug: 'fixture-household')
      membership = user.person.account.first_active_household_membership
      grant = household.person_access_grants.find_by(household_membership: membership, person: user.person)

      expect(membership).to have_attributes(household: household, person: user.person, role: 'member', status: 'active')
      expect(grant).to have_attributes(access_level: 'manage', relationship_type: 'self')
    end

    it 'links relationship-derived grants to their delegation source' do
      described_class.apply!

      relationship = CarerRelationship.find_by!(relationship_type: :parent, active: true)
      membership = relationship.household.household_memberships.find_by!(person: relationship.carer)
      grant = relationship.household.person_access_grants.find_by!(
        household_membership: membership,
        person: relationship.patient
      )

      expect(grant.carer_relationship).to eq(relationship)
    end

    it 'does not relabel an existing manual grant as relationship-owned' do
      described_class.apply!
      relationship = CarerRelationship.find_by!(relationship_type: :parent, active: true)
      membership = relationship.household.household_memberships.find_by!(person: relationship.carer)
      grant = relationship.household.person_access_grants.find_by!(
        household_membership: membership,
        person: relationship.patient
      )
      grant.update!(carer_relationship: nil)

      described_class.apply!

      expect(grant.reload.carer_relationship).to be_nil
    end

    it 'keeps self grants independent from legacy self relationships' do
      described_class.apply!
      relationship = CarerRelationship.find_by!(relationship_type: '0', active: true)
      membership = relationship.household.household_memberships.find_by!(person: relationship.carer)
      grant = relationship.household.person_access_grants.find_by!(
        household_membership: membership,
        person: relationship.patient
      )

      expect(grant.carer_relationship).to be_nil
    end
  end
end
