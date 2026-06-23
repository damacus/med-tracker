# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Household do
  describe '.create_with_owner!' do
    subject(:household) do
      described_class.create_with_owner!(
        name: 'Alex Family',
        owner_account: account,
        owner_person_attributes: {
          name: 'Alex',
          date_of_birth: 30.years.ago.to_date,
          person_type: :adult,
          has_capacity: true
        }
      )
    end

    let(:account) { Account.create!(email: 'owner@example.test', status: :verified) }

    it 'creates the household with default tenant settings and one Home location' do
      expect(household).to be_persisted
      expect(household).to be_active
      expect(household.slug).to be_present
      expect(household.timezone).to eq(Time.zone.name)
      expect(household.locations.pluck(:name)).to contain_exactly('Home')
    end

    it 'creates the owner membership and linked person' do
      membership = household.household_memberships.sole

      expect(membership).to have_attributes(account: account, role: 'owner', status: 'active')
      expect(membership.person).to have_attributes(account: account, household: household, name: 'Alex')
    end

    it 'creates the owner self grant' do
      membership = household.household_memberships.sole

      grant = household.person_access_grants.sole
      expect(grant).to have_attributes(
        household_membership: membership,
        person: membership.person,
        access_level: 'manage',
        relationship_type: 'self'
      )
    end
  end

  describe 'validations' do
    let(:account) { Account.create!(email: 'member-validation@example.test', status: :verified) }
    let(:household) { described_class.create!(name: 'Ownerless Family') }
    let(:person) do
      household.people.create!(
        name: 'Member',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      )
    end

    it 'requires at least one active owner before archiving is allowed' do
      household.household_memberships.create!(
        account: account,
        person: person,
        role: :member,
        status: :active
      )

      expect(household.reload).not_to be_valid
      expect(household.errors[:base]).to include('Household must have at least one active owner')
    end
  end
end
