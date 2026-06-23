# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AdminPeoplePolicy do
  fixtures :accounts, :people, :users

  subject(:policy) { described_class.new(user, :admin_people) }

  context 'with a household owner context' do
    let(:household) do
      Household.create_with_owner!(
        name: 'Admin People Family',
        owner_account: accounts(:admin),
        owner_person_attributes: {
          name: 'Admin People Owner',
          date_of_birth: 30.years.ago.to_date,
          person_type: :adult,
          has_capacity: true
        }
      )
    end
    let(:membership) { household.household_memberships.sole }
    let(:user) do
      AuthorizationContext.new(account: accounts(:admin), household: household, membership: membership)
    end

    it { expect(policy.index?).to be(true) }
  end

  context 'with a household member context' do
    let(:household) { Household.create!(name: 'Admin People Member Family', slug: 'admin-people-member-family') }
    let(:membership) do
      household.household_memberships.create!(account: accounts(:jane_doe), role: :member, status: :active)
    end
    let(:user) do
      AuthorizationContext.new(account: accounts(:jane_doe), household: household, membership: membership)
    end

    it { expect(policy.index?).to be(false) }
  end

  context 'with a non-administrator' do
    let(:user) { users(:jane) }

    it { expect(policy.index?).to be(false) }
  end

  context 'with no user' do
    let(:user) { nil }

    it { expect(policy.index?).to be(false) }
  end
end
