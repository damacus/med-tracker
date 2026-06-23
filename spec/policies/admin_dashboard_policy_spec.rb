# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AdminDashboardPolicy, type: :policy do
  fixtures :all

  it 'permits index? for active household managers' do
    household, account, owner = household_with_owner(email: 'dashboard-owner@example.test',
                                                     name: 'Dashboard Owner Family')
    administrator = create_membership(household, email: 'dashboard-admin@example.test', role: :administrator)
    member = create_membership(household, email: 'dashboard-member@example.test', role: :member)

    expect(
      owner: index_permitted?(account: account, household: household, membership: owner),
      administrator: index_permitted?(account: administrator.account, household: household, membership: administrator),
      member: index_permitted?(account: member.account, household: household, membership: member)
    ).to eq(owner: true, administrator: true, member: false)
  end

  it 'does not permit index? from legacy user role alone' do
    expect(described_class.new(users(:admin), :dashboard).index?).to be(false)
    expect(described_class.new(nil, :dashboard).index?).to be(false)
  end

  describe AdminDashboardPolicy::Scope do
    it 'returns the given scope unchanged' do
      scope = User.all
      expect(described_class.new(users(:admin), scope).resolve).to eq(scope)
    end
  end

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

  def context_for(account:, household:, membership:)
    AuthorizationContext.new(account: account, household: household, membership: membership)
  end

  def create_membership(household, email:, role:)
    household.household_memberships.create!(
      account: Account.create!(email: email, status: :verified),
      role: role,
      status: :active
    )
  end

  def index_permitted?(account:, household:, membership:)
    described_class.new(context_for(account: account, household: household, membership: membership), :dashboard).index?
  end
end
