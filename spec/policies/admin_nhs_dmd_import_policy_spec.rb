# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AdminNhsDmdImportPolicy, type: :policy do
  fixtures :all

  it 'permits new? for active household managers' do
    household, account, owner = household_with_owner(email: 'dmd-owner@example.test', name: 'DMD Owner Family')
    administrator = create_membership(household, email: 'dmd-admin@example.test', role: :administrator)
    member = create_membership(household, email: 'dmd-member@example.test', role: :member)

    expect(
      owner: new_permitted?(account: account, household: household, membership: owner),
      administrator: new_permitted?(account: administrator.account, household: household, membership: administrator),
      member: new_permitted?(account: member.account, household: household, membership: member)
    ).to eq(owner: true, administrator: true, member: false)
  end

  it 'permits create? for active household managers' do
    household, account, owner = household_with_owner(email: 'dmd-create-owner@example.test',
                                                     name: 'DMD Create Family')
    member = household.household_memberships.create!(
      account: Account.create!(email: 'dmd-create-member@example.test', status: :verified),
      role: :member,
      status: :active
    )

    expect(described_class.new(context_for(account: account, household: household, membership: owner), :import).create?)
      .to be(true)
    expect(described_class.new(context_for(account: member.account, household: household, membership: member),
                               :import).create?).to be(false)
  end

  it 'does not permit actions from legacy user role alone' do
    expect(described_class.new(users(:admin), :import).new?).to be(false)
    expect(described_class.new(users(:admin), :import).create?).to be(false)
    expect(described_class.new(nil, :import).create?).to be(false)
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

  def new_permitted?(account:, household:, membership:)
    described_class.new(context_for(account: account, household: household, membership: membership), :import).new?
  end
end
