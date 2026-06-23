# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HouseholdMembership do
  let(:household) { create_household_with_owner }
  let(:other_household) do
    create_household_with_owner(
      email: 'other-owner@example.test',
      name: 'Other Family'
    )
  end
  let(:account) { Account.create!(email: 'member@example.test', status: :verified) }
  let(:other_person) do
    other_household.people.create!(
      name: 'Other Person',
      date_of_birth: 40.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def create_household_with_owner(email: 'owner-membership@example.test', name: 'Membership Family')
    account = Account.create!(email: email, status: :verified)
    Household.create_with_owner!(
      name: name,
      owner_account: account,
      owner_person_attributes: {
        name: 'Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
  end

  it 'prevents demoting the last active owner' do
    membership = create_household_with_owner.household_memberships.sole

    expect(membership.update(role: :member)).to be(false)
    expect(membership.errors[:base]).to include('Last active owner cannot be removed')
  end

  it 'prevents revoking the last active owner' do
    membership = create_household_with_owner(
      email: 'owner-revoke@example.test',
      name: 'Revoke Family'
    ).household_memberships.sole

    expect(membership.update(status: :revoked)).to be(false)
    expect(membership.errors[:base]).to include('Last active owner cannot be removed')
  end

  it 'requires a linked person to belong to the same household' do
    membership = described_class.new(
      household: household,
      account: account,
      person: other_person,
      role: :member,
      status: :active
    )

    expect(membership).not_to be_valid
    expect(membership.errors[:person]).to include('must belong to the same household')
  end
end
